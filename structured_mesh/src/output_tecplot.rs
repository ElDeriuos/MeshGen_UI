//! Tecplot ASCII output writers — `ch3` (FEPOINT mesh) and `ch4` (structured zones).
//!
//! ## File formats
//!
//! ### `ch3` — FEPOINT quadrilateral mesh
//!
//! Compatible with Tecplot 360 and any tool that reads Tecplot ASCII FEPOINT format.
//!
//! ```text
//! VARIABLES=X  Y
//! ZONE T="main" N=<nnode> E=<nelem> F=FEPOINT ET=QUADRILATERAL
//! x  y          ← nnode lines, format '(2f11.4)'
//! n1 n2 n4 n3   ← nelem lines, 4 × width-9 integers (n4/n3 Fortran swap)
//! ```
//!
//! ### `ch4` — structured zones
//!
//! Written in a single sequential pass (the Fortran original used two passes
//! with a rewind — Rust writes everything in order the first time).
//!
//! ```text
//! VARIABLES=X  Y
//! ZONE T=" Main " I=<nx+1> J=<ny+1> F=POINT     ← full corner-node grid
//! x  y          ← (nx+1)*(ny+1) lines, format '(2f11.4)'
//!
//! ZONE T=" NRO   1" I=<ix> J=  2 F=POINT        ← one per RowRecord
//! x  y          ← ix*2 lines, format '(2f11.4)'
//! ...
//! ZONE T=" NCO   1" I=  2 J=<iy> F=POINT        ← one per ColRecord
//! x  y          ← 2*iy lines, format '(2f11.4)'
//! ...
//! ```
//!
//! ## Format constants
//!
//! | Purpose | Fortran label | Rust format string |
//! |---|---|---|
//! | Full-grid zone header | 41 | `"ZONE T=\" Main \" I={:3} J={:3} F=POINT"` |
//! | Row-span zone header | 21 | `"ZONE T=\" NRO {:3}\" I={:3} J={:3} F=POINT"` |
//! | Col-span zone header | 31 | `"ZONE T=\" NCO {:3}\" I={:3} J={:3} F=POINT"` |
//! | FEPOINT zone header | 11 | `"ZONE T=\"main\" N={:6} E={:6} F=FEPOINT ET=QUADRILATERAL"` |
//! | Coordinates (ch3) | 12 | `"{:11.4}{:11.4}"` |
//! | Coordinates (ch4) | 22 | `"{:11.4}{:11.4}"` |

use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::MeshError;
use crate::output_text::fmt_2f11_1;
use crate::scanline_rasterizer::{MeshState, PlotData};

// ---------------------------------------------------------------------------
// Local format helpers
// ---------------------------------------------------------------------------

/// Fortran `'(2f11.4)'` — two floats in width-11, 4 decimal places (22 chars).
#[inline]
pub fn fmt_2f11_4(x: f64, y: f64) -> String {
    format!("{:11.4}{:11.4}", x, y)
}

// ---------------------------------------------------------------------------
// Task 9.1 — write_ch3 (FEPOINT quadrilateral zone)
// ---------------------------------------------------------------------------

/// Write the `ch3` Tecplot FEPOINT file (Fortran unit 11, first open).
///
/// Structure:
/// ```text
/// VARIABLES=X  Y
/// ZONE T="main" N=<nnode> E=<nelem> F=FEPOINT ET=QUADRILATERAL
/// <nnode lines of 2f11.1 coordinates>
/// <nelem lines of first-4 columns of 6i9 connectivity>
/// ```
pub fn write_ch3(plot: &PlotData, path: &Path) -> Result<(), MeshError> {
    let file = std::fs::File::create(path).map_err(MeshError::Io)?;
    let mut w = BufWriter::new(file);

    let nnode = plot.nodes.len();
    let nelem = plot.elements.len();

    // Fortran format 10: VARIABLES=X  Y  (two spaces between X and Y)
    writeln!(w, "VARIABLES=X  Y").map_err(MeshError::Io)?;

    // Fortran format 11: ZONE T="main" N=<i6> E=<i6> F=FEPOINT ET=QUADRILATERAL
    writeln!(
        w,
        "ZONE T=\"main\" N={:6} E={:6} F=FEPOINT ET=QUADRILATERAL",
        nnode, nelem
    )
    .map_err(MeshError::Io)?;

    // Node coordinates — format 12: '(2f11.4)'
    for &(x, y) in &plot.nodes {
        writeln!(w, "{}", fmt_2f11_1(x, y)).map_err(MeshError::Io)?;
    }

    // Element connectivity — first 4 columns of format 13: '(6i9)'
    // elements stored as [n1, n2, n4, n3] — write directly.
    for elem in &plot.elements {
        writeln!(
            w,
            "{:9}{:9}{:9}{:9}",
            elem[0], elem[1], elem[2], elem[3]
        )
        .map_err(MeshError::Io)?;
    }

    w.flush().map_err(MeshError::Io)
}

// ---------------------------------------------------------------------------
// Task 9.2 — write_ch4 (full-grid zone + row/col span zones)
// ---------------------------------------------------------------------------

/// Write the `ch4` Tecplot structured-zone file (Fortran unit 11, second open).
///
/// Fortran writes this file in two passes (rewind + re-open); Rust writes
/// it in a single sequential pass in the same order the data would appear.
///
/// Structure:
/// ```text
/// VARIABLES=X  Y
/// ZONE T=" Main " I=<nx+1> J=<ny+1> F=POINT      ← format 41
/// <(nx+1)*(ny+1) lines of 2f11.4 coordinates>
///
/// For each RowRecord:
///   ZONE T=" NRO <i>" I=<ix> J=  2 F=POINT        ← format 21
///   <ix*2 lines of 2f11.4>
///
/// For each ColRecord:
///   ZONE T=" NCO <i>" I=  2 J=<iy> F=POINT        ← format 31
///   <2*iy lines of 2f11.4>
/// ```
pub fn write_ch4(state: &MeshState, plot: &PlotData, path: &Path) -> Result<(), MeshError> {
    let _ = plot; // plot not needed for ch4 coordinate generation

    let nx = state.nx;
    let ny = state.ny;
    let dx = state.geo.dx;
    let dy = state.geo.dy;
    let xmin = state.geo.xmin;
    let ymin = state.geo.ymin;

    let file = std::fs::File::create(path).map_err(MeshError::Io)?;
    let mut w = BufWriter::new(file);

    // Fortran format 10
    writeln!(w, "VARIABLES=X  Y").map_err(MeshError::Io)?;

    // Fortran format 41: ZONE T=" Main " I=<nx+1:3> J=<ny+1:3> F=POINT
    writeln!(
        w,
        "ZONE T=\" Main \" I={:3} J={:3} F=POINT",
        nx + 1,
        ny + 1
    )
    .map_err(MeshError::Io)?;

    // Full-grid corner coordinates — format 22: '(2f11.4)'
    // Fortran: do j=1,ny+1  do i=1,nx+1  x=xmin+(i-1.5)*dx  y=ymin+(j-1.5)*dy
    for j in 1..=(ny + 1) {
        let y = ymin + (j as f64 - 1.5) * dy;
        for i in 1..=(nx + 1) {
            let x = xmin + (i as f64 - 1.5) * dx;
            writeln!(w, "{}", fmt_2f11_4(x, y)).map_err(MeshError::Io)?;
        }
    }

    // Row-span zones — one per RowRecord.
    // Fortran: ix = i2 - i1 + 2,  iy = 2
    for (zone_idx, rr) in state.row_records.iter().enumerate() {
        let ix = rr.i2 - rr.i1 + 2;
        let iy = 2usize;
        // Fortran format 21: ZONE T=" NRO <i:3>" I=<ix:3> J=<iy:3> F=POINT
        writeln!(
            w,
            "ZONE T=\" NRO {:3}\" I={:3} J={:3} F=POINT",
            zone_idx + 1,
            ix,
            iy
        )
        .map_err(MeshError::Io)?;

        // Fortran: do k=1,2  y=ymin+(j0+k-2.5)*dy  do j=i1,i2+1  x=xmin+(j-1.5)*dx
        for k in 1..=2usize {
            let y = ymin + (rr.j0 as f64 + k as f64 - 2.5) * dy;
            for j in rr.i1..=(rr.i2 + 1) {
                let x = xmin + (j as f64 - 1.5) * dx;
                writeln!(w, "{}", fmt_2f11_4(x, y)).map_err(MeshError::Io)?;
            }
        }
    }

    // Column-span zones — one per ColRecord.
    // Fortran: ix = 2,  iy = j2 - j1 + 2
    for (zone_idx, cr) in state.col_records.iter().enumerate() {
        let ix = 2usize;
        let iy = cr.j2 - cr.j1 + 2;
        // Fortran format 31: ZONE T=" NCO <i:3>" I=<ix:3> J=<iy:3> F=POINT
        writeln!(
            w,
            "ZONE T=\" NCO {:3}\" I={:3} J={:3} F=POINT",
            zone_idx + 1,
            ix,
            iy
        )
        .map_err(MeshError::Io)?;

        // Fortran: do k=1,2  x=xmin+(i0+k-2.5)*dx  do j=j1,j2+1  y=ymin+(j-1.5)*dy
        for k in 1..=2usize {
            let x = xmin + (cr.i0 as f64 + k as f64 - 2.5) * dx;
            for j in cr.j1..=(cr.j2 + 1) {
                let y = ymin + (j as f64 - 1.5) * dy;
                writeln!(w, "{}", fmt_2f11_4(x, y)).map_err(MeshError::Io)?;
            }
        }
    }

    w.flush().map_err(MeshError::Io)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_parser::{Edge, MeshInput};
    use crate::scanline_rasterizer::{build_plot_data, compute_grid, rasterize};
    use tempfile::NamedTempFile;

    fn small_rect() -> (MeshState, PlotData) {
        let input = MeshInput {
            dx: 1.0,
            dy: 1.0,
            nodes: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
            edges: vec![
                Edge { k1: 0, k2: 1, kp: 1 },
                Edge { k1: 1, k2: 2, kp: 1 },
                Edge { k1: 2, k2: 3, kp: 1 },
                Edge { k1: 3, k2: 0, kp: 1 },
            ],
        };
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();
        let plot = build_plot_data(&state);
        (state, plot)
    }

    // ── fmt_2f11_4 ────────────────────────────────────────────────────────

    #[test]
    fn fmt_2f11_4_width() {
        assert_eq!(fmt_2f11_4(1.0, 2.0).len(), 22);
    }

    #[test]
    fn fmt_2f11_4_content() {
        // '(2f11.4)': "     1.0000     2.0000"
        assert_eq!(fmt_2f11_4(1.0, 2.0), "     1.0000     2.0000");
    }

    // ── write_ch3 ─────────────────────────────────────────────────────────

    #[test]
    fn ch3_header_line() {
        let (_, plot) = small_rect();
        let tmp = NamedTempFile::new().unwrap();
        write_ch3(&plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(
            content.starts_with("VARIABLES=X  Y\n"),
            "ch3 must start with VARIABLES=X  Y"
        );
    }

    #[test]
    fn ch3_zone_header_contains_nnode_nelem() {
        let (_, plot) = small_rect();
        let tmp = NamedTempFile::new().unwrap();
        write_ch3(&plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        let nnode = plot.nodes.len();
        let nelem = plot.elements.len();
        assert!(
            content.contains(&format!("N={:6}", nnode)),
            "ch3 zone header must have N={nnode:6}"
        );
        assert!(
            content.contains(&format!("E={:6}", nelem)),
            "ch3 zone header must have E={nelem:6}"
        );
        assert!(content.contains("F=FEPOINT ET=QUADRILATERAL"));
    }

    #[test]
    fn ch3_first_coord_line_is_22_chars() {
        let (_, plot) = small_rect();
        let tmp = NamedTempFile::new().unwrap();
        write_ch3(&plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        // Third line (0-indexed line 2) is first coordinate.
        let line = content.lines().nth(2).expect("must have coord line");
        assert_eq!(line.len(), 22, "coord line must be 22 chars: {line:?}");
    }

    // ── write_ch4 ─────────────────────────────────────────────────────────

    #[test]
    fn ch4_header_line() {
        let (state, plot) = small_rect();
        let tmp = NamedTempFile::new().unwrap();
        write_ch4(&state, &plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.starts_with("VARIABLES=X  Y\n"));
    }

    #[test]
    fn ch4_main_zone_header_field_widths() {
        let (state, plot) = small_rect();
        let tmp = NamedTempFile::new().unwrap();
        write_ch4(&state, &plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        let nx = state.nx;
        let ny = state.ny;
        let expected = format!(
            "ZONE T=\" Main \" I={:3} J={:3} F=POINT",
            nx + 1, ny + 1
        );
        assert!(
            content.contains(&expected),
            "ch4 must contain main zone header: {expected:?}"
        );
    }

    #[test]
    fn ch4_coord_lines_are_22_chars() {
        let (state, plot) = small_rect();
        let tmp = NamedTempFile::new().unwrap();
        write_ch4(&state, &plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        // All non-header lines should be exactly 22 chars (2f11.4).
        for line in content.lines().skip(2) {
            if line.starts_with("ZONE") {
                continue;
            }
            assert_eq!(
                line.len(), 22,
                "ch4 coord line must be 22 chars: {line:?}"
            );
        }
    }

    #[test]
    fn ch4_nro_zones_present() {
        let (state, plot) = small_rect();
        let nro = state.row_records.len();
        let tmp = NamedTempFile::new().unwrap();
        write_ch4(&state, &plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        let nro_count = content.lines().filter(|l| l.contains("NRO")).count();
        assert_eq!(nro_count, nro, "ch4 must have {nro} NRO zones");
    }
}

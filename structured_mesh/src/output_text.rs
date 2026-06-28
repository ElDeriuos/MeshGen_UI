//! Text output writers — `geom3.dat` (Fortran unit 7) and `ch2` (unit 21).
//!
//! ## Output format
//!
//! Both files receive **identical content** formatted to match the original
//! Fortran FORMAT statements byte-for-byte:
//!
//! | Content | Fortran format | Rust helper |
//! |---|---|---|
//! | `dx dy` header | list-directed | `" {} {}\n"` |
//! | `nx ny` header | list-directed | `" {} {}\n"` |
//! | `nro` / `nco` counts | list-directed | `" {}\n"` |
//! | Row/col span records | `'(5i6)'` | [`fmt_5i6`] |
//! | Node coordinates | `'(2f11.1)'` | [`fmt_2f11_1`] |
//! | Element connectivity | `'(6i9)'` | [`fmt_6i9`] |
//!
//! ## File structure
//!
//! ```text
//! <dx> <dy>
//! <nx> <ny>
//! <nro>
//! i1 i2 j0 kn_left kn_right    ← nro lines, format '(5i6)'
//! <nco>
//! j1 j2 i0 kn_bot  kn_top      ← nco lines, format '(5i6)'
//! <nnode> <nelem>
//! x  y                          ← nnode lines, format '(2f11.1)'
//! n1 n2 n4 n3 ik jk             ← nelem lines, format '(6i9)', note n4/n3 swap
//! ```
//!
//! ## Format helpers
//!
//! [`fmt_5i6`], [`fmt_2f11_1`], and [`fmt_6i9`] are `pub` so that
//! `output_tecplot` and tests can reuse them without duplication.

use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::MeshError;
use crate::scanline_rasterizer::{MeshState, PlotData};

// ---------------------------------------------------------------------------
// Format helpers (pub so output_tecplot / tests can reuse them)
// ---------------------------------------------------------------------------

/// Fortran `'(5i6)'` — five integers, each right-justified in width 6.
/// Produces exactly 30 characters (no trailing newline).
#[inline]
pub fn fmt_5i6(a: i64, b: i64, c: i64, d: i64, e: i64) -> String {
    format!("{:6}{:6}{:6}{:6}{:6}", a, b, c, d, e)
}

/// Fortran `'(2f11.1)'` — two floats, each right-justified in width 11 with
/// 1 decimal place.  Produces exactly 22 characters.
#[inline]
pub fn fmt_2f11_1(x: f64, y: f64) -> String {
    format!("{:11.1}{:11.1}", x, y)
}

/// Fortran `'(6i9)'` — six integers, each right-justified in width 9.
/// Produces exactly 54 characters.
#[inline]
pub fn fmt_6i9(a: i64, b: i64, c: i64, d: i64, e: i64, f: i64) -> String {
    format!("{:9}{:9}{:9}{:9}{:9}{:9}", a, b, c, d, e, f)
}

// ---------------------------------------------------------------------------
// Shared writer — both geom3.dat and ch2 get identical content
// ---------------------------------------------------------------------------

fn write_mesh<W: Write>(
    w: &mut W,
    state: &MeshState,
    plot: &PlotData,
) -> Result<(), MeshError> {
    let dx = state.geo.dx;
    let dy = state.geo.dy;
    let nx = state.nx;
    let ny = state.ny;

    // Fortran: write(21,*) dx, dy
    writeln!(w, " {}  {}", dx, dy).map_err(MeshError::Io)?;

    // Fortran: write(21,*) nx, ny
    writeln!(w, " {}  {}", nx, ny).map_err(MeshError::Io)?;

    // Fortran: write(7/21,*) nro
    let nro = state.row_records.len();
    writeln!(w, " {}", nro).map_err(MeshError::Io)?;

    // Fortran: write(7/21,'(5i6)') i1,i2,j0,kn_l,kn_r
    for rr in &state.row_records {
        writeln!(
            w,
            "{}",
            fmt_5i6(rr.i1 as i64, rr.i2 as i64, rr.j0 as i64,
                    rr.kn_left as i64, rr.kn_right as i64)
        )
        .map_err(MeshError::Io)?;
    }

    // Fortran: write(7/21,*) nco
    let nco = state.col_records.len();
    writeln!(w, " {}", nco).map_err(MeshError::Io)?;

    // Fortran: write(7/21,'(5i6)') j1,j2,i0,kn_b,kn_t
    for cr in &state.col_records {
        writeln!(
            w,
            "{}",
            fmt_5i6(cr.j1 as i64, cr.j2 as i64, cr.i0 as i64,
                    cr.kn_bot as i64, cr.kn_top as i64)
        )
        .map_err(MeshError::Io)?;
    }

    // Fortran: write(7/21,*) nnode, nelem
    let nnode = plot.nodes.len();
    let nelem = plot.elements.len();
    writeln!(w, " {}  {}", nnode, nelem).map_err(MeshError::Io)?;

    // Fortran: write(7/21,'(2f11.1)') xp(i), yp(i)
    for &(x, y) in &plot.nodes {
        writeln!(w, "{}", fmt_2f11_1(x, y)).map_err(MeshError::Io)?;
    }

    // Fortran: write(7/21,'(6i9)') n1,n2,n4,n3,ik,jk
    // elements already stored in [n1,n2,n4,n3] order.
    for (e, elem) in plot.elements.iter().enumerate() {
        let (ik, jk) = plot.elem_ij[e];
        writeln!(
            w,
            "{}",
            fmt_6i9(
                elem[0] as i64, elem[1] as i64,
                elem[2] as i64, elem[3] as i64,
                ik as i64, jk as i64,
            )
        )
        .map_err(MeshError::Io)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write `geom3.dat` (Fortran unit 7).
pub fn write_geom3(state: &MeshState, plot: &PlotData, path: &Path) -> Result<(), MeshError> {
    let file = std::fs::File::create(path).map_err(MeshError::Io)?;
    let mut bw = BufWriter::new(file);
    write_mesh(&mut bw, state, plot)?;
    bw.flush().map_err(MeshError::Io)
}

/// Write the `ch2` secondary text output (Fortran unit 21).
/// Identical content to `geom3.dat`.
pub fn write_ch2(state: &MeshState, plot: &PlotData, path: &Path) -> Result<(), MeshError> {
    write_geom3(state, plot, path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── format helpers ────────────────────────────────────────────────────

    #[test]
    fn fmt_5i6_width() {
        let s = fmt_5i6(1, 2, 3, 4, 5);
        assert_eq!(s.len(), 30, "5i6 must be exactly 30 chars: {s:?}");
    }

    #[test]
    fn fmt_5i6_content() {
        // Fortran reference: "     1     2     3     4     5"
        assert_eq!(fmt_5i6(1, 2, 3, 4, 5), "     1     2     3     4     5");
    }

    #[test]
    fn fmt_2f11_1_width() {
        let s = fmt_2f11_1(1.0, 2.0);
        assert_eq!(s.len(), 22, "2f11.1 must be exactly 22 chars: {s:?}");
    }

    #[test]
    fn fmt_2f11_1_content() {
        // Fortran '(2f11.1)': "        1.0        2.0"
        assert_eq!(fmt_2f11_1(1.0, 2.0), "        1.0        2.0");
    }

    #[test]
    fn fmt_6i9_width() {
        let s = fmt_6i9(1, 2, 3, 4, 5, 6);
        assert_eq!(s.len(), 54, "6i9 must be exactly 54 chars: {s:?}");
    }

    #[test]
    fn fmt_6i9_content() {
        assert_eq!(
            fmt_6i9(1, 2, 3, 4, 5, 6),
            "        1        2        3        4        5        6"
        );
    }

    #[test]
    fn fmt_5i6_negative() {
        // Negative values must still fit in width-6 fields.
        let s = fmt_5i6(-1, -22, -333, 4444, 55555);
        assert_eq!(s.len(), 30);
        assert!(s.contains("-1"));
    }

    // ── write_geom3 round-trip ────────────────────────────────────────────

    #[test]
    fn write_geom3_round_trip() {
        use crate::input_parser::{Edge, MeshInput};
        use crate::scanline_rasterizer::{build_plot_data, compute_grid, rasterize};
        use tempfile::NamedTempFile;

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

        let tmp = NamedTempFile::new().unwrap();
        write_geom3(&state, &plot, tmp.path()).expect("write_geom3 must succeed");

        let content = std::fs::read_to_string(tmp.path()).unwrap();

        // Header line 1: dx dy
        assert!(content.contains("1"), "file must contain grid spacing");

        // nro line
        let nro = state.row_records.len();
        assert!(
            content.contains(&format!(" {}", nro)),
            "file must contain nro={}", nro
        );

        // nnode nelem line
        let nnode = plot.nodes.len();
        assert!(
            content.contains(&format!("{}", nnode)),
            "file must mention nnode={}", nnode
        );

        // Row records are 30-char lines.
        if nro > 0 {
            let rr = &state.row_records[0];
            let expected = fmt_5i6(
                rr.i1 as i64, rr.i2 as i64, rr.j0 as i64,
                rr.kn_left as i64, rr.kn_right as i64,
            );
            assert!(
                content.contains(&expected),
                "file must contain first row record: {expected:?}"
            );
        }
    }
}

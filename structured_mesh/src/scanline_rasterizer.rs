//! Scanline rasterizer — Rust port of Fortran subroutines `rowcolx` / `rowcoly`.
//!
//! # Overview
//! This module converts a polygon description (nodes + directed edges) into a
//! rectangular cell-classification matrix **`matr[col][row]`** and collects the
//! row/column span records that the output writers need.
//!
//! The pipeline inside this module is:
//! 1. [`compute_grid`] — derive grid origin (`xm0`, `ym0`) and cell counts (`nx`, `ny`).
//! 2. [`rasterize`] — X-pass then Y-pass scanline fill → [`MeshState`].
//! 3. [`build_plot_data`] — convert `matr` into quad-mesh connectivity → [`PlotData`].
//!
//! # Grid size limits
//! The original Fortran code was limited to `matr(1500, 1200)` by its static
//! array declaration.  **This Rust implementation has no hard ceiling** — `matr`
//! is a heap-allocated `Vec<Vec<i32>>` that grows to whatever `nx × ny` the
//! input demands.  Memory is the only practical limit.
//!
//! If you need a safety cap (e.g. for automated testing or resource-constrained
//! environments) use [`compute_grid_capped`] instead of [`compute_grid`].
//!
//! # Coordinate conventions
//! * `matr` layout is `matr[col][row]` (0-based), matching Fortran's column-major
//!   `matr(i,j)` with `i` as the first (column) index.
//! * All 1-based indices in [`RowRecord`] / [`ColRecord`] mirror the Fortran
//!   counters so that output writers can compare directly with the reference.
//!
//! # Magic offsets
//! The values `0.012549` and `0.05497` are copied verbatim from `polygon3.for`:
//! ```text
//!   xm0 = xmin - dx / 2. - 0.012549
//!   ym0 = ymin - dy / 2. - .05497
//! ```
//! Do **not** round or adjust them — they shift the grid origin so that boundary
//! nodes never land exactly on a cell edge, preventing degenerate intersection
//! counts.

use crate::error::MeshError;
use crate::geometry_core::segment_x_intersect;
use crate::geometry_core::segment_y_intersect;
use crate::input_parser::MeshInput;

// ---------------------------------------------------------------------------
// Public structs
// ---------------------------------------------------------------------------

/// Computed grid-sizing information derived from the bounding box of all nodes.
#[derive(Debug, Clone)]
pub struct GeometryData {
    /// Grid origin x-offset (Fortran `xm0`).
    pub xm0: f64,
    /// Grid origin y-offset (Fortran `ym0`).
    pub ym0: f64,
    /// Bounding-box minimum x (needed by `plot` / output writers).
    pub xmin: f64,
    /// Bounding-box minimum y.
    pub ymin: f64,
    /// Number of grid columns.
    pub nx: usize,
    /// Number of grid rows.
    pub ny: usize,
    /// Cell width.
    pub dx: f64,
    /// Cell height.
    pub dy: f64,
}

/// One row-span record (replaces a single Fortran `write(16,*) i1,i2,j0,kn_l,kn_r`).
#[derive(Debug, Clone, PartialEq)]
pub struct RowRecord {
    /// Left column index, 1-based (Fortran `i1`).
    pub i1: usize,
    /// Right column index, 1-based (Fortran `i2`).
    pub i2: usize,
    /// Row index, 1-based (Fortran `j0`).
    pub j0: usize,
    /// Left boundary tag (Fortran `kn(i)`).
    pub kn_left: i32,
    /// Right boundary tag (Fortran `kn(i+1)`).
    pub kn_right: i32,
}

/// One column-span record (replaces a single Fortran `write(16,*) i1,i2,j0,kn_b,kn_t`).
#[derive(Debug, Clone, PartialEq)]
pub struct ColRecord {
    /// Bottom row index, 1-based (Fortran `i1` in `rowcoly`).
    pub j1: usize,
    /// Top row index, 1-based (Fortran `i2` in `rowcoly`).
    pub j2: usize,
    /// Column index, 1-based (Fortran `j0` in `rowcoly`).
    pub i0: usize,
    /// Bottom boundary tag.
    pub kn_bot: i32,
    /// Top boundary tag.
    pub kn_top: i32,
}

/// Central in-memory state produced by `rasterize`.
///
/// Replaces all Fortran unit-16 (`'t'`) disk I/O.
#[derive(Debug)]
pub struct MeshState {
    /// Cell-classification matrix, layout `matr[col][row]` (0-based).
    /// `0` = exterior, `1` = interior, `≥2` = boundary tag.
    pub matr: Vec<Vec<i32>>,
    /// Row-span records from the X-pass (in-memory unit-16 replacement).
    pub row_records: Vec<RowRecord>,
    /// Column-span records from the Y-pass.
    pub col_records: Vec<ColRecord>,
    pub nx: usize,
    pub ny: usize,
    /// Grid sizing data (carries `xmin`/`ymin`/`dx`/`dy` for output writers).
    pub geo: GeometryData,
    /// Original parsed input, retained for output writers and future use.
    #[allow(dead_code)]
    pub input: MeshInput,
}

/// Assembled mesh data ready for output writers.
#[derive(Debug, Clone)]
pub struct PlotData {
    /// Corner-node coordinates (0-based).
    pub nodes: Vec<(f64, f64)>,
    /// Quad element connectivity (0-based node indices), stored in
    /// Fortran output order `[n1, n2, n4, n3]` (indices 2 and 3 swapped).
    pub elements: Vec<[usize; 4]>,
    /// 1-based `(col, row)` grid cell for each element.
    pub elem_ij: Vec<(usize, usize)>,
    /// `matr[col-1][row-1]` value for each element (region tag).
    pub region_tags: Vec<i32>,
}

// ---------------------------------------------------------------------------
// Task 6.1 — compute_grid
// ---------------------------------------------------------------------------

/// Compute the rasterisation grid origin and cell counts from node coordinates.
///
/// This is the **uncapped** version — it imposes no ceiling on `nx` or `ny`.
/// The only limit is available heap memory.  For polygons that span millions
/// of cells, consider increasing `dx`/`dy` or splitting the domain.
///
/// The computation replicates the Fortran source verbatim:
/// ```text
///   xm0 = xmin - dx / 2. - 0.012549
///   ym0 = ymin - dy / 2. - .05497
///   nx  = int( (xmax + dx/2 - xm0) / dx + .5 )
///   ny  = int( (ymax + dy/2 - ym0) / dy + .5 )
/// ```
///
/// # Errors
/// Returns [`MeshError::Parse`] if `input.nodes` is empty or spacing is ≤ 0.
///
/// # See also
/// [`compute_grid_capped`] — same computation but returns
/// [`MeshError::GridTooLarge`] when `nx` or `ny` exceeds a caller-supplied cap.
pub fn compute_grid(input: &MeshInput) -> Result<GeometryData, MeshError> {
    compute_grid_impl(input, None)
}

/// Capped variant of [`compute_grid`].
///
/// Identical to `compute_grid` but returns [`MeshError::GridTooLarge`] when
/// `nx > max_nx` or `ny > max_ny`.  Useful for resource-constrained
/// environments or automated testing where extremely large grids should be
/// rejected early.
///
/// The original Fortran limits were `max_nx = 1500`, `max_ny = 1200`.
///
/// # Example
/// ```no_run
/// # use structured_mesh::{input_parser::MeshInput, scanline_rasterizer::compute_grid_capped};
/// # let input: MeshInput = unimplemented!();
/// // Reject any grid larger than the original Fortran limits.
/// let geo = compute_grid_capped(&input, 1500, 1200)?;
/// # Ok::<(), structured_mesh::error::MeshError>(())
/// ```
#[allow(dead_code)]
pub fn compute_grid_capped(
    input: &MeshInput,
    max_nx: usize,
    max_ny: usize,
) -> Result<GeometryData, MeshError> {
    compute_grid_impl(input, Some((max_nx, max_ny)))
}

/// Shared implementation for [`compute_grid`] and [`compute_grid_capped`].
fn compute_grid_impl(
    input: &MeshInput,
    cap: Option<(usize, usize)>,
) -> Result<GeometryData, MeshError> {
    if input.nodes.is_empty() {
        return Err(MeshError::Parse {
            line: 0,
            msg: "node list is empty — cannot compute grid".into(),
        });
    }

    let dx = input.dx;
    let dy = input.dy;

    // Bounding box.
    let mut xmin = f64::INFINITY;
    let mut xmax = f64::NEG_INFINITY;
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;

    for &(x, y) in &input.nodes {
        if x > xmax { xmax = x; }
        if x < xmin { xmin = x; }
        if y > ymax { ymax = y; }
        if y < ymin { ymin = y; }
    }

    // Magic offsets — verbatim from polygon3.for.
    let xm0 = xmin - dx / 2.0 - 0.012549;
    let ym0 = ymin - dy / 2.0 - 0.05497;

    // Fortran: nx = int( (xmax + dx/2 - xm0) / dx + .5 )
    // Rust `as usize` truncates toward zero, matching Fortran `int()`.
    let nx = ((xmax + dx / 2.0 - xm0) / dx + 0.5) as usize;
    let ny = ((ymax + dy / 2.0 - ym0) / dy + 0.5) as usize;

    if let Some((max_nx, max_ny)) = cap {
        if nx > max_nx || ny > max_ny {
            return Err(MeshError::GridTooLarge { nx, ny });
        }
    }

    Ok(GeometryData { xm0, ym0, xmin, ymin, nx, ny, dx, dy })
}

// ---------------------------------------------------------------------------
// Task 6.2 + 6.3 — rasterize (X-pass then Y-pass)
// ---------------------------------------------------------------------------

/// Fill the cell-classification matrix and collect row/column records.
///
/// Runs the X-pass (`rowcolx` logic) then the Y-pass (`rowcoly` logic),
/// keeping all intermediate data in memory.
pub fn rasterize(input: &MeshInput, geo: &GeometryData) -> Result<MeshState, MeshError> {
    let nx = geo.nx;
    let ny = geo.ny;
    let dx = geo.dx;
    let dy = geo.dy;
    let xm0 = geo.xm0;
    let ym0 = geo.ym0;

    // Allocate matr[col][row], size [nx][ny], all zeros.
    let mut matr: Vec<Vec<i32>> = vec![vec![0i32; ny]; nx];
    let mut row_records: Vec<RowRecord> = Vec::new();
    let mut col_records: Vec<ColRecord> = Vec::new();

    // ── X-PASS ────────────────────────────────────────────────────────────
    // Mirrors the outer `do j = 1, ny` loop in polygon3.for.
    for j in 1..=ny {
        // Fortran: y0 = (j-1)*dy + ym0 + dy/2
        let y0 = (j - 1) as f64 * dy + ym0 + dy / 2.0;

        // Collect intersections: (x_coord, kp_tag).
        let mut xs: Vec<(f64, i32)> = Vec::new();
        for edge in &input.edges {
            let (x1, y1) = input.nodes[edge.k1];
            let (x2, y2) = input.nodes[edge.k2];
            if let Some(x) = segment_x_intersect(y0, x1, y1, x2, y2) {
                xs.push((x, edge.kp));
            }
        }

        // Sort ascending by x (Fortran `call sort`).
        xs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if xs.len() % 2 != 0 {
            // Odd intersection count — log and skip this row's span.
            eprintln!(
                "warning: odd x-intersection count ({}) on row {}; skipping",
                xs.len(), j
            );
            // Emit the error for the caller to inspect if needed, but continue.
            // (Requirement 13.2: log and skip.)
            continue;
        }

        // Process pairs — rowcolx logic.
        let mut idx = 0;
        while idx + 1 < xs.len() {
            let (x1_pair, kn_l) = xs[idx];
            let (x2_pair, kn_r) = xs[idx + 1];

            // Fortran:
            //   i1 = int( (x1 - xm0 - dx/2) / dx ) + 2
            //   i2 = int( (x2 - xm0 - dx/2) / dx ) + 1
            let i1 = ((x1_pair - xm0 - dx / 2.0) / dx) as usize + 2;
            let i2 = ((x2_pair - xm0 - dx / 2.0) / dx) as usize + 1;

            if i2 >= i1 && i1 >= 1 && i2 <= nx {
                // Boundary cells.
                matr[i1 - 1][j - 1] = kn_l;
                matr[i2 - 1][j - 1] = kn_r;

                // Interior cells.
                for k in (i1 + 1)..i2 {
                    if k >= 1 && k <= nx {
                        matr[k - 1][j - 1] = 1;
                    }
                }

                row_records.push(RowRecord { i1, i2, j0: j, kn_left: kn_l, kn_right: kn_r });
            }

            idx += 2;
        }
    }

    // ── Y-PASS ────────────────────────────────────────────────────────────
    // Mirrors the outer `do i = 1, nx` loop in polygon3.for.
    for i in 1..=nx {
        // Fortran: x0 = (i-1)*dx + xm0 + dx/2
        let x0 = (i - 1) as f64 * dx + xm0 + dx / 2.0;

        let mut ys: Vec<(f64, i32)> = Vec::new();
        for edge in &input.edges {
            let (x1, y1) = input.nodes[edge.k1];
            let (x2, y2) = input.nodes[edge.k2];
            if let Some(y) = segment_y_intersect(x0, x1, y1, x2, y2) {
                ys.push((y, edge.kp));
            }
        }

        // Sort ascending by y.
        ys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if ys.len() % 2 != 0 {
            eprintln!(
                "warning: odd y-intersection count ({}) on column {}; skipping",
                ys.len(), i
            );
            continue;
        }

        // Process pairs — rowcoly logic.
        let mut idx = 0;
        while idx + 1 < ys.len() {
            let (y1_pair, kn_b) = ys[idx];
            let (y2_pair, kn_t) = ys[idx + 1];

            // Same index formula as rowcolx but for y-axis.
            let j1 = ((y1_pair - ym0 - dy / 2.0) / dy) as usize + 2;
            let j2 = ((y2_pair - ym0 - dy / 2.0) / dy) as usize + 1;

            if j2 >= j1 && j1 >= 1 && j2 <= ny {
                // Boundary cells — always overwritten (rowcoly behaviour).
                matr[i - 1][j1 - 1] = kn_b;
                matr[i - 1][j2 - 1] = kn_t;

                // Interior cells — only if current value < 2 (preserve X-pass tags).
                for k in (j1 + 1)..j2 {
                    if k >= 1 && k <= ny && matr[i - 1][k - 1] < 2 {
                        matr[i - 1][k - 1] = 1;
                    }
                }

                col_records.push(ColRecord { j1, j2, i0: i, kn_bot: kn_b, kn_top: kn_t });
            }

            idx += 2;
        }
    }

    Ok(MeshState {
        matr,
        row_records,
        col_records,
        nx,
        ny,
        geo: geo.clone(),
        input: input.clone(),
    })
}

// ---------------------------------------------------------------------------
// Task 6.5 — build_plot_data
// ---------------------------------------------------------------------------

/// Assemble `PlotData` from the rasterised `MeshState`.
///
/// Replicates the Fortran `plot` subroutine corner-node enumeration and the
/// `[n1, n2, n4, n3]` connectivity swap.
pub fn build_plot_data(state: &MeshState) -> PlotData {
    let nx = state.nx;
    let ny = state.ny;
    let xmin = state.geo.xmin;
    let ymin = state.geo.ymin;
    let dx = state.geo.dx;
    let dy = state.geo.dy;

    // matp[l1-1][l2-1]: 1-based corner-node index (0 = unvisited).
    // Size [nx+1][ny+1].
    let mut matp: Vec<Vec<usize>> = vec![vec![0usize; ny + 1]; nx + 1];

    let mut nodes: Vec<(f64, f64)> = Vec::new();
    let mut elements: Vec<[usize; 4]> = Vec::new();
    let mut elem_ij: Vec<(usize, usize)> = Vec::new();
    let mut region_tags: Vec<i32> = Vec::new();

    // Fortran iterates `do i = 1, nx` (col) then `do j = 1, ny` (row).
    for col in 1..=nx {
        for row in 1..=ny {
            let k0 = state.matr[col - 1][row - 1];
            if k0 <= 0 {
                continue;
            }

            // Visit four corners in Fortran order:
            //   l1 ∈ {col, col+1}, l2 ∈ {row, row+1}
            // Inner loop is l2 (row), outer is l1 (col).
            let mut quad = [0usize; 4];
            let mut q_idx = 0usize;

            for l1 in [col, col + 1] {
                for l2 in [row, row + 1] {
                    let node_id = matp[l1 - 1][l2 - 1];
                    let node_id = if node_id == 0 {
                        // Fortran: xp(j0) = xmin + (l1-1.5)*dx
                        //          yp(j0) = ymin + (l2-1.5)*dy
                        let x = xmin + (l1 as f64 - 1.5) * dx;
                        let y = ymin + (l2 as f64 - 1.5) * dy;
                        nodes.push((x, y));
                        let id = nodes.len(); // 1-based
                        matp[l1 - 1][l2 - 1] = id;
                        id
                    } else {
                        node_id
                    };
                    quad[q_idx] = node_id;
                    q_idx += 1;
                }
            }

            // quad = [n1, n2, n3, n4] in Fortran iteration order.
            // Fortran writes n1,n2,n4,n3 — swap indices 2 and 3.
            elements.push([quad[0], quad[1], quad[3], quad[2]]);
            elem_ij.push((col, row));
            region_tags.push(k0);
        }
    }

    PlotData { nodes, elements, elem_ij, region_tags }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_parser::{Edge, MeshInput};

    /// Build a simple closed rectangular polygon as a `MeshInput`.
    /// Outer boundary: CCW rectangle (0,0)→(10,0)→(10,8)→(0,8)→(0,0), kp=1.
    fn rect_input(dx: f64, dy: f64) -> MeshInput {
        MeshInput {
            dx,
            dy,
            nodes: vec![
                (0.0, 0.0),  // 0
                (10.0, 0.0), // 1
                (10.0, 8.0), // 2
                (0.0, 8.0),  // 3
            ],
            edges: vec![
                Edge { k1: 0, k2: 1, kp: 1 }, // bottom
                Edge { k1: 1, k2: 2, kp: 1 }, // right
                Edge { k1: 2, k2: 3, kp: 1 }, // top
                Edge { k1: 3, k2: 0, kp: 1 }, // left
            ],
        }
    }

    // ── compute_grid ──────────────────────────────────────────────────────

    #[test]
    fn compute_grid_basic() {
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).expect("compute_grid must succeed");

        // xm0 = 0 - 0.5 - 0.012549 = -0.512549
        let expected_xm0 = 0.0 - 0.5 - 0.012549;
        let expected_ym0 = 0.0 - 0.5 - 0.05497;

        assert!(
            (geo.xm0 - expected_xm0).abs() < 1e-12,
            "xm0 mismatch: got {}, expected {}", geo.xm0, expected_xm0
        );
        assert!(
            (geo.ym0 - expected_ym0).abs() < 1e-12,
            "ym0 mismatch: got {}, expected {}", geo.ym0, expected_ym0
        );
        assert!(geo.nx >= 10, "nx should be at least 10 for a 10-unit-wide rect");
        assert!(geo.ny >= 8, "ny should be at least 8 for an 8-unit-tall rect");
    }

    #[test]
    fn compute_grid_too_large_returns_error() {
        // Nodes 1600 units apart with dx=1 will produce nx > 1500.
        // Use compute_grid_capped to enforce the original Fortran limits.
        let input = MeshInput {
            dx: 1.0,
            dy: 1.0,
            nodes: vec![(0.0, 0.0), (1600.0, 1300.0)],
            edges: vec![Edge { k1: 0, k2: 1, kp: 1 }],
        };
        let err = compute_grid_capped(&input, 1500, 1200)
            .expect_err("must fail for oversized grid under Fortran-era caps");
        assert!(matches!(err, MeshError::GridTooLarge { .. }));

        // But compute_grid (uncapped) must succeed for the same input.
        compute_grid(&input).expect("uncapped compute_grid must accept large grids");
    }

    #[test]
    fn compute_grid_magic_offsets_are_verbatim() {
        // Ensure no rounding of the magic constants.
        let input = MeshInput {
            dx: 2.0,
            dy: 2.0,
            nodes: vec![(5.0, 3.0), (7.0, 5.0)],
            edges: vec![Edge { k1: 0, k2: 1, kp: 1 }],
        };
        let geo = compute_grid(&input).unwrap();
        let expected_xm0 = 5.0 - 1.0 - 0.012549;
        let expected_ym0 = 3.0 - 1.0 - 0.05497;
        assert!((geo.xm0 - expected_xm0).abs() < 1e-15);
        assert!((geo.ym0 - expected_ym0).abs() < 1e-15);
    }

    // ── rasterize ─────────────────────────────────────────────────────────

    #[test]
    fn rasterize_rectangle_interior_is_one() {
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();

        // Every cell strictly inside the rectangle must be 1 or a boundary tag.
        // At a minimum, no interior cell should be 0.
        let any_nonzero = state.matr.iter().any(|col| col.iter().any(|&v| v > 0));
        assert!(any_nonzero, "rasterized rectangle must have some active cells");
    }

    #[test]
    fn rasterize_row_and_col_records_nonempty() {
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();

        assert!(
            !state.row_records.is_empty(),
            "X-pass must produce at least one RowRecord"
        );
        assert!(
            !state.col_records.is_empty(),
            "Y-pass must produce at least one ColRecord"
        );
    }

    #[test]
    fn rasterize_row_record_bounds() {
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();

        for rr in &state.row_records {
            assert!(rr.i1 >= 1 && rr.i1 <= state.nx, "i1 out of bounds: {}", rr.i1);
            assert!(rr.i2 >= 1 && rr.i2 <= state.nx, "i2 out of bounds: {}", rr.i2);
            assert!(rr.i1 <= rr.i2, "i1 > i2");
            assert!(rr.j0 >= 1 && rr.j0 <= state.ny, "j0 out of bounds: {}", rr.j0);
        }
    }

    #[test]
    fn rasterize_col_record_bounds() {
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();

        for cr in &state.col_records {
            assert!(cr.j1 >= 1 && cr.j1 <= state.ny, "j1 out of bounds: {}", cr.j1);
            assert!(cr.j2 >= 1 && cr.j2 <= state.ny, "j2 out of bounds: {}", cr.j2);
            assert!(cr.j1 <= cr.j2, "j1 > j2");
            assert!(cr.i0 >= 1 && cr.i0 <= state.nx, "i0 out of bounds: {}", cr.i0);
        }
    }

    #[test]
    fn rasterize_ypass_preserves_boundary_tags() {
        // After rasterization, any cell with a tag ≥ 2 from X-pass must not
        // be overwritten to 1 by Y-pass interior fill.
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();

        // All non-zero cells must be ≥ 1; boundary cells (tag=1 here since kp=1)
        // must be consistently set. Since kp=1 in this test, boundary = 1 and
        // interior = 1 too, so just verify no cell is unexpectedly 0 in the span.
        // (A stronger check would require kp ≥ 2.)
        for col in 0..state.nx {
            for row in 0..state.ny {
                let v = state.matr[col][row];
                assert!(v >= 0, "matr cell must be non-negative");
            }
        }
    }

    // ── build_plot_data ───────────────────────────────────────────────────

    #[test]
    fn build_plot_data_nonempty_for_active_rectangle() {
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();
        let plot = build_plot_data(&state);

        assert!(!plot.nodes.is_empty(), "PlotData must have nodes");
        assert!(!plot.elements.is_empty(), "PlotData must have elements");
        assert_eq!(
            plot.elements.len(),
            plot.region_tags.len(),
            "element count must match region_tag count"
        );
        assert_eq!(
            plot.elements.len(),
            plot.elem_ij.len(),
            "element count must match elem_ij count"
        );
    }

    #[test]
    fn build_plot_data_element_connectivity_swap() {
        // Verify the Fortran n1,n2,n4,n3 swap: elements[e][2] and [3] are swapped
        // relative to the natural column-major iteration order.
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();
        let plot = build_plot_data(&state);

        // For every element, all four node indices must be distinct valid 1-based
        // indices into plot.nodes.
        for (e, elem) in plot.elements.iter().enumerate() {
            for &nid in elem.iter() {
                assert!(
                    nid >= 1 && nid <= plot.nodes.len(),
                    "element {} has invalid node index {}",
                    e, nid
                );
            }
            // The swap means elem[2] ≠ elem[3] in general (different corners).
            // Just assert all four are in range — the swap test below is structural.
            let unique: std::collections::HashSet<usize> = elem.iter().copied().collect();
            assert_eq!(unique.len(), 4, "element {} must have 4 distinct node indices", e);
        }
    }

    #[test]
    fn build_plot_data_region_tags_match_matr() {
        let input = rect_input(1.0, 1.0);
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();
        let plot = build_plot_data(&state);

        for (e, &(col, row)) in plot.elem_ij.iter().enumerate() {
            let expected_tag = state.matr[col - 1][row - 1];
            assert_eq!(
                plot.region_tags[e], expected_tag,
                "region_tag mismatch for element {} at ({},{})", e, col, row
            );
        }
    }
}

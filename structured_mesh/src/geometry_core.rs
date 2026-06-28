//! Winding-order classification, signed-area, point-in-polygon, and
//! line-segment intersection utilities.
//!
//! ## Why this module exists
//!
//! The original Fortran code used an ad-hoc polygon-index convention to
//! distinguish outer boundaries from holes.  This module replaces that with
//! proper **winding-order** classification:
//!
//! - **CCW** (counter-clockwise, positive signed area) → outer boundary
//! - **CW** (clockwise, negative signed area) → hole / interior void
//!
//! ## Key functions
//!
//! | Function | Description |
//! |---|---|
//! | [`signed_area`] | Shoelace formula — positive = CCW, negative = CW |
//! | [`winding_order`] | Classify a polygon as [`CCW`](WindingOrder::CCW), [`CW`](WindingOrder::CW), or [`Degenerate`](WindingOrder::Degenerate) |
//! | [`centroid`] | Arithmetic mean of vertex coordinates |
//! | [`point_in_polygon`] | Horizontal ray-cast with the same half-open interval as the scanline passes |
//! | [`segment_x_intersect`] | X-coordinate where a segment crosses a horizontal scanline — exact Fortran `.gt./.le.` convention |
//! | [`segment_y_intersect`] | Y-coordinate where a segment crosses a vertical scanline |
//! | [`assign_holes`] | Assign each CW hole to its innermost enclosing CCW outer boundary |
//!
//! ## Half-open interval convention
//!
//! Both intersection functions use a **half-open interval** `(min, max]`
//! (exclusive lower bound, inclusive upper bound), matching Fortran's
//! `.gt. yy1 .and. y0 .le. yy2`.  This prevents shared vertices between
//! adjacent edges from being counted twice during the scanline fill.
//!
//! ## Signed-area formula
//!
//! The shoelace variant used here is:
//!
//! ```text
//! area = -( Σ (x[i+1] - x[i]) * (y[i+1] + y[i]) ) / 2
//! ```
//!
//! This is the Fortran-compatible form.  A positive result means CCW; negative
//! means CW.

// Public API — items are used by the rasterizer and tests; the binary
// currently calls only the intersection functions directly, so suppress
// dead-code lints for the rest of the module.
#![allow(dead_code)]

use crate::error::MeshError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindingOrder {
    /// Counter-clockwise — positive signed area.
    CCW,
    /// Clockwise — negative signed area.
    CW,
    /// Degenerate polygon (collinear vertices or zero area).
    Degenerate,
}

/// A closed polygon boundary together with its mesh region tag.
#[derive(Debug)]
pub struct Boundary {
    /// Vertex coordinates of the closed polygon (last vertex NOT repeated).
    pub nodes: Vec<(f64, f64)>,
    /// Mesh region tag (matches `kp` from the geometry file).
    pub region_tag: i32,
}

/// An outer boundary together with all hole boundaries contained within it.
#[derive(Debug)]
pub struct PolygonGroup {
    pub outer: Boundary,
    pub holes: Vec<Boundary>,
}

// ---------------------------------------------------------------------------
// Task 4.1 — signed_area / winding_order
// ---------------------------------------------------------------------------

/// Signed area of a polygon using the Fortran-compatible shoelace variant.
///
/// The formula sums `(x[i+1] - x[i]) * (y[i+1] + y[i])`, then negates and
/// halves.  A positive result means the vertices are ordered CCW; negative
/// means CW.  This matches the convention in the original `polygon3.for`.
pub fn signed_area(polygon: &[(f64, f64)]) -> f64 {
    let n = polygon.len();
    let mut area = 0.0_f64;
    for i in 0..n {
        let (x1, y1) = polygon[i];
        let (x2, y2) = polygon[(i + 1) % n];
        area += (x2 - x1) * (y2 + y1);
    }
    -area / 2.0
}

/// Classify a polygon's winding order by the sign of its `signed_area`.
///
/// * `> 1e-10`  → `CCW`
/// * `< -1e-10` → `CW`
/// * otherwise  → `Degenerate`
pub fn winding_order(polygon: &[(f64, f64)]) -> WindingOrder {
    const EPS: f64 = 1e-10;
    let a = signed_area(polygon);
    if a > EPS {
        WindingOrder::CCW
    } else if a < -EPS {
        WindingOrder::CW
    } else {
        WindingOrder::Degenerate
    }
}

// ---------------------------------------------------------------------------
// Task 4.4 — centroid / point_in_polygon
// ---------------------------------------------------------------------------

/// Arithmetic mean of all vertex coordinates.
///
/// Returns `(0.0, 0.0)` for an empty polygon (caller must ensure non-empty
/// when using the result for containment tests).
pub fn centroid(polygon: &[(f64, f64)]) -> (f64, f64) {
    let n = polygon.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    let (sx, sy) = polygon
        .iter()
        .fold((0.0_f64, 0.0_f64), |(ax, ay), &(x, y)| (ax + x, ay + y));
    (sx / n as f64, sy / n as f64)
}

/// Ray-casting point-in-polygon test using a horizontal ray to `+∞`.
///
/// Uses the same half-open interval as `segment_x_intersect`:
/// a crossing is counted when `y > min(y1, y2) && y <= max(y1, y2)`.
/// Returns `true` if the crossing count is odd (strictly inside).
pub fn point_in_polygon(pt: (f64, f64), polygon: &[(f64, f64)]) -> bool {
    let (px, py) = pt;
    let n = polygon.len();
    let mut crossings: usize = 0;

    for i in 0..n {
        let (x1, y1) = polygon[i];
        let (x2, y2) = polygon[(i + 1) % n];

        // Use segment_x_intersect to leverage the identical half-open logic.
        if let Some(xi) = segment_x_intersect(py, x1, y1, x2, y2) {
            // Only count intersections to the right of the query point.
            if xi > px {
                crossings += 1;
            }
        }
    }

    crossings % 2 == 1
}

// ---------------------------------------------------------------------------
// Task 4.5 — segment_x_intersect / segment_y_intersect
// ---------------------------------------------------------------------------

/// X-coordinate where the segment `(x1,y1)–(x2,y2)` crosses the horizontal
/// scanline `y = y0`, using the Fortran half-open interval `(yy1, yy2]`.
///
/// Returns `None` when `y0` is outside the half-open interval or the segment
/// is horizontal (`yy1 == yy2` means the condition is always false, so no
/// explicit denominator guard is needed beyond the interval check).
pub fn segment_x_intersect(y0: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> Option<f64> {
    let yy1 = y1.min(y2);
    let yy2 = y1.max(y2);
    if y0 > yy1 && y0 <= yy2 {
        let r1 = (y2 - y0) / (y2 - y1);
        let r2 = 1.0 - r1;
        Some(r1 * x1 + r2 * x2)
    } else {
        None
    }
}

/// Y-coordinate where the segment `(x1,y1)–(x2,y2)` crosses the vertical
/// scanline `x = x0`, using the Fortran half-open interval `(xx1, xx2]`.
///
/// Returns `None` when `x0` is outside the half-open interval or the segment
/// is vertical.
pub fn segment_y_intersect(x0: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> Option<f64> {
    let xx1 = x1.min(x2);
    let xx2 = x1.max(x2);
    if x0 > xx1 && x0 <= xx2 {
        let r1 = (x2 - x0) / (x2 - x1);
        let r2 = 1.0 - r1;
        Some(r1 * y1 + r2 * y2)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Task 4.7 — assign_holes
// ---------------------------------------------------------------------------

/// Assigns CW hole boundaries to their enclosing CCW outer boundary.
///
/// For each CW boundary (hole), the function computes its centroid and finds
/// the smallest-area CCW outer boundary that contains that centroid.  If a
/// hole has no enclosing outer boundary, `MeshError::OrphanedHole` is
/// returned.  Degenerate boundaries (neither CCW nor CW) are silently
/// ignored.
///
/// # Algorithm
/// 1. Partition `boundaries` into outers (CCW) and holes (CW) by index.
/// 2. For each hole, test centroid containment against every outer; pick the
///    one with the smallest absolute signed area (innermost enclosing outer).
/// 3. Return a `Vec<PolygonGroup>` parallel to the outers list.
pub fn assign_holes(boundaries: &[Boundary]) -> Result<Vec<PolygonGroup>, MeshError> {
    // --- 1. Partition by winding order, keeping original indices. ---
    let outer_indices: Vec<usize> = boundaries
        .iter()
        .enumerate()
        .filter(|(_, b)| winding_order(&b.nodes) == WindingOrder::CCW)
        .map(|(i, _)| i)
        .collect();

    let hole_indices: Vec<usize> = boundaries
        .iter()
        .enumerate()
        .filter(|(_, b)| winding_order(&b.nodes) == WindingOrder::CW)
        .map(|(i, _)| i)
        .collect();

    // outers_holes[k] = list of hole indices (into `hole_indices`) assigned
    // to the k-th entry of `outer_indices`.
    let mut outers_holes: Vec<Vec<usize>> = vec![vec![]; outer_indices.len()];

    // --- 2. Assign each hole to its innermost enclosing outer. ---
    for (h_slot, &h_idx) in hole_indices.iter().enumerate() {
        let hole = &boundaries[h_idx];
        let c = centroid(&hole.nodes);

        let mut best_outer_slot: Option<usize> = None;
        let mut min_area = f64::INFINITY;

        for (o_slot, &o_idx) in outer_indices.iter().enumerate() {
            let outer = &boundaries[o_idx];
            if point_in_polygon(c, &outer.nodes) {
                let area = signed_area(&outer.nodes).abs();
                if area < min_area {
                    min_area = area;
                    best_outer_slot = Some(o_slot);
                }
            }
        }

        match best_outer_slot {
            Some(slot) => outers_holes[slot].push(h_slot),
            None => {
                return Err(MeshError::OrphanedHole {
                    hole_region_tag: hole.region_tag,
                });
            }
        }
    }

    // --- 3. Construct the PolygonGroup vec. ---
    let mut groups: Vec<PolygonGroup> = Vec::with_capacity(outer_indices.len());

    for (o_slot, &o_idx) in outer_indices.iter().enumerate() {
        let outer_b = &boundaries[o_idx];
        let outer = Boundary {
            nodes: outer_b.nodes.clone(),
            region_tag: outer_b.region_tag,
        };

        let holes: Vec<Boundary> = outers_holes[o_slot]
            .iter()
            .map(|&h_slot| {
                let h_idx = hole_indices[h_slot];
                let hb = &boundaries[h_idx];
                Boundary {
                    nodes: hb.nodes.clone(),
                    region_tag: hb.region_tag,
                }
            })
            .collect();

        groups.push(PolygonGroup { outer, holes });
    }

    Ok(groups)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- signed_area / winding_order ---

    #[test]
    fn unit_square_ccw() {
        // Vertices listed counter-clockwise: bottom-left → bottom-right →
        // top-right → top-left.
        let square = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let area = signed_area(&square);
        assert!(
            area > 0.0,
            "CCW square must have positive signed area, got {area}"
        );
        assert_eq!(winding_order(&square), WindingOrder::CCW);
    }

    #[test]
    fn unit_square_cw() {
        // Reverse the CCW order → clockwise.
        let square = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)];
        let area = signed_area(&square);
        assert!(
            area < 0.0,
            "CW square must have negative signed area, got {area}"
        );
        assert_eq!(winding_order(&square), WindingOrder::CW);
    }

    #[test]
    fn collinear_points_degenerate() {
        let line = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        assert_eq!(winding_order(&line), WindingOrder::Degenerate);
    }

    // --- segment_x_intersect ---

    #[test]
    fn seg_x_at_upper_bound_returns_some() {
        // Half-open interval (0, 1]: y0 == yy2 should return Some.
        let result = segment_x_intersect(1.0, 0.0, 0.0, 1.0, 1.0);
        assert!(
            result.is_some(),
            "y0 == upper bound must return Some (half-open includes upper)"
        );
    }

    #[test]
    fn seg_x_at_lower_bound_returns_none() {
        // Half-open interval (0, 1]: y0 == yy1 (lower bound) must return None.
        let result = segment_x_intersect(0.0, 0.0, 0.0, 1.0, 1.0);
        assert!(
            result.is_none(),
            "y0 == lower bound must return None (half-open excludes lower)"
        );
    }

    #[test]
    fn seg_x_horizontal_segment_returns_none() {
        // Horizontal segment: yy1 == yy2 so condition y0 > yy1 && y0 <= yy2
        // is never satisfied for any finite y0.
        let result = segment_x_intersect(0.5, 0.0, 0.5, 1.0, 0.5);
        assert!(
            result.is_none(),
            "horizontal segment must always return None"
        );
    }

    // --- point_in_polygon ---

    #[test]
    fn point_inside_square() {
        let square = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        assert!(
            point_in_polygon((2.0, 2.0), &square),
            "center of square must be inside"
        );
    }

    #[test]
    fn point_outside_square() {
        let square = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        assert!(
            !point_in_polygon((5.0, 5.0), &square),
            "point far outside square must not be inside"
        );
    }

    // --- assign_holes ---

    #[test]
    fn assign_holes_one_outer_one_hole() {
        // CCW outer: 10×10 square.
        let outer = Boundary {
            nodes: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            region_tag: 1,
        };
        // CW hole: small square inside the outer.
        let hole = Boundary {
            nodes: vec![(2.0, 2.0), (2.0, 4.0), (4.0, 4.0), (4.0, 2.0)],
            region_tag: 2,
        };

        let boundaries = vec![outer, hole];
        let groups = assign_holes(&boundaries).expect("assign_holes must succeed");

        assert_eq!(groups.len(), 1, "should produce exactly one PolygonGroup");
        assert_eq!(groups[0].outer.region_tag, 1, "outer region_tag must match");
        assert_eq!(
            groups[0].holes.len(),
            1,
            "outer must have exactly one hole assigned"
        );
    }

    #[test]
    fn assign_holes_orphaned_hole_returns_error() {
        // A single CW boundary with no enclosing CCW outer.
        let orphan = Boundary {
            nodes: vec![(2.0, 2.0), (2.0, 4.0), (4.0, 4.0), (4.0, 2.0)],
            region_tag: 99,
        };

        let boundaries = vec![orphan];
        let result = assign_holes(&boundaries);

        match result {
            Err(MeshError::OrphanedHole { hole_region_tag }) => {
                assert_eq!(
                    hole_region_tag, 99,
                    "orphaned hole error must carry the correct region_tag"
                );
            }
            other => panic!("expected OrphanedHole error, got {other:?}"),
        }
    }

    #[test]
    fn assign_holes_two_outers_no_holes() {
        // Two separate CCW boundaries, no holes.
        let outer_a = Boundary {
            nodes: vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)],
            region_tag: 1,
        };
        let outer_b = Boundary {
            nodes: vec![(10.0, 0.0), (15.0, 0.0), (15.0, 5.0), (10.0, 5.0)],
            region_tag: 2,
        };

        let boundaries = vec![outer_a, outer_b];
        let groups = assign_holes(&boundaries).expect("assign_holes must succeed");

        assert_eq!(groups.len(), 2, "should produce two PolygonGroups");
        assert!(groups[0].holes.is_empty(), "first group must have no holes");
        assert!(
            groups[1].holes.is_empty(),
            "second group must have no holes"
        );
    }
}

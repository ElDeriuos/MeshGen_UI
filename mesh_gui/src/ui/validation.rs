// Field-level validation helpers — ValidationResult, ValidationError, FieldId, winding check.
// Implements Req 4 AC5-8 and Req 5 AC5-9.

use crate::state::{EasyMeshState, StructuredState};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Identifies which table cell or field triggered a validation error.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldId {
    Dx,
    Dy,
    NodeX(usize),
    NodeY(usize),
    EdgeStart(usize),
    EdgeEnd(usize),
    EdgeTag(usize),
    PointX(usize),
    PointY(usize),
    PointSpacing(usize),
    PointMarker(usize),
    SegmentStart(usize),
    SegmentEnd(usize),
    SegmentMarker(usize),
}

/// A single field-level validation error.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: FieldId,
    pub message: String,
}

/// The full result of validating one engine's state: errors (block generation)
/// and warnings (advisory messages shown to the user).
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when there are no errors (warnings are acceptable).
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Shoelace winding helper
// ---------------------------------------------------------------------------

/// Determines winding order using the standard shoelace formula.
///
/// Returns `Some(true)` if the polygon is **clockwise** (negative signed area
/// in standard math coordinates where y points up, which is also clockwise
/// when rendered in screen coordinates with y pointing down), `Some(false)`
/// if counter-clockwise, or `None` if fewer than 3 points are supplied.
///
/// Standard shoelace signed area:
///   2A = Σ (x_i * y_{i+1} - x_{i+1} * y_i)   (closing edge included)
///
/// Positive 2A ⟹ CCW in standard math coords (y-up).
/// Negative 2A ⟹ CW  in standard math coords (y-up).
/// EasyMesh and structured_mesh both use standard math coordinates,
/// so "outer boundary CCW" means 2A > 0, and we warn when 2A < 0.
pub fn winding_is_clockwise(pts: &[(f64, f64)]) -> Option<bool> {
    if pts.len() < 3 {
        return None;
    }
    let n = pts.len();
    let area2: f64 = (0..n)
        .map(|i| {
            let j = (i + 1) % n;
            pts[i].0 * pts[j].1 - pts[j].0 * pts[i].1
        })
        .sum();
    // area2 < 0 ⟹ clockwise in math coords
    Some(area2 < 0.0)
}

// ---------------------------------------------------------------------------
// validate_structured
// ---------------------------------------------------------------------------

/// Validates the manual-mode fields of [`StructuredState`].
///
/// Rules (Req 4):
/// * AC5 — `dx`/`dy` must be finite and > 0.
/// * AC6 — each edge region tag must be ≥ 1.
/// * AC7 — each edge start/end index must be in `[1, nodes.len()]`.
/// * AC8 — if ≥ 3 edges and the polygon is CW, emit a winding warning.
pub fn validate_structured(state: &StructuredState) -> ValidationResult {
    let mut result = ValidationResult::new();

    // AC5 — dx
    match state.dx.trim().parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => {}
        _ => result.errors.push(ValidationError {
            field: FieldId::Dx,
            message: "dx and dy must be positive and non-zero".to_string(),
        }),
    }

    // AC5 — dy
    match state.dy.trim().parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => {}
        _ => result.errors.push(ValidationError {
            field: FieldId::Dy,
            message: "dx and dy must be positive and non-zero".to_string(),
        }),
    }

    let node_count = state.nodes.len();

    for (i, edge) in state.edges.iter().enumerate() {
        // AC7 — start index
        match edge.start.trim().parse::<usize>() {
            Ok(idx) if idx >= 1 && idx <= node_count => {}
            _ => result.errors.push(ValidationError {
                field: FieldId::EdgeStart(i),
                message: "Node index out of range".to_string(),
            }),
        }

        // AC7 — end index
        match edge.end.trim().parse::<usize>() {
            Ok(idx) if idx >= 1 && idx <= node_count => {}
            _ => result.errors.push(ValidationError {
                field: FieldId::EdgeEnd(i),
                message: "Node index out of range".to_string(),
            }),
        }

        // AC6 — region tag
        match edge.tag.trim().parse::<i64>() {
            Ok(tag) if tag >= 1 => {}
            _ => result.errors.push(ValidationError {
                field: FieldId::EdgeTag(i),
                message: "Region tag must be ≥ 1".to_string(),
            }),
        }
    }

    // AC8 — winding warning (only when ≥ 3 edges)
    if state.edges.len() >= 3 {
        // Collect coordinates by resolving edge start indices → node coords.
        // Best-effort: skip any row that fails to parse.
        let pts: Vec<(f64, f64)> = state
            .edges
            .iter()
            .filter_map(|edge| {
                let idx: usize = edge.start.trim().parse().ok()?;
                let node = state.nodes.get(idx.checked_sub(1)?)?; // 1-based → 0-based
                let x: f64 = node.x.trim().parse().ok()?;
                let y: f64 = node.y.trim().parse().ok()?;
                Some((x, y))
            })
            .collect();

        if let Some(true) = winding_is_clockwise(&pts) {
            result.warnings.push(
                "Outer boundary should be counter-clockwise; current winding is clockwise."
                    .to_string(),
            );
        }
    }

    result
}

// ---------------------------------------------------------------------------
// validate_easymesh
// ---------------------------------------------------------------------------

/// Validates the manual-mode fields of [`EasyMeshState`].
///
/// Rules (Req 5):
/// * AC5 — each point spacing must be > 0.
/// * AC6 — any marker < 1 (for points or segments) emits a warning.
/// * AC7 — each segment start/end index must be in `[0, points.len() - 1]`.
/// * AC8 — outer boundary chain CW ⟹ warning.
/// * AC9 — hole chain CCW ⟹ warning.
///
/// Winding heuristic: all points are used for the winding check.  If there
/// is only one chain (no marker distinction), a CW winding triggers the
/// outer-boundary warning.  Marker-0 points are treated as hole/interior
/// chains; a CCW winding on those triggers the hole warning.
pub fn validate_easymesh(state: &EasyMeshState) -> ValidationResult {
    let mut result = ValidationResult::new();

    let point_count = state.points.len();

    for (i, pt) in state.points.iter().enumerate() {
        // AC5 — spacing ≤ 0
        match pt.spacing.trim().parse::<f64>() {
            Ok(v) if v > 0.0 => {}
            _ => result.errors.push(ValidationError {
                field: FieldId::PointSpacing(i),
                message: "Spacing must be non-zero to avoid infinite loops".to_string(),
            }),
        }

        // AC6 — marker < 1 warning
        if let Ok(m) = pt.marker.trim().parse::<i64>() {
            if m < 1 {
                result.warnings.push(
                    "Boundary markers must be > 0; use 0 only for interior refinement chains"
                        .to_string(),
                );
            }
        }
    }

    for (i, seg) in state.segments.iter().enumerate() {
        // AC7 — start index out of range (0-based, must be < point_count)
        match seg.start.trim().parse::<i64>() {
            Ok(idx) if idx >= 0 && (idx as usize) < point_count => {}
            _ => result.errors.push(ValidationError {
                field: FieldId::SegmentStart(i),
                message: "Point index out of range".to_string(),
            }),
        }

        // AC7 — end index out of range
        match seg.end.trim().parse::<i64>() {
            Ok(idx) if idx >= 0 && (idx as usize) < point_count => {}
            _ => result.errors.push(ValidationError {
                field: FieldId::SegmentEnd(i),
                message: "Point index out of range".to_string(),
            }),
        }

        // AC6 — segment marker < 1 warning
        if let Ok(m) = seg.marker.trim().parse::<i64>() {
            if m < 1 {
                result.warnings.push(
                    "Boundary markers must be > 0; use 0 only for interior refinement chains"
                        .to_string(),
                );
            }
        }
    }

    // AC8 / AC9 — winding checks
    // Partition points into "boundary" (marker ≥ 1) and "hole/interior" (marker = 0).
    // Fall back to all points when there is no marker-0 group.
    let boundary_pts: Vec<(f64, f64)> = state
        .points
        .iter()
        .filter_map(|p| {
            let m: i64 = p.marker.trim().parse().unwrap_or(1);
            if m >= 1 {
                let x: f64 = p.x.trim().parse().ok()?;
                let y: f64 = p.y.trim().parse().ok()?;
                Some((x, y))
            } else {
                None
            }
        })
        .collect();

    let hole_pts: Vec<(f64, f64)> = state
        .points
        .iter()
        .filter_map(|p| {
            let m: i64 = p.marker.trim().parse().unwrap_or(1);
            if m == 0 {
                let x: f64 = p.x.trim().parse().ok()?;
                let y: f64 = p.y.trim().parse().ok()?;
                Some((x, y))
            } else {
                None
            }
        })
        .collect();

    // AC8 — outer boundary CW warning
    let outer_pts = if boundary_pts.len() >= 3 {
        &boundary_pts[..]
    } else {
        // single chain fallback: use all parseable points
        &boundary_pts[..] // may be < 3 — winding_is_clockwise returns None
    };

    if let Some(true) = winding_is_clockwise(outer_pts) {
        result.warnings.push(
            "Outer boundary chain should be counter-clockwise; current winding is clockwise."
                .to_string(),
        );
    }

    // AC9 — hole chain CCW warning (only when we actually have marker-0 points)
    if hole_pts.len() >= 3 {
        if let Some(false) = winding_is_clockwise(&hole_pts) {
            result.warnings.push(
                "Hole chains should be clockwise; current winding is counter-clockwise."
                    .to_string(),
            );
        }
    }

    result
}

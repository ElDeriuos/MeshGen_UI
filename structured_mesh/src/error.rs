//! Error types for the structured mesh generator.
//!
//! All fallible operations in this crate return `Result<_, MeshError>`.
//! [`MeshError`] is defined with [`thiserror`](https://docs.rs/thiserror) so
//! every variant has a human-readable [`Display`](std::fmt::Display) message
//! suitable for printing directly to the user.
//!
//! ## Variant reference
//!
//! | Variant | When it occurs |
//! |---|---|
//! | `Io` | Any OS I/O failure (file not found, permission denied, …) |
//! | `Parse` | Malformed token in the geometry or project file |
//! | `GridTooLarge` | Grid exceeds a cap set by [`compute_grid_capped`](crate::scanline_rasterizer::compute_grid_capped) |
//! | `OddIntersectionCount` | Scanline hits an odd number of edges — usually a self-intersecting polygon |
//! | `NodeIndexOutOfRange` | An edge record references a node that does not exist |
//! | `OrphanedHole` | A CW (hole) boundary has no enclosing CCW outer boundary |
//! | `DivisionByZero` | Degenerate geometry triggered a zero-denominator guard |

use thiserror::Error;

/// Canonical error type for the mesh generator.
///
/// Every recoverable and unrecoverable failure in the pipeline is represented
/// here.  `thiserror` generates the `Display` impl from the `#[error("...")]`
/// attributes; `std::io::Error` is automatically converted via `#[from]`.
///
/// `Clone` is intentionally *not* derived: `std::io::Error` (wrapped by the
/// `Io` variant) does not implement `Clone`.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum MeshError {
    /// An OS-level I/O error (file not found, permission denied, …).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Malformed input at a specific line of a source file.
    #[error("parse error at line {line}: {msg}")]
    Parse { line: usize, msg: String },

    /// The computed grid dimensions exceed available memory.
    ///
    /// This is now only triggered if the caller requests a safety cap via
    /// [`compute_grid_capped`](crate::scanline_rasterizer::compute_grid_capped).
    /// The uncapped [`compute_grid`](crate::scanline_rasterizer::compute_grid)
    /// grows dynamically with no hard ceiling.
    #[error("grid too large: requested {nx}×{ny} cells")]
    GridTooLarge { nx: usize, ny: usize },

    /// The scanline produced an odd number of boundary intersections, which
    /// violates the even-parity invariant required by the fill algorithm.
    #[error(
        "odd intersection count ({count}) on {axis}-axis scanline {scanline}; \
         expected an even number — check for self-intersecting edges"
    )]
    OddIntersectionCount {
        scanline: usize,
        axis: &'static str,
        count: usize,
    },

    /// An edge record references a node index that is outside the valid range.
    #[error(
        "node index out of range: edge {edge} references node {node}, \
         which does not exist in the node list"
    )]
    NodeIndexOutOfRange { edge: usize, node: usize },

    /// A clockwise (hole) boundary has no enclosing counter-clockwise outer
    /// boundary — it cannot be assigned to any region.
    #[error(
        "orphaned hole: CW boundary with region tag {hole_region_tag} has \
         no enclosing CCW outer boundary"
    )]
    OrphanedHole { hole_region_tag: i32 },

    /// A division-by-zero guard was triggered by degenerate geometry.
    #[error("division by zero in {context}")]
    DivisionByZero { context: &'static str },
}

// EasyMeshState — points/segments tables, CLI flags, and binary path for the EasyMesh engine.
// Includes PointRow, SegmentRow, and all output format and toggle flags.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::project::InputMode;

// ---------------------------------------------------------------------------
// Platform helper
// ---------------------------------------------------------------------------

/// Returns the expected EasyMesh binary name for the current platform (Req 13 AC5).
pub fn expected_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "Easy.exe";
    #[cfg(not(target_os = "windows"))]
    return "Easy";
}

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

/// A single row in the points table.  All fields stored as `String` for live
/// editing; parsed and validated each frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PointRow {
    pub x: String,
    pub y: String,
    pub spacing: String,
    pub marker: String,
}

impl Default for PointRow {
    fn default() -> Self {
        PointRow {
            x: "0.0".to_string(),
            y: "0.0".to_string(),
            spacing: "0.25".to_string(),
            marker: "1".to_string(),
        }
    }
}

/// A single row in the segments table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentRow {
    pub start: String,
    pub end: String,
    pub marker: String,
}

impl Default for SegmentRow {
    fn default() -> Self {
        SegmentRow {
            start: "0".to_string(),
            end: "1".to_string(),
            marker: "1".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// EasyMeshState
// ---------------------------------------------------------------------------

/// All state for the EasyMesh engine panel (Req 5, 7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EasyMeshState {
    /// File or Manual input mode.
    pub input_mode: InputMode,

    // --- File mode ---
    pub file_path: Option<PathBuf>,

    // --- Manual mode ---
    /// Point coordinate rows (x, y, spacing, marker).
    pub points: Vec<PointRow>,
    /// Index of the currently-selected point row, if any.
    pub selected_point: Option<usize>,
    /// Segment definition rows (start index, end index, marker).
    pub segments: Vec<SegmentRow>,
    /// Index of the currently-selected segment row, if any.
    pub selected_segment: Option<usize>,

    // --- Output config ---
    pub output_dir: Option<PathBuf>,

    // Req 7 AC1: format checkboxes — all unchecked by default
    pub fmt_tec: bool,
    pub fmt_vtk: bool,
    pub fmt_eps: bool,

    // Req 7 AC2: aggressiveness 0..=6, default 0
    pub aggressiveness: u8,

    // Req 7 AC3: toggle flags — all off by default
    pub skip_relaxation: bool,
    pub skip_smoothing: bool,
    pub boundary_only: bool,
    pub suppress_messages: bool,

    // Req 13 AC5: path to the EasyMesh binary
    pub easymesh_binary: PathBuf,
}

/// Resolve the EasyMesh binary path by checking a prioritised list of
/// candidate locations.
///
/// Search order (first hit wins):
/// 1. Alongside the GUI binary itself — `<exe_dir>/Easy[.exe]`
/// 2. `<exe_dir>/../EasyMesh/Src/Easy[.exe]`
/// 3. `<exe_dir>/../../EasyMesh/Src/Easy[.exe]`
/// 4. `<exe_dir>/../../../EasyMesh/Src/Easy[.exe]`
/// 5. `<exe_dir>/../../../../EasyMesh/Src/Easy[.exe]`
/// 6. Walk up from `<exe_dir>` up to 6 levels checking for a sibling
///    `EasyMesh/Src/<binary>` at each level (covers `target/debug`,
///    `target/release`, etc.).
/// 7. CWD-relative `EasyMesh/Src/<binary>` fallback (works with `cargo run`
///    from the workspace root).
pub fn resolve_easymesh_binary() -> PathBuf {
    let bin_name = expected_binary_name();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // 1. Alongside the GUI binary itself.
            let alongside = exe_dir.join(bin_name);
            if alongside.exists() {
                return alongside;
            }

            // 2–5. Relative "../EasyMesh/Src/", "../../EasyMesh/Src/", etc.
            let mut base = exe_dir.to_path_buf();
            for _ in 0..4 {
                base = match base.parent() {
                    Some(p) => p.to_path_buf(),
                    None => break,
                };
                let candidate = base.join("EasyMesh").join("Src").join(bin_name);
                if candidate.exists() {
                    return candidate;
                }
            }

            // 6. Walk up from exe_dir checking for `EasyMesh/Src/<binary>` at
            //    each level (handles nested build artefact directories).
            let mut dir = exe_dir.to_path_buf();
            for _ in 0..6 {
                let candidate = dir.join("EasyMesh").join("Src").join(bin_name);
                if candidate.exists() {
                    return candidate;
                }
                match dir.parent() {
                    Some(p) => dir = p.to_path_buf(),
                    None => break,
                }
            }
        }
    }

    // 7. Fallback: CWD-relative (works when running `cargo run` from the
    //    workspace root or from within the mesh_gui directory).
    PathBuf::from("EasyMesh").join("Src").join(bin_name)
}

impl Default for EasyMeshState {
    fn default() -> Self {
        EasyMeshState {
            input_mode: InputMode::File,
            file_path: None,
            points: Vec::new(),
            selected_point: None,
            segments: Vec::new(),
            selected_segment: None,
            output_dir: Some(PathBuf::from("outputs")),
            // Req 7 AC1
            fmt_tec: false,
            fmt_vtk: false,
            fmt_eps: false,
            // Req 7 AC2
            aggressiveness: 0,
            // Req 7 AC3
            skip_relaxation: false,
            skip_smoothing: false,
            boundary_only: false,
            suppress_messages: false,
            easymesh_binary: resolve_easymesh_binary(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Req 7 AC1 — all three format checkboxes are unchecked by default.
    #[test]
    fn default_format_flags_all_unchecked() {
        let s = EasyMeshState::default();
        assert!(!s.fmt_tec, "fmt_tec should default to false");
        assert!(!s.fmt_vtk, "fmt_vtk should default to false");
        assert!(!s.fmt_eps, "fmt_eps should default to false");
    }

    /// Req 7 AC2 — aggressiveness defaults to 0.
    #[test]
    fn default_aggressiveness_is_zero() {
        let s = EasyMeshState::default();
        assert_eq!(s.aggressiveness, 0);
    }

    /// Req 7 AC3 — all four toggle controls are off by default.
    #[test]
    fn default_toggles_all_off() {
        let s = EasyMeshState::default();
        assert!(!s.skip_relaxation, "skip_relaxation should default to false");
        assert!(!s.skip_smoothing, "skip_smoothing should default to false");
        assert!(!s.boundary_only, "boundary_only should default to false");
        assert!(!s.suppress_messages, "suppress_messages should default to false");
    }

    /// Default input mode is File.
    #[test]
    fn default_input_mode_is_file() {
        let s = EasyMeshState::default();
        assert_eq!(s.input_mode, InputMode::File);
    }

    /// Default tables are empty.
    #[test]
    fn default_tables_are_empty() {
        let s = EasyMeshState::default();
        assert!(s.points.is_empty());
        assert!(s.segments.is_empty());
    }

    /// Req 13 AC5 — binary path resolves to the platform-correct binary name,
    /// and now points to an absolute path (resolved from the executable location).
    #[test]
    fn default_binary_name_is_platform_correct() {
        let s = EasyMeshState::default();
        // The binary name component must be correct for the platform.
        let bin_name = s
            .easymesh_binary
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        #[cfg(target_os = "windows")]
        assert_eq!(bin_name, "Easy.exe", "expected Easy.exe on Windows");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(bin_name, "Easy", "expected Easy on non-Windows");

        // The parent directory must be named "Src" (platform-independent).
        let parent = s
            .easymesh_binary
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        assert_eq!(parent, "Src", "binary must live inside a 'Src' directory");
    }

    /// expected_binary_name() returns the right value for the current platform.
    #[test]
    fn expected_binary_name_for_platform() {
        let name = expected_binary_name();
        #[cfg(target_os = "windows")]
        assert_eq!(name, "Easy.exe");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, "Easy");
    }

    /// EasyMeshState round-trips through JSON.
    #[test]
    fn easymesh_state_serde_round_trip() {
        let original = EasyMeshState::default();
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: EasyMeshState = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }
}

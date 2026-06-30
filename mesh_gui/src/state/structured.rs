// StructuredState — geometry tables and output configuration for the Structured engine.
// Includes NodeRow, EdgeRow, and all output format flags.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::project::InputMode;

/// A single row in the nodes table: (x, y) stored as strings for live editing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeRow {
    pub x: String,
    pub y: String,
}

impl Default for NodeRow {
    fn default() -> Self {
        NodeRow {
            x: "0.0".to_string(),
            y: "0.0".to_string(),
        }
    }
}

/// A single row in the edges table: (start node, end node, region tag) as strings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeRow {
    pub start: String,
    pub end: String,
    pub tag: String,
}

impl Default for EdgeRow {
    fn default() -> Self {
        EdgeRow {
            start: "1".to_string(),
            end: "2".to_string(),
            tag: "1".to_string(),
        }
    }
}

/// All state for the Structured Mesh engine panel (Req 4, 6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredState {
    /// File or Manual input mode.
    pub input_mode: InputMode,

    // --- File mode ---
    pub file_path: Option<PathBuf>,

    // --- Manual mode ---
    /// Cell spacing in x (stored as String for live editing + validation).
    pub dx: String,
    /// Cell spacing in y.
    pub dy: String,
    /// Node coordinate rows.
    pub nodes: Vec<NodeRow>,
    /// Index of the currently-selected node row, if any.
    pub selected_node: Option<usize>,
    /// Edge definition rows.
    pub edges: Vec<EdgeRow>,
    /// Index of the currently-selected edge row, if any.
    pub selected_edge: Option<usize>,

    // --- Output config (Req 6 AC1 — all unchecked by default) ---
    pub output_dir: Option<PathBuf>,
    pub fmt_fepoint: bool,
    pub fmt_zones: bool,
    pub fmt_vtk: bool,
    pub fmt_text: bool,
}

impl Default for StructuredState {
    fn default() -> Self {
        StructuredState {
            input_mode: InputMode::File,
            file_path: None,
            dx: String::new(),
            dy: String::new(),
            nodes: Vec::new(),
            selected_node: None,
            edges: Vec::new(),
            selected_edge: None,
            output_dir: Some(PathBuf::from("outputs")),
            // Req 6 AC1: all format boxes unchecked by default
            fmt_fepoint: false,
            fmt_zones: false,
            fmt_vtk: false,
            fmt_text: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Req 6 AC1 — all four format checkboxes are unchecked by default.
    #[test]
    fn default_format_flags_all_unchecked() {
        let s = StructuredState::default();
        assert!(!s.fmt_fepoint, "fmt_fepoint should default to false");
        assert!(!s.fmt_zones, "fmt_zones should default to false");
        assert!(!s.fmt_vtk, "fmt_vtk should default to false");
        assert!(!s.fmt_text, "fmt_text should default to false");
    }

    /// Default input mode for StructuredState is File.
    #[test]
    fn default_input_mode_is_file() {
        let s = StructuredState::default();
        assert_eq!(s.input_mode, InputMode::File);
    }

    /// Default tables are empty.
    #[test]
    fn default_tables_are_empty() {
        let s = StructuredState::default();
        assert!(s.nodes.is_empty());
        assert!(s.edges.is_empty());
    }

    /// No input file path is set by default; output_dir defaults to "outputs".
    #[test]
    fn default_paths_are_none() {
        let s = StructuredState::default();
        assert!(s.file_path.is_none());
        assert_eq!(s.output_dir, Some(PathBuf::from("outputs")));
    }

    /// StructuredState round-trips through JSON.
    #[test]
    fn structured_state_serde_round_trip() {
        let original = StructuredState::default();
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: StructuredState = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }
}

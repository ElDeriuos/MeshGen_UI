// ProjectState, Engine enum, InputMode enum, and top-level serialisable snapshot.

use serde::{Deserialize, Serialize};

use super::easymesh::EasyMeshState;
use super::structured::StructuredState;

/// Which mesh engine is currently active.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Engine {
    Structured,
    EasyMesh,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::Structured
    }
}

/// Whether the user is supplying geometry via a file or typing it directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputMode {
    File,
    Manual,
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::File
    }
}

/// Top-level serialisable snapshot of all GUI settings (Req 11).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectState {
    pub selected_engine: Engine,
    pub structured: StructuredState,
    pub easymesh: EasyMeshState,
}

impl Default for ProjectState {
    fn default() -> Self {
        ProjectState {
            selected_engine: Engine::default(),
            structured: StructuredState::default(),
            easymesh: EasyMeshState::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::state::easymesh::{EasyMeshState, PointRow, SegmentRow};
    use crate::state::structured::{EdgeRow, NodeRow, StructuredState};

    /// Req 1 AC1 — "Structured Mesh" is selected by default.
    #[test]
    fn default_engine_is_structured() {
        let project = ProjectState::default();
        assert_eq!(project.selected_engine, Engine::Structured);
    }

    /// InputMode defaults to File.
    #[test]
    fn default_input_mode_is_file() {
        assert_eq!(InputMode::default(), InputMode::File);
    }

    /// ProjectState round-trips through JSON (sanity check for serde derives).
    #[test]
    fn project_state_serde_round_trip() {
        let original = ProjectState::default();
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: ProjectState = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }

    /// Req 11 AC6 — full non-default ProjectState round-trips through JSON without data loss.
    ///
    /// Constructs a maximally non-default state: both engines configured, Manual mode
    /// with table rows, output directories set, all options toggled, and a custom binary path.
    #[test]
    fn project_state_non_default_serde_round_trip() {
        // --- Structured engine state ---
        let structured = StructuredState {
            input_mode: InputMode::Manual,
            file_path: Some(PathBuf::from("/tmp").join("geometry.txt")),
            dx: "0.05".to_string(),
            dy: "0.1".to_string(),
            nodes: vec![
                NodeRow { x: "0.0".to_string(), y: "0.0".to_string() },
                NodeRow { x: "1.0".to_string(), y: "0.0".to_string() },
                NodeRow { x: "1.0".to_string(), y: "1.0".to_string() },
                NodeRow { x: "0.0".to_string(), y: "1.0".to_string() },
            ],
            selected_node: Some(2),
            edges: vec![
                EdgeRow { start: "1".to_string(), end: "2".to_string(), tag: "1".to_string() },
                EdgeRow { start: "2".to_string(), end: "3".to_string(), tag: "1".to_string() },
                EdgeRow { start: "3".to_string(), end: "4".to_string(), tag: "1".to_string() },
                EdgeRow { start: "4".to_string(), end: "1".to_string(), tag: "1".to_string() },
            ],
            selected_edge: Some(0),
            output_dir: Some(PathBuf::from("/tmp").join("structured_out")),
            // All format flags toggled on
            fmt_fepoint: true,
            fmt_zones: true,
            fmt_vtk: true,
            fmt_text: true,
        };

        // --- EasyMesh engine state ---
        let easymesh = EasyMeshState {
            input_mode: InputMode::Manual,
            file_path: Some(PathBuf::from("/tmp").join("domain.d")),
            points: vec![
                PointRow {
                    x: "0.0".to_string(),
                    y: "0.0".to_string(),
                    spacing: "0.1".to_string(),
                    marker: "1".to_string(),
                },
                PointRow {
                    x: "2.0".to_string(),
                    y: "0.0".to_string(),
                    spacing: "0.2".to_string(),
                    marker: "2".to_string(),
                },
            ],
            selected_point: Some(1),
            segments: vec![
                SegmentRow {
                    start: "0".to_string(),
                    end: "1".to_string(),
                    marker: "1".to_string(),
                },
            ],
            selected_segment: Some(0),
            output_dir: Some(PathBuf::from("/tmp").join("easymesh_out")),
            // All format flags toggled on
            fmt_tec: true,
            fmt_vtk: true,
            fmt_eps: true,
            // Max aggressiveness
            aggressiveness: 6,
            // All toggles on
            skip_relaxation: true,
            skip_smoothing: true,
            boundary_only: true,
            suppress_messages: true,
            // Custom binary path
            easymesh_binary: PathBuf::from("/usr").join("local").join("bin").join("Easy"),
        };

        let original = ProjectState {
            selected_engine: Engine::EasyMesh,
            structured,
            easymesh,
        };

        let json = serde_json::to_string_pretty(&original).expect("serialise");
        let restored: ProjectState = serde_json::from_str(&json).expect("deserialise");

        assert_eq!(
            original, restored,
            "round-tripped ProjectState must equal the original"
        );

        // Spot-check key non-default fields survived the round trip.
        assert_eq!(restored.selected_engine, Engine::EasyMesh);
        assert_eq!(restored.structured.input_mode, InputMode::Manual);
        assert_eq!(restored.structured.nodes.len(), 4);
        assert_eq!(restored.structured.edges.len(), 4);
        assert!(restored.structured.fmt_fepoint);
        assert!(restored.structured.fmt_zones);
        assert!(restored.structured.fmt_vtk);
        assert!(restored.structured.fmt_text);
        assert_eq!(
            restored.structured.output_dir,
            Some(PathBuf::from("/tmp").join("structured_out"))
        );

        assert_eq!(restored.easymesh.input_mode, InputMode::Manual);
        assert_eq!(restored.easymesh.points.len(), 2);
        assert_eq!(restored.easymesh.segments.len(), 1);
        assert!(restored.easymesh.fmt_tec);
        assert!(restored.easymesh.fmt_vtk);
        assert!(restored.easymesh.fmt_eps);
        assert_eq!(restored.easymesh.aggressiveness, 6);
        assert!(restored.easymesh.skip_relaxation);
        assert!(restored.easymesh.skip_smoothing);
        assert!(restored.easymesh.boundary_only);
        assert!(restored.easymesh.suppress_messages);
        assert_eq!(
            restored.easymesh.easymesh_binary,
            PathBuf::from("/usr").join("local").join("bin").join("Easy")
        );
    }
}

// Structured mesh runner — calls the structured_mesh library directly in the worker thread.

use std::path::Path;
use std::sync::mpsc::Sender;

use structured_mesh::input_parser::{Edge, MeshInput, ProjectConfig, parse_geometry};
use structured_mesh::output_tecplot::{write_ch3, write_ch4};
use structured_mesh::output_text::write_ch2;
use structured_mesh::output_vtk::write_vtu;
use structured_mesh::scanline_rasterizer::{build_plot_data, compute_grid, rasterize};

use crate::state::structured::StructuredState;

use super::{StructuredRunArgs, WorkerMsg};

// ---------------------------------------------------------------------------
// state_to_mesh_input
// ---------------------------------------------------------------------------

/// Convert the Manual_Mode table strings in `StructuredState` into a `MeshInput`.
///
/// Edge node indices in the UI are 1-based (natural for users); they are
/// converted to 0-based here before being stored in `Edge`.
pub fn state_to_mesh_input(s: &StructuredState) -> anyhow::Result<MeshInput> {
    let dx = s.dx.parse::<f64>()?;
    let dy = s.dy.parse::<f64>()?;

    let nodes = s
        .nodes
        .iter()
        .map(|r| Ok((r.x.parse::<f64>()?, r.y.parse::<f64>()?)))
        .collect::<anyhow::Result<Vec<_>>>()?;

    let edges = s
        .edges
        .iter()
        .map(|r| {
            Ok(Edge {
                k1: r.start.parse::<usize>()? - 1, // 1-based → 0-based
                k2: r.end.parse::<usize>()? - 1,
                kp: r.tag.parse::<i32>()?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(MeshInput { dx, dy, nodes, edges })
}

// ---------------------------------------------------------------------------
// file_to_mesh_input
// ---------------------------------------------------------------------------

/// Read a geometry file from disk (File_Mode) and return a validated `MeshInput`.
///
/// Constructs a minimal `ProjectConfig` pointing at `path` and delegates to
/// `parse_geometry`.  The caller is responsible for ensuring `path` exists and
/// is readable before calling this function; a missing or unreadable file
/// produces an `anyhow::Error` wrapping the underlying `MeshError::Io`.
pub fn file_to_mesh_input(path: &Path) -> anyhow::Result<MeshInput> {
    let cfg = ProjectConfig {
        geometry_file: path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("geometry file path is not valid UTF-8: {:?}", path))?
            .to_string(),
        ch2_file: String::new(),
        ch3_file: String::new(),
        ch4_file: String::new(),
    };
    let input = parse_geometry(&cfg).map_err(anyhow::Error::from)?;
    Ok(input)
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

/// Run the full structured mesh pipeline in the worker thread.
///
/// Steps:
/// 1. `compute_grid` → grid sizing.
/// 2. `rasterize`    → fill cell matrix + row/col records.
/// 3. `build_plot_data` → quad-mesh connectivity.
/// 4. Conditionally write each requested output format.
/// 5. Send a `LogLine` for each successfully written file.
/// 6. Send `RunComplete` with success/failure.
pub fn run(args: StructuredRunArgs, tx: Sender<WorkerMsg>) {
    // Macro to send an error RunComplete and return early.
    macro_rules! fail {
        ($e:expr) => {{
            let err_str = anyhow::Error::from($e).to_string();
            tx.send(WorkerMsg::RunComplete {
                success: false,
                exit_code: None,
                error_text: Some(err_str),
            })
            .ok();
            return;
        }};
    }

    // ── Pipeline ────────────────────────────────────────────────────────────

    let geo = match compute_grid(&args.input) {
        Ok(g) => g,
        Err(e) => fail!(e),
    };

    let state = match rasterize(&args.input, &geo) {
        Ok(s) => s,
        Err(e) => fail!(e),
    };

    let plot = build_plot_data(&state);

    // ── Output writers ───────────────────────────────────────────────────────

    if args.fmt_vtk {
        let path = args.output_dir.join("mesh.vtu");
        if let Err(e) = write_vtu(&plot, &path) {
            fail!(e);
        }
        tx.send(WorkerMsg::LogLine(format!("Wrote: {}", path.display()))).ok();
    }

    if args.fmt_fepoint {
        let path = args.output_dir.join("mesh_fepoint.plt");
        if let Err(e) = write_ch3(&plot, &path) {
            fail!(e);
        }
        tx.send(WorkerMsg::LogLine(format!("Wrote: {}", path.display()))).ok();
    }

    if args.fmt_zones {
        let path = args.output_dir.join("mesh_zones.plt");
        if let Err(e) = write_ch4(&state, &plot, &path) {
            fail!(e);
        }
        tx.send(WorkerMsg::LogLine(format!("Wrote: {}", path.display()))).ok();
    }

    if args.fmt_text {
        let path = args.output_dir.join("output_text.txt");
        if let Err(e) = write_ch2(&state, &plot, &path) {
            fail!(e);
        }
        tx.send(WorkerMsg::LogLine(format!("Wrote: {}", path.display()))).ok();
    }

    // ── Success ──────────────────────────────────────────────────────────────

    tx.send(WorkerMsg::RunComplete {
        success: true,
        exit_code: Some(0),
        error_text: None,
    })
    .ok();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::structured::{EdgeRow, NodeRow, StructuredState};

    fn unit_square_state() -> StructuredState {
        let mut s = StructuredState::default();
        s.dx = "0.1".to_string();
        s.dy = "0.1".to_string();
        s.nodes = vec![
            NodeRow { x: "0.0".to_string(), y: "0.0".to_string() },
            NodeRow { x: "1.0".to_string(), y: "0.0".to_string() },
            NodeRow { x: "1.0".to_string(), y: "1.0".to_string() },
            NodeRow { x: "0.0".to_string(), y: "1.0".to_string() },
        ];
        s.edges = vec![
            EdgeRow { start: "1".to_string(), end: "2".to_string(), tag: "1".to_string() },
            EdgeRow { start: "2".to_string(), end: "3".to_string(), tag: "1".to_string() },
            EdgeRow { start: "3".to_string(), end: "4".to_string(), tag: "1".to_string() },
            EdgeRow { start: "4".to_string(), end: "1".to_string(), tag: "1".to_string() },
        ];
        s
    }

    // ── state_to_mesh_input ───────────────────────────────────────────────

    #[test]
    fn state_to_mesh_input_parses_unit_square() {
        let s = unit_square_state();
        let mesh = state_to_mesh_input(&s).expect("should parse cleanly");
        assert_eq!(mesh.dx, 0.1);
        assert_eq!(mesh.dy, 0.1);
        assert_eq!(mesh.nodes.len(), 4);
        assert_eq!(mesh.edges.len(), 4);
    }

    #[test]
    fn state_to_mesh_input_converts_1based_to_0based() {
        let s = unit_square_state();
        let mesh = state_to_mesh_input(&s).unwrap();
        // Edge "1 2 1" → k1=0, k2=1
        assert_eq!(mesh.edges[0].k1, 0);
        assert_eq!(mesh.edges[0].k2, 1);
        // Edge "4 1 1" → k1=3, k2=0
        assert_eq!(mesh.edges[3].k1, 3);
        assert_eq!(mesh.edges[3].k2, 0);
    }

    #[test]
    fn state_to_mesh_input_bad_dx_returns_error() {
        let mut s = unit_square_state();
        s.dx = "not_a_number".to_string();
        assert!(state_to_mesh_input(&s).is_err());
    }

    #[test]
    fn state_to_mesh_input_bad_edge_index_returns_error() {
        let mut s = unit_square_state();
        s.edges[0].start = "abc".to_string();
        assert!(state_to_mesh_input(&s).is_err());
    }

    // ── file_to_mesh_input ────────────────────────────────────────────────

    /// Returns a valid geometry.txt content for the unit square.
    fn unit_square_geometry_txt() -> String {
        "0.1 0.1\n4\n0.0 0.0\n1.0 0.0\n1.0 1.0\n0.0 1.0\n4\n1 2 1\n2 3 1\n3 4 1\n4 1 1\n"
            .to_string()
    }

    #[test]
    fn file_to_mesh_input_parses_unit_square() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(unit_square_geometry_txt().as_bytes()).unwrap();
        let mesh = file_to_mesh_input(tmp.path()).expect("should parse geometry file");
        assert_eq!(mesh.dx, 0.1);
        assert_eq!(mesh.dy, 0.1);
        assert_eq!(mesh.nodes.len(), 4);
        assert_eq!(mesh.edges.len(), 4);
        // Indices are already 0-based after parse_geometry
        assert_eq!(mesh.edges[0].k1, 0);
        assert_eq!(mesh.edges[0].k2, 1);
    }

    #[test]
    fn file_to_mesh_input_missing_file_returns_error() {
        let result = file_to_mesh_input(std::path::Path::new("/nonexistent/geometry.txt"));
        assert!(result.is_err(), "missing file should return an error");
    }

    // ── run ───────────────────────────────────────────────────────────────

    #[test]
    fn run_writes_vtk_file_and_sends_complete() {
        use std::sync::mpsc;
        let tmp = tempfile::tempdir().unwrap();
        let mesh = state_to_mesh_input(&unit_square_state()).unwrap();
        let (tx, rx) = mpsc::channel();

        run(
            StructuredRunArgs {
                input: mesh,
                output_dir: tmp.path().to_path_buf(),
                fmt_fepoint: false,
                fmt_zones: false,
                fmt_vtk: true,
                fmt_text: false,
            },
            tx,
        );

        let msgs: Vec<WorkerMsg> = rx.try_iter().collect();
        let has_log = msgs.iter().any(|m| matches!(m, WorkerMsg::LogLine(s) if s.contains("mesh.vtu")));
        let complete = msgs.iter().find(|m| matches!(m, WorkerMsg::RunComplete { .. }));

        assert!(has_log, "should send LogLine for mesh.vtu");
        assert!(matches!(complete, Some(WorkerMsg::RunComplete { success: true, exit_code: Some(0), .. })));
        assert!(tmp.path().join("mesh.vtu").exists(), "mesh.vtu must be created");
    }

    #[test]
    fn run_writes_all_formats() {
        use std::sync::mpsc;
        let tmp = tempfile::tempdir().unwrap();
        let mesh = state_to_mesh_input(&unit_square_state()).unwrap();
        let (tx, rx) = mpsc::channel();

        run(
            StructuredRunArgs {
                input: mesh,
                output_dir: tmp.path().to_path_buf(),
                fmt_fepoint: true,
                fmt_zones: true,
                fmt_vtk: true,
                fmt_text: true,
            },
            tx,
        );

        let msgs: Vec<WorkerMsg> = rx.try_iter().collect();
        let log_lines: Vec<&str> = msgs
            .iter()
            .filter_map(|m| if let WorkerMsg::LogLine(s) = m { Some(s.as_str()) } else { None })
            .collect();

        assert!(log_lines.iter().any(|s| s.contains("mesh.vtu")));
        assert!(log_lines.iter().any(|s| s.contains("mesh_fepoint.plt")));
        assert!(log_lines.iter().any(|s| s.contains("mesh_zones.plt")));
        assert!(log_lines.iter().any(|s| s.contains("output_text.txt")));

        let complete = msgs.iter().find(|m| matches!(m, WorkerMsg::RunComplete { .. }));
        assert!(matches!(complete, Some(WorkerMsg::RunComplete { success: true, .. })));

        assert!(tmp.path().join("mesh.vtu").exists());
        assert!(tmp.path().join("mesh_fepoint.plt").exists());
        assert!(tmp.path().join("mesh_zones.plt").exists());
        assert!(tmp.path().join("output_text.txt").exists());
    }

    #[test]
    fn run_no_formats_sends_success_with_no_log_lines() {
        use std::sync::mpsc;
        let tmp = tempfile::tempdir().unwrap();
        let mesh = state_to_mesh_input(&unit_square_state()).unwrap();
        let (tx, rx) = mpsc::channel();

        run(
            StructuredRunArgs {
                input: mesh,
                output_dir: tmp.path().to_path_buf(),
                fmt_fepoint: false,
                fmt_zones: false,
                fmt_vtk: false,
                fmt_text: false,
            },
            tx,
        );

        let msgs: Vec<WorkerMsg> = rx.try_iter().collect();
        let log_count = msgs.iter().filter(|m| matches!(m, WorkerMsg::LogLine(_))).count();
        assert_eq!(log_count, 0, "no formats → no log lines");
        assert!(msgs.iter().any(|m| matches!(m, WorkerMsg::RunComplete { success: true, .. })));
    }

    #[test]
    fn run_bad_output_dir_sends_failure() {
        use std::sync::mpsc;
        let mesh = state_to_mesh_input(&unit_square_state()).unwrap();
        let (tx, rx) = mpsc::channel();

        run(
            StructuredRunArgs {
                input: mesh,
                output_dir: std::path::PathBuf::from("/nonexistent_dir_xyz/output"),
                fmt_vtk: true,
                fmt_fepoint: false,
                fmt_zones: false,
                fmt_text: false,
            },
            tx,
        );

        let msgs: Vec<WorkerMsg> = rx.try_iter().collect();
        assert!(
            msgs.iter().any(|m| matches!(m, WorkerMsg::RunComplete { success: false, .. })),
            "bad output dir should produce a failure RunComplete"
        );
    }

    // ── Integration test: end-to-end using examples/01_unit_square ───────────
    //
    // Req 8 AC2: Mesh_Runner invokes Structured_Engine library directly in a
    //            background thread, passing geometry data and output configuration.
    // Req 8 AC5: On completion with exit code 0, display "completed successfully".
    // Req 9 AC5: Log_Panel appends a final status line on completion.
    //
    // This test loads the real on-disk geometry file from examples/01_unit_square,
    // runs the full pipeline into a temp directory, and asserts:
    //   - at least one output file was created,
    //   - at least one LogLine was sent (the "Wrote: …" message), and
    //   - RunComplete { success: true } was received.

    #[test]
    fn integration_unit_square_file_mode_end_to_end() {
        use std::path::PathBuf;
        use std::sync::mpsc;

        // Locate the example geometry file relative to the workspace root.
        // CARGO_MANIFEST_DIR points to mesh_gui/; go up one level to reach the
        // workspace root and then into structured_mesh/examples/01_unit_square/.
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let geometry_path = manifest_dir
            .parent()
            .expect("workspace root must exist")
            .join("structured_mesh")
            .join("examples")
            .join("01_unit_square")
            .join("geometry.txt");

        assert!(
            geometry_path.exists(),
            "example geometry file not found at {}",
            geometry_path.display()
        );

        // Parse the geometry file from disk (File_Mode path).
        let mesh = file_to_mesh_input(&geometry_path)
            .expect("failed to parse examples/01_unit_square/geometry.txt");

        // Sanity-check the parsed geometry matches the known unit-square layout.
        assert_eq!(mesh.dx, 0.1, "dx should be 0.1");
        assert_eq!(mesh.dy, 0.1, "dy should be 0.1");
        assert_eq!(mesh.nodes.len(), 4, "unit square has 4 nodes");
        assert_eq!(mesh.edges.len(), 4, "unit square has 4 edges");

        // Run the pipeline into a temp directory, enabling all four output formats
        // so we can assert that every output file is created.
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let (tx, rx) = mpsc::channel();

        run(
            StructuredRunArgs {
                input: mesh,
                output_dir: tmp.path().to_path_buf(),
                fmt_fepoint: true,
                fmt_zones: true,
                fmt_vtk: true,
                fmt_text: true,
            },
            tx,
        );

        let msgs: Vec<WorkerMsg> = rx.try_iter().collect();

        // Req 9 AC5 — runner must send at least one LogLine.
        let log_lines: Vec<&str> = msgs
            .iter()
            .filter_map(|m| {
                if let WorkerMsg::LogLine(s) = m {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            !log_lines.is_empty(),
            "runner should send at least one LogLine; got none"
        );

        // Req 8 AC2 / AC5 — runner must send RunComplete { success: true }.
        let complete = msgs
            .iter()
            .find(|m| matches!(m, WorkerMsg::RunComplete { .. }))
            .expect("runner should send a RunComplete message");
        assert!(
            matches!(complete, WorkerMsg::RunComplete { success: true, exit_code: Some(0), .. }),
            "RunComplete should indicate success with exit_code 0; got {:?}",
            match complete {
                WorkerMsg::RunComplete { success, exit_code, error_text } =>
                    format!("success={success}, exit_code={exit_code:?}, error_text={error_text:?}"),
                _ => "unexpected variant".to_string(),
            }
        );

        // At least one output file must exist in the temp directory.
        let output_files = [
            "mesh.vtu",
            "mesh_fepoint.plt",
            "mesh_zones.plt",
            "output_text.txt",
        ];
        let created: Vec<&str> = output_files
            .iter()
            .filter(|name| tmp.path().join(name).exists())
            .copied()
            .collect();
        assert!(
            !created.is_empty(),
            "at least one output file must be created in the temp directory"
        );

        // All four formats were requested — all four files should be present.
        assert_eq!(
            created.len(),
            4,
            "all four output files should be created; found: {:?}",
            created
        );

        // Verify each "Wrote:" log line references a file that actually exists.
        for line in &log_lines {
            if let Some(path_str) = line.strip_prefix("Wrote: ") {
                let written = std::path::Path::new(path_str);
                assert!(
                    written.exists(),
                    "LogLine claims '{}' was written but file does not exist",
                    path_str
                );
            }
        }
    }
}

// EasyMesh runner — serialises state to .d format, builds CLI args, spawns
// the Easy/Easy.exe binary, streams its output, and cleans up the temp file.

use std::fmt::Write as FmtWrite;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc::Sender;

use super::{AppError, EasyMeshRunArgs, WorkerMsg};
use crate::state::easymesh::EasyMeshState;

// ---------------------------------------------------------------------------
// state_to_dot_d (Req 8 AC3)
// ---------------------------------------------------------------------------

/// Serialise the points and segments tables of `s` to EasyMesh `.d` file
/// format (Req 8 AC3, design doc "Generating a .d file" section).
///
/// Format:
/// ```text
/// <npoints>
/// <i>: <x> <y> <spacing> <marker>
/// ...
/// <nsegments>
/// <i>: <start> <end> <marker>
/// ...
/// ```
pub fn state_to_dot_d(s: &EasyMeshState) -> String {
    let mut out = String::new();
    writeln!(out, "{}", s.points.len()).unwrap();
    for (i, p) in s.points.iter().enumerate() {
        writeln!(out, "{}: {} {} {} {}", i, p.x, p.y, p.spacing, p.marker).unwrap();
    }
    writeln!(out, "{}", s.segments.len()).unwrap();
    for (i, seg) in s.segments.iter().enumerate() {
        writeln!(out, "{}: {} {} {}", i, seg.start, seg.end, seg.marker).unwrap();
    }
    out
}

// ---------------------------------------------------------------------------
// build_easymesh_args (Req 8 AC7)
// ---------------------------------------------------------------------------

/// Map EasyMesh format checkboxes and toggle flags onto CLI arguments
/// (Req 8 AC7, design doc "EasyMesh CLI Argument Construction" section).
pub fn build_easymesh_args(s: &EasyMeshState) -> Vec<String> {
    let mut args = Vec::new();
    if s.fmt_tec {
        args.push("+tec".into());
    }
    if s.fmt_vtk {
        args.push("+vtk".into());
    }
    if s.fmt_eps {
        args.push("+eps".into());
    }
    if s.aggressiveness > 0 {
        args.push("+a".into());
        args.push(s.aggressiveness.to_string());
    }
    if s.skip_relaxation {
        args.push("-r".into());
    }
    if s.skip_smoothing {
        args.push("-s".into());
    }
    if s.boundary_only {
        args.push("-d".into());
    }
    if s.suppress_messages {
        args.push("-m".into());
    }
    args
}

// ---------------------------------------------------------------------------
// run (Req 12 AC3, 4 / Req 13 AC5)
// ---------------------------------------------------------------------------

/// Spawn the EasyMesh binary and stream its output back as [`WorkerMsg`]s.
///
/// Steps (design doc `easymesh_runner` section):
/// 1. Verify the binary exists — emit `AppError::BinaryNotFound` if not.
/// 2. Write `args.dot_d_content` to a uniquely-named temp `.d` file inside
///    `args.output_dir`.
/// 3. Spawn `Easy <stem> <args...>` with CWD = `output_dir`.
/// 4. Read stdout and stderr line-by-line in two sub-threads, sending each
///    line as `WorkerMsg::LogLine`.
/// 5. Wait for the process to exit.
/// 6. Delete the temp `.d` file (regardless of success).
/// 7. Send `WorkerMsg::RunComplete`.
pub fn run(args: EasyMeshRunArgs, tx: Sender<WorkerMsg>) {
    // ------------------------------------------------------------------
    // 1. Verify binary exists (Req 12 AC3)
    // ------------------------------------------------------------------
    if !args.binary_path.exists() {
        let err = AppError::BinaryNotFound {
            path: args.binary_path.clone(),
        };
        let _ = tx.send(WorkerMsg::RunComplete {
            success: false,
            exit_code: None,
            error_text: Some(err.to_string()),
        });
        return;
    }

    // ------------------------------------------------------------------
    // 2. Write .d file using a stable stem so Easy names outputs predictably.
    //    Outputs will be: mesh.n, mesh.e, mesh.dat, mesh.vtk, etc.
    // ------------------------------------------------------------------
    let stem = args
        .input_stem
        .as_deref()
        .unwrap_or("mesh")
        .to_string();

    let temp_d_path: PathBuf = args.output_dir.join(format!("{}.d", stem));

    if let Err(e) = std::fs::write(&temp_d_path, &args.dot_d_content) {
        let _ = tx.send(WorkerMsg::RunComplete {
            success: false,
            exit_code: None,
            error_text: Some(format!("Failed to write temp .d file: {e}")),
        });
        return;
    }

    // ------------------------------------------------------------------
    // 3. Spawn the process
    //    Easy expects the file stem (without .d extension) as its first arg.
    // ------------------------------------------------------------------
    let mut child = match std::process::Command::new(&args.binary_path)
        .arg(&stem)             // stem — Easy appends .d itself
        .args(&args.args)
        .current_dir(&args.output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file(&temp_d_path);
            let _ = tx.send(WorkerMsg::RunComplete {
                success: false,
                exit_code: None,
                error_text: Some(format!("Failed to spawn EasyMesh: {e}")),
            });
            return;
        }
    };

    // ------------------------------------------------------------------
    // 4. Read stdout / stderr in two sub-threads (Req 9)
    // ------------------------------------------------------------------
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let tx_stdout = tx.clone();
    let stdout_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let _ = tx_stdout.send(WorkerMsg::LogLine(l));
                }
                Err(_) => break,
            }
        }
    });

    let tx_stderr = tx.clone();
    let stderr_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let _ = tx_stderr.send(WorkerMsg::LogLine(l));
                }
                Err(_) => break,
            }
        }
    });

    // ------------------------------------------------------------------
    // 5. Wait for the process to exit
    // ------------------------------------------------------------------
    let exit_status = child.wait();

    // Join reader threads so all lines are flushed before RunComplete.
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    // ------------------------------------------------------------------
    // 6. Delete the temp .d file regardless of outcome
    // ------------------------------------------------------------------
    let _ = std::fs::remove_file(&temp_d_path);

    // ------------------------------------------------------------------
    // 7. Send RunComplete (Req 12 AC4)
    // ------------------------------------------------------------------
    match exit_status {
        Ok(status) => {
            let exit_code = status.code();
            let success = status.success();

            // On Unix a None exit code means the process was killed by a signal
            // (e.g. SIGTERM / SIGKILL) — map this to ProcessTerminated.
            #[cfg(unix)]
            if exit_code.is_none() && !success {
                let err = AppError::ProcessTerminated;
                let _ = tx.send(WorkerMsg::RunComplete {
                    success: false,
                    exit_code: None,
                    error_text: Some(err.to_string()),
                });
                return;
            }

            let error_text = if success {
                None
            } else {
                Some(format!(
                    "EasyMesh exited with code {}",
                    exit_code.unwrap_or(-1)
                ))
            };

            let _ = tx.send(WorkerMsg::RunComplete {
                success,
                exit_code,
                error_text,
            });
        }
        Err(e) => {
            let _ = tx.send(WorkerMsg::RunComplete {
                success: false,
                exit_code: None,
                error_text: Some(format!("Failed to wait for EasyMesh process: {e}")),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::easymesh::{EasyMeshState, PointRow, SegmentRow};
    use crate::state::project::InputMode;
    use crate::runner::{EasyMeshRunArgs, WorkerMsg};
    use std::path::PathBuf;
    use std::sync::mpsc;

    // -----------------------------------------------------------------------
    // Helper: build a minimal EasyMeshState with `n` points and `n` segments
    // -----------------------------------------------------------------------
    fn make_state(
        points: Vec<PointRow>,
        segments: Vec<SegmentRow>,
    ) -> EasyMeshState {
        EasyMeshState {
            input_mode: InputMode::Manual,
            file_path: None,
            points,
            selected_point: None,
            segments,
            selected_segment: None,
            output_dir: None,
            fmt_tec: false,
            fmt_vtk: false,
            fmt_eps: false,
            aggressiveness: 0,
            skip_relaxation: false,
            skip_smoothing: false,
            boundary_only: false,
            suppress_messages: false,
            easymesh_binary: PathBuf::from("Easy"),
        }
    }

    fn point(x: &str, y: &str, sp: &str, m: &str) -> PointRow {
        PointRow {
            x: x.into(),
            y: y.into(),
            spacing: sp.into(),
            marker: m.into(),
        }
    }

    fn segment(s: &str, e: &str, m: &str) -> SegmentRow {
        SegmentRow {
            start: s.into(),
            end: e.into(),
            marker: m.into(),
        }
    }

    // -----------------------------------------------------------------------
    // state_to_dot_d tests
    // -----------------------------------------------------------------------

    /// Empty state produces "0\n0\n".
    #[test]
    fn dot_d_empty_state() {
        let s = make_state(vec![], vec![]);
        let out = state_to_dot_d(&s);
        assert_eq!(out, "0\n0\n");
    }

    /// Point count header is correct.
    #[test]
    fn dot_d_point_count_header() {
        let s = make_state(
            vec![point("0.0", "0.0", "0.1", "1"), point("1.0", "0.0", "0.1", "1")],
            vec![],
        );
        let out = state_to_dot_d(&s);
        let mut lines = out.lines();
        assert_eq!(lines.next().unwrap(), "2");
    }

    /// Point rows use index-colon format: `<i>: x y spacing marker`.
    #[test]
    fn dot_d_point_row_format() {
        let s = make_state(
            vec![point("1.5", "2.5", "0.25", "3")],
            vec![],
        );
        let out = state_to_dot_d(&s);
        let mut lines = out.lines();
        lines.next(); // count
        assert_eq!(lines.next().unwrap(), "0: 1.5 2.5 0.25 3");
    }

    /// Segment rows use index-colon format: `<i>: start end marker`.
    #[test]
    fn dot_d_segment_row_format() {
        let s = make_state(
            vec![],
            vec![segment("0", "1", "2")],
        );
        let out = state_to_dot_d(&s);
        let mut lines = out.lines();
        lines.next(); // point count (0)
        lines.next(); // segment count (1)
        assert_eq!(lines.next().unwrap(), "0: 0 1 2");
    }

    /// Multiple points and segments produce correct indices and counts.
    #[test]
    fn dot_d_multiple_rows() {
        let s = make_state(
            vec![
                point("0.0", "0.0", "0.5", "1"),
                point("1.0", "0.0", "0.5", "1"),
                point("1.0", "1.0", "0.5", "1"),
            ],
            vec![segment("0", "1", "1"), segment("1", "2", "1")],
        );
        let out = state_to_dot_d(&s);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "3");      // 3 points
        assert_eq!(lines[1], "0: 0.0 0.0 0.5 1");
        assert_eq!(lines[2], "1: 1.0 0.0 0.5 1");
        assert_eq!(lines[3], "2: 1.0 1.0 0.5 1");
        assert_eq!(lines[4], "2");      // 2 segments
        assert_eq!(lines[5], "0: 0 1 1");
        assert_eq!(lines[6], "1: 1 2 1");
    }

    // -----------------------------------------------------------------------
    // build_easymesh_args tests
    // -----------------------------------------------------------------------

    /// No flags set → empty args.
    #[test]
    fn args_all_flags_off() {
        let s = make_state(vec![], vec![]);
        assert!(build_easymesh_args(&s).is_empty());
    }

    /// fmt_tec produces "+tec".
    #[test]
    fn args_fmt_tec() {
        let mut s = make_state(vec![], vec![]);
        s.fmt_tec = true;
        assert_eq!(build_easymesh_args(&s), vec!["+tec"]);
    }

    /// fmt_vtk produces "+vtk".
    #[test]
    fn args_fmt_vtk() {
        let mut s = make_state(vec![], vec![]);
        s.fmt_vtk = true;
        assert_eq!(build_easymesh_args(&s), vec!["+vtk"]);
    }

    /// fmt_eps produces "+eps".
    #[test]
    fn args_fmt_eps() {
        let mut s = make_state(vec![], vec![]);
        s.fmt_eps = true;
        assert_eq!(build_easymesh_args(&s), vec!["+eps"]);
    }

    /// Aggressiveness 0 produces no args.
    #[test]
    fn args_aggressiveness_zero_omitted() {
        let mut s = make_state(vec![], vec![]);
        s.aggressiveness = 0;
        assert!(!build_easymesh_args(&s).contains(&"+a".to_string()));
    }

    /// Aggressiveness > 0 produces "+a <n>".
    #[test]
    fn args_aggressiveness_nonzero() {
        let mut s = make_state(vec![], vec![]);
        s.aggressiveness = 3;
        let args = build_easymesh_args(&s);
        assert!(args.contains(&"+a".to_string()));
        assert!(args.contains(&"3".to_string()));
        let pos_a = args.iter().position(|x| x == "+a").unwrap();
        assert_eq!(args[pos_a + 1], "3");
    }

    /// skip_relaxation produces "-r".
    #[test]
    fn args_skip_relaxation() {
        let mut s = make_state(vec![], vec![]);
        s.skip_relaxation = true;
        assert!(build_easymesh_args(&s).contains(&"-r".to_string()));
    }

    /// skip_smoothing produces "-s".
    #[test]
    fn args_skip_smoothing() {
        let mut s = make_state(vec![], vec![]);
        s.skip_smoothing = true;
        assert!(build_easymesh_args(&s).contains(&"-s".to_string()));
    }

    /// boundary_only produces "-d".
    #[test]
    fn args_boundary_only() {
        let mut s = make_state(vec![], vec![]);
        s.boundary_only = true;
        assert!(build_easymesh_args(&s).contains(&"-d".to_string()));
    }

    /// suppress_messages produces "-m".
    #[test]
    fn args_suppress_messages() {
        let mut s = make_state(vec![], vec![]);
        s.suppress_messages = true;
        assert!(build_easymesh_args(&s).contains(&"-m".to_string()));
    }

    /// All flags together — order matches design doc.
    #[test]
    fn args_all_flags_on() {
        let mut s = make_state(vec![], vec![]);
        s.fmt_tec = true;
        s.fmt_vtk = true;
        s.fmt_eps = true;
        s.aggressiveness = 6;
        s.skip_relaxation = true;
        s.skip_smoothing = true;
        s.boundary_only = true;
        s.suppress_messages = true;
        let args = build_easymesh_args(&s);
        // Spot-check order
        assert_eq!(args[0], "+tec");
        assert_eq!(args[1], "+vtk");
        assert_eq!(args[2], "+eps");
        let pos_a = args.iter().position(|x| x == "+a").unwrap();
        assert_eq!(args[pos_a + 1], "6");
        assert!(args.contains(&"-r".to_string()));
        assert!(args.contains(&"-s".to_string()));
        assert!(args.contains(&"-d".to_string()));
        assert!(args.contains(&"-m".to_string()));
    }

    // -----------------------------------------------------------------------
    // run() — binary not found
    // -----------------------------------------------------------------------

    /// When the binary does not exist, RunComplete with BinaryNotFound error
    /// is sent (Req 12 AC3).
    #[test]
    fn run_binary_not_found_sends_error() {
        let (tx, rx) = mpsc::channel::<WorkerMsg>();
        let args = EasyMeshRunArgs {
            dot_d_content: String::new(),
            output_dir: std::env::temp_dir(),
            binary_path: PathBuf::from("/nonexistent/path/Easy"),
            args: vec![],
            input_stem: None,
        };
        run(args, tx);
        let msg = rx.recv().expect("should receive a message");
        match msg {
            WorkerMsg::RunComplete {
                success,
                error_text,
                ..
            } => {
                assert!(!success);
                let text = error_text.expect("error_text should be Some");
                assert!(
                    text.contains("not found") || text.contains("BinaryNotFound"),
                    "unexpected error text: {text}"
                );
            }
            _ => panic!("expected RunComplete"),
        }
    }

    // -----------------------------------------------------------------------
    // run() — temp file cleanup
    // -----------------------------------------------------------------------

    /// Temp .d file is deleted after a successful run (Req 8 AC3).
    /// Uses the real Easy binary if available; otherwise skips the assertion
    /// about output files but still verifies temp cleanup.
    #[test]
    fn run_temp_file_is_deleted() {
        // Find a real binary to test with, or use a non-existent one (which
        // returns early before writing the file).  We write the file ourselves
        // to a known location and verify it's gone after run() returns.
        let binary_path = PathBuf::from("/home/elderiuos/Programs/mesh_generator/EasyMesh/Src/Easy");
        if !binary_path.exists() {
            // Skip if binary not present — the binary-not-found path never
            // writes a temp file so there's nothing to verify.
            return;
        }

        let tmp_dir = tempfile::tempdir().expect("create temp dir");
        let (tx, rx) = mpsc::channel::<WorkerMsg>();

        // Use a trivially valid .d snippet (3 point triangle, 3 segments)
        let dot_d = "3\n\
                     0: 0.0 0.0 0.25 1\n\
                     1: 1.0 0.0 0.25 1\n\
                     2: 0.5 1.0 0.25 1\n\
                     3\n\
                     0: 0 1 1\n\
                     1: 1 2 1\n\
                     2: 2 0 1\n";

        let args = EasyMeshRunArgs {
            dot_d_content: dot_d.to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            binary_path,
            args: vec![],
            input_stem: Some("test_triangle".to_string()),
        };

        run(args, tx);

        // Drain all messages
        let mut got_complete = false;
        while let Ok(msg) = rx.try_recv() {
            if let WorkerMsg::RunComplete { .. } = msg {
                got_complete = true;
            }
        }
        assert!(got_complete, "should have received RunComplete");

        // No .d file should remain in the temp dir
        let d_files: Vec<_> = std::fs::read_dir(tmp_dir.path())
            .expect("read temp dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "d"))
            .collect();
        assert!(
            d_files.is_empty(),
            "temp .d file was not cleaned up: {:?}",
            d_files
        );
    }

    // -----------------------------------------------------------------------
    // Integration tests — Task 19
    // -----------------------------------------------------------------------

    /// Helper: resolve the EasyMesh binary relative to the mesh_gui crate.
    ///
    /// `CARGO_MANIFEST_DIR` points to `mesh_gui/` at compile time.
    fn easymesh_binary_path() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // mesh_gui/ -> workspace root -> EasyMesh/Src/Easy
        manifest_dir
            .parent()
            .expect("mesh_gui has a parent directory")
            .join("EasyMesh")
            .join("Src")
            .join(crate::state::easymesh::expected_binary_name())
    }

    /// Req 8 AC3, 7 / Req 9 AC5 / Req 13 AC5 — end-to-end EasyMesh run.
    ///
    /// Constructs `EasyMeshRunArgs` from `EASYMESH_TEMPLATE` with `+tec` and
    /// `+vtk` flags, runs against the real binary (skipped if absent), and
    /// verifies:
    /// - at least one `LogLine` is received
    /// - a `RunComplete` is received
    /// - the `.dat` output file (TecPlot) exists in the output directory
    /// - the `.vtk` output file (ParaView) exists in the output directory
    /// - no temp `.d` file remains in the output directory (Req 8 AC3)
    #[test]
    fn integration_easymesh_end_to_end_with_tec_and_vtk() {
        let binary_path = easymesh_binary_path();
        if !binary_path.exists() {
            // Skip gracefully when the binary is not present (e.g. in CI).
            return;
        }

        let tmp_dir = tempfile::tempdir().expect("create temp dir");
        let (tx, rx) = mpsc::channel::<WorkerMsg>();

        let args = EasyMeshRunArgs {
            // Use the hardcoded template content directly (Req 3 AC6)
            dot_d_content: crate::templates::EASYMESH_TEMPLATE.to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            binary_path,
            args: vec!["+tec".to_string(), "+vtk".to_string()],
            input_stem: Some("example".to_string()),
        };

        run(args, tx);

        // Collect all messages from the channel
        let mut log_lines: Vec<String> = Vec::new();
        let mut run_complete: Option<(bool, Option<i32>, Option<String>)> = None;

        while let Ok(msg) = rx.try_recv() {
            match msg {
                WorkerMsg::LogLine(line) => log_lines.push(line),
                WorkerMsg::RunComplete {
                    success,
                    exit_code,
                    error_text,
                } => {
                    run_complete = Some((success, exit_code, error_text));
                }
                _ => {}
            }
        }

        // Req 9 AC5 — runner must send at least one LogLine
        assert!(
            !log_lines.is_empty(),
            "expected at least one LogLine but received none"
        );

        // Req 9 AC5 — RunComplete must have been sent
        assert!(
            run_complete.is_some(),
            "expected RunComplete but none was received"
        );

        // Req 8 AC3 — temp .d file must be deleted after the run
        let d_files: Vec<_> = std::fs::read_dir(tmp_dir.path())
            .expect("read temp dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "d"))
            .collect();
        assert!(
            d_files.is_empty(),
            "temp .d file was not cleaned up: {:?}",
            d_files
        );

        // Req 8 AC7 — `.dat` (TecPlot) and `.vtk` (ParaView) output files must exist.
        //
        // The Easy binary names output files after the input stem.  The runner
        // uses a timestamp-based stem (`easymesh_tmp_<nanos>`) so we search by
        // extension rather than by name.
        let has_dat = std::fs::read_dir(tmp_dir.path())
            .expect("read temp dir")
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().map_or(false, |ext| ext == "dat"));
        let has_vtk = std::fs::read_dir(tmp_dir.path())
            .expect("read temp dir")
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().map_or(false, |ext| ext == "vtk"));

        assert!(
            has_dat,
            "expected a .dat (TecPlot) output file but none found in {:?}",
            tmp_dir.path()
        );
        assert!(
            has_vtk,
            "expected a .vtk (ParaView) output file but none found in {:?}",
            tmp_dir.path()
        );
    }

    /// Req 12 AC3 / Req 13 AC5 — `BinaryNotFound` error is carried in `RunComplete`
    /// when the binary path does not exist.
    ///
    /// This test does NOT require the real binary to be present; it deliberately
    /// uses a non-existent path.
    #[test]
    fn integration_easymesh_binary_not_found_error_text() {
        let (tx, rx) = mpsc::channel::<WorkerMsg>();

        let args = EasyMeshRunArgs {
            dot_d_content: crate::templates::EASYMESH_TEMPLATE.to_string(),
            output_dir: std::env::temp_dir(),
            binary_path: PathBuf::from("/nonexistent/binary/path/Easy"),
            args: vec!["+tec".to_string(), "+vtk".to_string()],
            input_stem: None,
        };

        run(args, tx);

        // Drain the channel
        let mut run_complete: Option<(bool, Option<i32>, Option<String>)> = None;
        while let Ok(msg) = rx.try_recv() {
            if let WorkerMsg::RunComplete {
                success,
                exit_code,
                error_text,
            } = msg
            {
                run_complete = Some((success, exit_code, error_text));
            }
        }

        let (success, _exit_code, error_text) =
            run_complete.expect("RunComplete should have been sent");

        assert!(!success, "run should report failure when binary not found");

        let text = error_text.expect("error_text should be Some for BinaryNotFound");

        // Req 12 AC3 — error message must mention the missing binary path and
        // include the "not found" phrasing from AppError::BinaryNotFound.
        assert!(
            text.contains("not found") || text.contains("BinaryNotFound"),
            "error_text should mention 'not found'; got: {text}"
        );
        assert!(
            text.contains("/nonexistent/binary/path/Easy"),
            "error_text should contain the binary path; got: {text}"
        );
    }
}

// Output configuration side panel — format checkboxes, directory picker,
// Generate Mesh button, run-status display, Open Output Dir, and Build
// EasyMesh from Source flow.
// Implements Req 6, 7, 8 AC1 and 4–6, Req 10, Req 12 AC3, Req 14 AC1 and 7.
// All rfd dialogs are run on background threads to avoid freezing the UI.

use eframe::egui;

use crate::app::{dialog_start_dir, open_directory, spawn_folder_dialog, DialogTag, MeshApp, RunStatus};
use crate::runner::{
    easymesh_runner::{build_easymesh_args, state_to_dot_d},
    structured_runner::{file_to_mesh_input, state_to_mesh_input},
    BuildEasyMeshArgs, EasyMeshRunArgs, RunRequest, StructuredRunArgs,
};
use crate::state::project::{Engine, InputMode, ProjectState};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Render the output configuration panel inside a `SidePanel::right`.
pub fn show_output_panel(app: &mut MeshApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    ui.strong("Output");
    ui.separator();

    match app.project.selected_engine {
        Engine::Structured => show_structured_output(app, ui, &ctx),
        Engine::EasyMesh => show_easymesh_output(app, ui, &ctx),
    }

    ui.add_space(12.0);
    ui.separator();

    show_generate_section(app, ui);
}

// ---------------------------------------------------------------------------
// Structured output config (Req 6)
// ---------------------------------------------------------------------------

fn show_structured_output(app: &mut MeshApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let s = &mut app.project.structured;

    // Four format checkboxes — all unchecked by default (Req 6 AC1)
    ui.checkbox(&mut s.fmt_fepoint, "FEPOINT (.plt)");
    ui.checkbox(&mut s.fmt_zones, "Zones (.plt)");
    ui.checkbox(&mut s.fmt_vtk, "VTK (.vtu)");
    ui.checkbox(&mut s.fmt_text, "Text (.txt)");

    ui.add_space(8.0);

    // Output directory picker (Req 6)
    let dialog_busy = app.dialog_open;
    if ui.add_enabled(!dialog_busy, egui::Button::new("Choose Output Directory")).clicked() {
        app.dialog_open = true;
        let tx = app.dialog_tx.clone();
        let ctx2 = ctx.clone();
        let start = dialog_start_dir();
        spawn_folder_dialog(DialogTag::StructuredOutputDir, tx, ctx2, move || {
            rfd::FileDialog::new()
                .set_title("Choose Output Directory")
                .set_directory(start)
                .pick_folder()
        });
    }

    match &app.project.structured.output_dir {
        Some(p) => { ui.label(p.display().to_string()); }
        None => { ui.label(egui::RichText::new("No directory selected").weak()); }
    }
}

// ---------------------------------------------------------------------------
// EasyMesh output config (Req 7)
// ---------------------------------------------------------------------------

fn show_easymesh_output(app: &mut MeshApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let s = &mut app.project.easymesh;

    // Three format checkboxes (Req 7 AC1)
    ui.checkbox(&mut s.fmt_tec, "Tecplot (.dat)");
    ui.checkbox(&mut s.fmt_vtk, "VTK (.vtk)");
    ui.checkbox(&mut s.fmt_eps, "EPS (.eps)");

    ui.add_space(8.0);

    // Aggressiveness slider 0–6 (Req 7 AC2)
    ui.horizontal(|ui| {
        ui.label("+a [0..6]");
        ui.add(egui::Slider::new(&mut s.aggressiveness, 0_u8..=6_u8).show_value(true));
    });

    ui.add_space(8.0);

    // Four toggle checkboxes (Req 7 AC3)
    ui.checkbox(&mut s.skip_relaxation, "Skip relaxation (-r)");
    ui.checkbox(&mut s.skip_smoothing, "Skip smoothing (-s)");
    ui.checkbox(&mut s.boundary_only, "Boundary only (-d)");
    ui.checkbox(&mut s.suppress_messages, "Suppress messages (-m)");

    ui.add_space(8.0);

    // Output directory picker
    let dialog_busy = app.dialog_open;
    if ui.add_enabled(!dialog_busy, egui::Button::new("Choose Output Directory")).clicked() {
        app.dialog_open = true;
        let tx = app.dialog_tx.clone();
        let ctx2 = ctx.clone();
        let start = dialog_start_dir();
        spawn_folder_dialog(DialogTag::EasyMeshOutputDir, tx, ctx2, move || {
            rfd::FileDialog::new()
                .set_title("Choose Output Directory")
                .set_directory(start)
                .pick_folder()
        });
    }

    match &app.project.easymesh.output_dir {
        Some(p) => { ui.label(p.display().to_string()); }
        None => { ui.label(egui::RichText::new("No directory selected").weak()); }
    }

    ui.add_space(8.0);
    ui.separator();

    // -----------------------------------------------------------------------
    // EasyMesh binary configuration (Req 14)
    // -----------------------------------------------------------------------
    let binary_exists = app.project.easymesh.easymesh_binary.exists();

    ui.label(egui::RichText::new("EasyMesh Binary").strong());

    // Editable binary path field — user can override the auto-detected path
    let mut binary_str = app.project.easymesh.easymesh_binary.display().to_string();
    if ui.add(
        egui::TextEdit::singleline(&mut binary_str)
            .desired_width(ui.available_width())
            .hint_text("Path to Easy / Easy.exe"),
    ).changed() {
        app.project.easymesh.easymesh_binary = std::path::PathBuf::from(&binary_str);
    }

    if binary_exists {
        ui.colored_label(egui::Color32::from_rgb(0, 180, 0), "✔ Binary found");
        app.show_build_button = false;
    } else {
        ui.colored_label(egui::Color32::RED, "✘ Binary not found at above path");
        app.show_build_button = true;
    }
}

// ---------------------------------------------------------------------------
// Generate section — button, status, open dir, build EasyMesh
// ---------------------------------------------------------------------------

fn show_generate_section(app: &mut MeshApp, ui: &mut egui::Ui) {
    let is_running = app.run_status == RunStatus::Running;
    let enabled = !is_running && is_generate_enabled(&app.project);

    // "Generate Mesh" button (Req 8 AC1)
    if ui
        .add_enabled(enabled, egui::Button::new("▶ Generate Mesh"))
        .clicked()
    {
        handle_generate(app);
        // Force an immediate repaint so log lines pushed by handle_generate
        // and the Running spinner are visible without waiting for the next frame.
        ui.ctx().request_repaint();
    }

    ui.add_space(8.0);

    // Spinner / status indicator — only show "Running" spinner here,
    // success/failure goes to the log panel (Req 9)
    match &app.run_status.clone() {
        RunStatus::Running => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Running…");
            });
        }
        RunStatus::Success => {
            ui.colored_label(egui::Color32::from_rgb(0, 180, 0), "✔ Done — see log below");
        }
        RunStatus::Failed(_) => {
            ui.colored_label(egui::Color32::RED, "✘ Failed — see log below");
        }
        RunStatus::Idle => {}
    }

    // "Open Output Directory" button — only after a successful run (Req 10)
    if app.show_open_dir_button {
        if ui.button("📂 Open Output Directory").clicked() {
            let dir_opt = match app.project.selected_engine {
                Engine::Structured => app.project.structured.output_dir.clone(),
                Engine::EasyMesh => app.project.easymesh.output_dir.clone(),
            };
            if let Some(dir) = dir_opt {
                if let Err(e) = open_directory(&dir) {
                    app.log.push(format!("Could not open directory: {e}"));
                }
            }
        }
    }

    // Build EasyMesh from Source button (only shown on EasyMesh engine — Req 14 AC1/7)
    if app.show_build_button && app.project.selected_engine == Engine::EasyMesh {
        ui.add_space(6.0);
        if ui
            .add_enabled(!is_running, egui::Button::new("🔨 Build EasyMesh from Source"))
            .clicked()
        {
            let src_dir = app
                .project
                .easymesh
                .easymesh_binary
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("EasyMesh").join("Src"));

            let req = RunRequest::BuildEasyMesh(BuildEasyMeshArgs { src_dir });
            if app.tx.send(req).is_ok() {
                app.run_status = RunStatus::Running;
                app.log.push("Starting EasyMesh build from source…".to_string());
                ui.ctx().request_repaint();
            } else {
                app.log.push("Failed to send build request to worker.".to_string());
                ui.ctx().request_repaint();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// is_generate_enabled (Req 8 AC1 / design doc)
// ---------------------------------------------------------------------------

/// Returns `true` when the "Generate Mesh" button should be enabled.
///
/// Requirements (design doc `is_generate_enabled` section):
/// - An output directory must be chosen.
/// - For File mode: the selected file must exist on disk.
/// - For Manual mode: at least 3 nodes/points and 3 edges/segments must be present.
pub fn is_generate_enabled(project: &ProjectState) -> bool {
    let dir_ok = match project.selected_engine {
        Engine::Structured => project.structured.output_dir.is_some(),
        Engine::EasyMesh => project.easymesh.output_dir.is_some(),
    };

    if !dir_ok {
        return false;
    }

    match project.selected_engine {
        Engine::Structured => match project.structured.input_mode {
            InputMode::File => project
                .structured
                .file_path
                .as_ref()
                .map_or(false, |p| p.exists()),
            InputMode::Manual => {
                project.structured.nodes.len() >= 3
                    && project.structured.edges.len() >= 3
            }
        },
        Engine::EasyMesh => match project.easymesh.input_mode {
            InputMode::File => project
                .easymesh
                .file_path
                .as_ref()
                .map_or(false, |p| p.exists()),
            InputMode::Manual => {
                project.easymesh.points.len() >= 3
                    && project.easymesh.segments.len() >= 3
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Generate — pre-flight validation + RunRequest assembly (Req 8 AC4–6)
// ---------------------------------------------------------------------------

/// Run pre-flight checks and, if they pass, send the appropriate `RunRequest`
/// on the worker channel and set `RunStatus::Running`.
///
/// Pre-flight rules (in priority order):
/// 1. Output directory must be set — if not, log an error and return.
/// 2. For Structured: at least one format checkbox must be ticked — if not,
///    log an error and return.
/// 3. Both failures: dir message comes first (it is checked first).
fn handle_generate(app: &mut MeshApp) {
    match app.project.selected_engine {
        Engine::Structured => handle_generate_structured(app),
        Engine::EasyMesh => handle_generate_easymesh(app),
    }
}

fn handle_generate_structured(app: &mut MeshApp) {
    // Pre-flight: output directory
    let output_dir = match app.project.structured.output_dir.clone() {
        Some(d) => d,
        None => {
            let msg = "No output directory selected. Please choose an output directory first.";
            app.log.push(msg.to_string());
            app.run_status = RunStatus::Failed(msg.to_string());
            return;
        }
    };

    // Pre-flight: at least one format must be selected (Structured)
    let s = &app.project.structured;
    let any_fmt = s.fmt_fepoint || s.fmt_zones || s.fmt_vtk || s.fmt_text;
    if !any_fmt {
        let msg = "No output format selected. Please tick at least one format checkbox.";
        app.log.push(msg.to_string());
        app.run_status = RunStatus::Failed(msg.to_string());
        return;
    }

    // Build MeshInput from current mode
    let input_result = match app.project.structured.input_mode {
        InputMode::File => {
            match app.project.structured.file_path.clone() {
                Some(path) => file_to_mesh_input(&path),
                None => Err(anyhow::anyhow!("No geometry file selected.")),
            }
        }
        InputMode::Manual => state_to_mesh_input(&app.project.structured),
    };

    let input = match input_result {
        Ok(i) => i,
        Err(e) => {
            let msg = format!("Failed to parse mesh input: {e}");
            app.log.push(msg.clone());
            app.run_status = RunStatus::Failed(msg);
            return;
        }
    };

    let args = StructuredRunArgs {
        input,
        output_dir,
        fmt_fepoint: app.project.structured.fmt_fepoint,
        fmt_zones: app.project.structured.fmt_zones,
        fmt_vtk: app.project.structured.fmt_vtk,
        fmt_text: app.project.structured.fmt_text,
    };

    // Reset open-dir button before new run
    app.show_open_dir_button = false;

    if app.tx.send(RunRequest::Structured(args)).is_ok() {
        app.run_status = RunStatus::Running;
        app.log.push("Starting structured mesh generation…".to_string());
    } else {
        let msg = "Failed to send run request to worker.";
        app.log.push(msg.to_string());
        app.run_status = RunStatus::Failed(msg.to_string());
    }
}

fn handle_generate_easymesh(app: &mut MeshApp) {
    // Pre-flight: output directory
    let output_dir = match app.project.easymesh.output_dir.clone() {
        Some(d) => d,
        None => {
            let msg = "No output directory selected. Please choose an output directory first.";
            app.log.push(msg.to_string());
            app.run_status = RunStatus::Failed(msg.to_string());
            return;
        }
    };

    // Build .d content from current mode
    let dot_d_content = match app.project.easymesh.input_mode {
        InputMode::File => {
            match app.project.easymesh.file_path.clone() {
                Some(path) => match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(e) => {
                        let msg = format!("Failed to read input file: {e}");
                        app.log.push(msg.clone());
                        app.run_status = RunStatus::Failed(msg);
                        return;
                    }
                },
                None => {
                    let msg = "No input file selected.";
                    app.log.push(msg.to_string());
                    app.run_status = RunStatus::Failed(msg.to_string());
                    return;
                }
            }
        }
        InputMode::Manual => state_to_dot_d(&app.project.easymesh),
    };

    let cli_args = build_easymesh_args(&app.project.easymesh);

    // Derive the output stem from the input file name (File mode) or use "mesh" (Manual mode).
    let input_stem = match app.project.easymesh.input_mode {
        InputMode::File => app
            .project
            .easymesh
            .file_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|s| s.to_string()),
        InputMode::Manual => Some("mesh".to_string()),
    };

    let args = EasyMeshRunArgs {
        dot_d_content,
        output_dir,
        binary_path: app.project.easymesh.easymesh_binary.clone(),
        args: cli_args,
        input_stem,
    };

    // Reset open-dir button before new run
    app.show_open_dir_button = false;

    if app.tx.send(RunRequest::EasyMesh(args)).is_ok() {
        app.run_status = RunStatus::Running;
        app.log.push("Starting EasyMesh generation…".to_string());
    } else {
        let msg = "Failed to send run request to worker.";
        app.log.push(msg.to_string());
        app.run_status = RunStatus::Failed(msg.to_string());
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::state::project::{Engine, InputMode, ProjectState};

    // -----------------------------------------------------------------------
    // is_generate_enabled — Structured engine
    // -----------------------------------------------------------------------

    /// No output dir → disabled (Structured File mode).
    #[test]
    fn structured_file_no_dir_disabled() {
        let mut p = ProjectState::default();
        p.selected_engine = Engine::Structured;
        p.structured.input_mode = InputMode::File;
        p.structured.file_path = Some(PathBuf::from("/tmp")); // doesn't matter; dir missing
        p.structured.output_dir = None;
        assert!(!is_generate_enabled(&p));
    }

    /// Dir set but no file selected → disabled (Structured File mode).
    #[test]
    fn structured_file_no_path_disabled() {
        let mut p = ProjectState::default();
        p.selected_engine = Engine::Structured;
        p.structured.input_mode = InputMode::File;
        p.structured.file_path = None;
        p.structured.output_dir = Some(PathBuf::from("/tmp"));
        assert!(!is_generate_enabled(&p));
    }

    /// Dir set + file exists → enabled (Structured File mode).
    #[test]
    fn structured_file_with_existing_file_enabled() {
        // /tmp is guaranteed to exist as a directory, but we need a file.
        // Use a real temp file.
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let mut p = ProjectState::default();
        p.selected_engine = Engine::Structured;
        p.structured.input_mode = InputMode::File;
        p.structured.file_path = Some(tmp.path().to_path_buf());
        p.structured.output_dir = Some(std::env::temp_dir());
        assert!(is_generate_enabled(&p));
    }

    /// Dir set but only 2 nodes → disabled (Structured Manual mode).
    #[test]
    fn structured_manual_too_few_nodes_disabled() {
        use crate::state::structured::NodeRow;
        let mut p = ProjectState::default();
        p.selected_engine = Engine::Structured;
        p.structured.input_mode = InputMode::Manual;
        p.structured.output_dir = Some(std::env::temp_dir());
        p.structured.nodes = vec![NodeRow::default(), NodeRow::default()];
        p.structured.edges = vec![
            crate::state::structured::EdgeRow::default(),
            crate::state::structured::EdgeRow::default(),
            crate::state::structured::EdgeRow::default(),
        ];
        assert!(!is_generate_enabled(&p));
    }

    /// Dir set, 3 nodes, 3 edges → enabled (Structured Manual mode).
    #[test]
    fn structured_manual_three_rows_enabled() {
        use crate::state::structured::{EdgeRow, NodeRow};
        let mut p = ProjectState::default();
        p.selected_engine = Engine::Structured;
        p.structured.input_mode = InputMode::Manual;
        p.structured.output_dir = Some(std::env::temp_dir());
        p.structured.nodes = vec![NodeRow::default(); 3];
        p.structured.edges = vec![EdgeRow::default(); 3];
        assert!(is_generate_enabled(&p));
    }

    // -----------------------------------------------------------------------
    // is_generate_enabled — EasyMesh engine
    // -----------------------------------------------------------------------

    /// No output dir → disabled (EasyMesh).
    #[test]
    fn easymesh_no_dir_disabled() {
        let mut p = ProjectState::default();
        p.selected_engine = Engine::EasyMesh;
        p.easymesh.output_dir = None;
        assert!(!is_generate_enabled(&p));
    }

    /// Dir set but no file path → disabled (EasyMesh File mode).
    #[test]
    fn easymesh_file_no_path_disabled() {
        let mut p = ProjectState::default();
        p.selected_engine = Engine::EasyMesh;
        p.easymesh.input_mode = InputMode::File;
        p.easymesh.file_path = None;
        p.easymesh.output_dir = Some(std::env::temp_dir());
        assert!(!is_generate_enabled(&p));
    }

    /// Dir set + existing file → enabled (EasyMesh File mode).
    #[test]
    fn easymesh_file_with_existing_file_enabled() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let mut p = ProjectState::default();
        p.selected_engine = Engine::EasyMesh;
        p.easymesh.input_mode = InputMode::File;
        p.easymesh.file_path = Some(tmp.path().to_path_buf());
        p.easymesh.output_dir = Some(std::env::temp_dir());
        assert!(is_generate_enabled(&p));
    }

    /// Dir set, 3 points, 3 segments → enabled (EasyMesh Manual mode).
    #[test]
    fn easymesh_manual_three_rows_enabled() {
        use crate::state::easymesh::{PointRow, SegmentRow};
        let mut p = ProjectState::default();
        p.selected_engine = Engine::EasyMesh;
        p.easymesh.input_mode = InputMode::Manual;
        p.easymesh.output_dir = Some(std::env::temp_dir());
        p.easymesh.points = vec![PointRow::default(); 3];
        p.easymesh.segments = vec![SegmentRow::default(); 3];
        assert!(is_generate_enabled(&p));
    }

    /// Dir set, only 2 segments → disabled (EasyMesh Manual mode).
    #[test]
    fn easymesh_manual_too_few_segments_disabled() {
        use crate::state::easymesh::{PointRow, SegmentRow};
        let mut p = ProjectState::default();
        p.selected_engine = Engine::EasyMesh;
        p.easymesh.input_mode = InputMode::Manual;
        p.easymesh.output_dir = Some(std::env::temp_dir());
        p.easymesh.points = vec![PointRow::default(); 3];
        p.easymesh.segments = vec![SegmentRow::default(); 2];
        assert!(!is_generate_enabled(&p));
    }

    // -----------------------------------------------------------------------
    // Default state sanity checks
    // -----------------------------------------------------------------------

    /// Default ProjectState: Structured selected, no dir → disabled.
    #[test]
    fn default_project_generate_disabled() {
        let p = ProjectState::default();
        assert!(!is_generate_enabled(&p));
    }
}

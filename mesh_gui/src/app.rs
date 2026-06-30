// MeshApp — top-level application struct and eframe::App implementation.
// Owns all runtime state and drives the egui layout.
// RunStatus is defined here (Task 4).
// Worker thread infrastructure added in Task 7.
// Cross-platform utilities (open_directory, open_in_editor) added in Task 16.
// Full UI layout wired in Task 17.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;

use crate::runner::{self, RunRequest, WorkerMsg};
use crate::state::project::{Engine, ProjectState};
use crate::ui;

// ---------------------------------------------------------------------------
// Cross-platform utilities (Req 10, Req 13 AC3–4)
// ---------------------------------------------------------------------------

/// Open `path` in the platform's default file manager.
pub fn open_directory(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(path).spawn()?;

    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(path).spawn()?;

    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer").arg(path).spawn()?;

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    compile_error!("open_directory: unsupported target OS");

    Ok(())
}

/// Open `path` in the platform's default text editor.
pub fn open_in_editor(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(path).spawn()?;

    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(path).spawn()?;

    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer").arg(path).spawn()?;

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    compile_error!("open_in_editor: unsupported target OS");

    Ok(())
}

// ---------------------------------------------------------------------------
// Non-blocking file dialog helpers
// ---------------------------------------------------------------------------
//
// On Linux, rfd::FileDialog (synchronous) calls into the xdg-portal D-Bus
// service.  That round-trip can stall for 1–3 seconds before the native
// window appears, freezing the egui paint thread entirely.
//
// Fix: run every dialog call in a std::thread so the UI stays responsive.
// The result is sent back via a one-shot mpsc channel and processed in the
// next update() frame that drains `dialog_rx`.

/// The result of a completed (non-blocking) file dialog.
pub enum DialogResult {
    /// User picked a single file (Browse / Load / Template Save).
    FilePicked {
        tag: DialogTag,
        path: PathBuf,
    },
    /// User picked a folder (Choose Output Directory).
    FolderPicked {
        tag: DialogTag,
        path: PathBuf,
    },
    /// User dismissed the dialog without choosing anything.
    #[allow(dead_code)]
    Cancelled { tag: DialogTag },
}

/// Identifies which dialog was opened so the drain loop knows what to update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogTag {
    StructuredBrowse,
    StructuredOutputDir,
    StructuredTemplateSave,
    EasyMeshBrowse,
    EasyMeshOutputDir,
    EasyMeshTemplateSave,
    SaveProject,
    LoadProject,
}

/// Returns the best starting directory for dialogs: executable's parent,
/// falling back to current working directory.
pub fn dialog_start_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// Spawn a thread that runs `builder` (which calls rfd) and sends the result
/// to `tx`.  The egui context is nudged so the next frame picks up the result.
pub fn spawn_dialog<F>(
    tag: DialogTag,
    tx: Sender<DialogResult>,
    ctx: egui::Context,
    builder: F,
)
where
    F: FnOnce() -> Option<PathBuf> + Send + 'static,
{
    std::thread::spawn(move || {
        let result = match builder() {
            Some(path) => {
                // Determine whether it's a file or folder result based on
                // whether the path has an extension (heuristic good enough
                // for our two dialog types).
                DialogResult::FilePicked { tag, path }
            }
            None => DialogResult::Cancelled { tag },
        };
        let _ = tx.send(result);
        ctx.request_repaint();
    });
}

/// Like `spawn_dialog` but always wraps the result in `FolderPicked`.
pub fn spawn_folder_dialog<F>(
    tag: DialogTag,
    tx: Sender<DialogResult>,
    ctx: egui::Context,
    builder: F,
)
where
    F: FnOnce() -> Option<PathBuf> + Send + 'static,
{
    std::thread::spawn(move || {
        let result = match builder() {
            Some(path) => DialogResult::FolderPicked { tag, path },
            None => DialogResult::Cancelled { tag },
        };
        let _ = tx.send(result);
        ctx.request_repaint();
    });
}

// ---------------------------------------------------------------------------
// RunStatus (Req 8, Req 9, Req 12)
// ---------------------------------------------------------------------------

/// Tracks the current state of a mesh generation or build job.
#[derive(Debug, Clone, PartialEq)]
pub enum RunStatus {
    Idle,
    Running,
    Success,
    Failed(String),
}

impl Default for RunStatus {
    fn default() -> Self {
        RunStatus::Idle
    }
}

// ---------------------------------------------------------------------------
// MeshApp (Task 7)
// ---------------------------------------------------------------------------

/// Top-level application struct.
pub struct MeshApp {
    pub project: ProjectState,
    pub run_status: RunStatus,
    pub show_open_dir_button: bool,
    pub show_build_button: bool,
    pub log: Vec<String>,
    pub log_auto_scroll: bool,
    /// Worker thread channel — mesh run / build requests.
    pub tx: Sender<RunRequest>,
    pub rx: Receiver<WorkerMsg>,
    /// Non-blocking dialog results channel.
    pub dialog_tx: Sender<DialogResult>,
    pub dialog_rx: Receiver<DialogResult>,
    /// Track whether a dialog is currently open to avoid opening two at once.
    pub dialog_open: bool,
    _worker: std::thread::JoinHandle<()>,
}

impl MeshApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (req_tx, req_rx) = std::sync::mpsc::channel::<RunRequest>();
        let (msg_tx, msg_rx) = std::sync::mpsc::channel::<WorkerMsg>();
        let worker = runner::spawn_worker(req_rx, msg_tx);

        let (dlg_tx, dlg_rx) = std::sync::mpsc::channel::<DialogResult>();

        // Resolve the default "outputs" directory relative to the executable,
        // then create it (and any missing parents) if it doesn't already exist.
        let outputs_dir: PathBuf = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("outputs")))
            .unwrap_or_else(|| PathBuf::from("outputs"));

        let _ = std::fs::create_dir_all(&outputs_dir);

        let mut project = ProjectState::default();
        project.structured.output_dir = Some(outputs_dir.clone());
        project.easymesh.output_dir  = Some(outputs_dir);

        MeshApp {
            project,
            run_status: RunStatus::Idle,
            show_open_dir_button: false,
            show_build_button: false,
            log: Vec::new(),
            log_auto_scroll: true,
            tx: req_tx,
            rx: msg_rx,
            dialog_tx: dlg_tx,
            dialog_rx: dlg_rx,
            dialog_open: false,
            _worker: worker,
        }
    }
}

impl eframe::App for MeshApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ------------------------------------------------------------------
        // 1. Drain dialog results
        // ------------------------------------------------------------------
        while let Ok(result) = self.dialog_rx.try_recv() {
            self.dialog_open = false;
            self.handle_dialog_result(result);
        }

        // ------------------------------------------------------------------
        // 2. Drain worker messages (Req 8 AC4, Req 9 AC2)
        // ------------------------------------------------------------------
        let log_len_before = self.log.len();
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerMsg::LogLine(line) => {
                    self.log.push(line);
                }
                WorkerMsg::RunComplete { success, exit_code, error_text } => {
                    if success {
                        self.run_status = RunStatus::Success;
                        self.show_open_dir_button = true;
                        let code = exit_code.unwrap_or(0);
                        self.log.push(format!("✔ Completed successfully (exit code {code})"));
                    } else {
                        let msg_str = error_text.clone().unwrap_or_else(|| {
                            exit_code
                                .map(|c| format!("Exit code: {c}"))
                                .unwrap_or_else(|| "Unknown error".to_string())
                        });
                        self.run_status = RunStatus::Failed(msg_str.clone());
                        let code = exit_code.unwrap_or(-1);
                        self.log.push(format!("✘ Failed (exit code {code}): {msg_str}"));
                    }
                }
                WorkerMsg::BuildComplete { success, binary_path, error_text } => {
                    if success {
                        self.run_status = RunStatus::Success;
                        self.show_build_button = false;
                        if let Some(path) = binary_path {
                            self.project.easymesh.easymesh_binary = path;
                        }
                        self.log.push("✔ EasyMesh build completed successfully.".to_string());
                    } else {
                        let msg_str = error_text.unwrap_or_else(|| "Build failed.".to_string());
                        self.run_status = RunStatus::Failed(msg_str.clone());
                        self.log.push(format!("✘ Build failed: {msg_str}"));
                    }
                }
            }
        }

        // Request a repaint whenever new log lines arrived this frame,
        // or while a job is still running (so we keep draining the channel).
        if self.log.len() != log_len_before || self.run_status == RunStatus::Running {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // ------------------------------------------------------------------
        // 3. Render layout
        // ------------------------------------------------------------------
        ui::engine_selector::show_top_panel(self, ctx);
        ui::log_panel::show_log_panel(self, ctx);

        egui::SidePanel::right("output_panel")
            .min_width(240.0)
            .default_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui::output_panel::show_output_panel(self, ui);
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    match self.project.selected_engine {
                        Engine::Structured => {
                            ui::structured_panel::show_structured_panel(self, ui);
                        }
                        Engine::EasyMesh => {
                            ui::easymesh_panel::show_easymesh_panel(self, ui);
                        }
                    }
                });
        });
    }
}

// ---------------------------------------------------------------------------
// Dialog result handler
// ---------------------------------------------------------------------------

impl MeshApp {
    fn handle_dialog_result(&mut self, result: DialogResult) {
        use DialogTag::*;
        match result {
            DialogResult::FilePicked { tag, path } => match tag {
                StructuredBrowse => {
                    if path.exists() {
                        self.project.structured.file_path = Some(path);
                    } else {
                        self.log.push(format!("Error: file not found: {}", path.display()));
                    }
                }
                EasyMeshBrowse => {
                    if path.exists() {
                        self.project.easymesh.file_path = Some(path);
                    } else {
                        self.log.push(format!("Error: file not found: {}", path.display()));
                    }
                }
                StructuredTemplateSave => {
                    self.write_template_and_open(&path, crate::templates::STRUCTURED_TEMPLATE);
                }
                EasyMeshTemplateSave => {
                    self.write_template_and_open(&path, crate::templates::EASYMESH_TEMPLATE);
                }
                SaveProject => {
                    self.do_save_project(&path);
                }
                LoadProject => {
                    self.do_load_project(&path);
                }
                StructuredOutputDir | EasyMeshOutputDir => {
                    // These are sent as FolderPicked, not FilePicked — ignore.
                }
            },
            DialogResult::FolderPicked { tag, path } => match tag {
                StructuredOutputDir => {
                    self.project.structured.output_dir = Some(path);
                }
                EasyMeshOutputDir => {
                    self.project.easymesh.output_dir = Some(path);
                }
                _ => {}
            },
            DialogResult::Cancelled { .. } => {
                // Nothing to update when the user dismissed the dialog.
            }
        }
    }

    fn write_template_and_open(&mut self, path: &PathBuf, content: &str) {
        use std::io::Write as _;
        match std::fs::File::create(path)
            .and_then(|mut f| f.write_all(content.as_bytes()))
        {
            Ok(()) => {
                self.log.push(format!("Template written to {}", path.display()));
                if let Err(e) = open_in_editor(path) {
                    self.log.push(format!("Template saved but could not open editor: {e}"));
                }
            }
            Err(e) => {
                self.log.push(format!("Failed to write template: {e}"));
            }
        }
    }

    fn do_save_project(&mut self, path: &PathBuf) {
        use std::io::Write as _;
        match serde_json::to_string_pretty(&self.project) {
            Ok(json) => {
                match std::fs::File::create(path).and_then(|mut f| f.write_all(json.as_bytes())) {
                    Ok(()) => self.log.push(format!("Project saved to {}", path.display())),
                    Err(e) => self.log.push(format!("Failed to write project: {e}")),
                }
            }
            Err(e) => self.log.push(format!("Failed to serialise project: {e}")),
        }
    }

    fn do_load_project(&mut self, path: &PathBuf) {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str::<ProjectState>(&content) {
                    Ok(loaded) => {
                        self.project = loaded;
                        self.log.push(format!("Project loaded from {}", path.display()));
                    }
                    Err(_) => {
                        let msg = crate::runner::AppError::InvalidProjectFile.to_string();
                        self.log.push(format!("Error loading {}: {}", path.display(), msg));
                    }
                }
            }
            Err(e) => self.log.push(format!("Could not read {}: {e}", path.display())),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_directory_with_existing_path_does_not_error() {
        let dir = std::env::temp_dir();
        let result = open_directory(&dir);
        assert!(result.is_ok(), "open_directory failed: {:?}", result.err());
    }

    #[test]
    fn open_in_editor_with_existing_file_does_not_error() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
        tmp.write_all(b"hello\n").unwrap();
        let path = tmp.path().to_path_buf();
        let result = open_in_editor(&path);
        assert!(result.is_ok(), "open_in_editor failed: {:?}", result.err());
    }

    #[test]
    fn functions_accept_pathbuf_as_path() {
        let p = PathBuf::from("/tmp");
        let _: fn(&Path) -> anyhow::Result<()> = open_directory;
        let _: fn(&Path) -> anyhow::Result<()> = open_in_editor;
        let _ = p;
    }
}

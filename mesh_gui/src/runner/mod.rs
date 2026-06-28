// Runner module — re-exports, RunRequest, WorkerMsg, AppError, and worker thread.
// AppError and message/request types are defined here (Task 4).
// Worker thread infrastructure will be added in Task 7.

pub mod easymesh_builder;
pub mod easymesh_runner;
pub mod structured_runner;

use std::path::PathBuf;

use thiserror::Error;

use structured_mesh::input_parser::MeshInput;


// ---------------------------------------------------------------------------
// AppError (Req 12, Req 14)
// ---------------------------------------------------------------------------

/// Typed errors for the mesh GUI application.
///
/// These are converted to `String` via `to_string()` before being placed in
/// `WorkerMsg::RunComplete::error_text` / `WorkerMsg::BuildComplete::error_text`
/// so the UI only has to deal with plain strings.
#[derive(Debug, Error)]
pub enum AppError {
    #[error(
        "EasyMesh binary not found at {path}. \
         Please ensure the Easy executable is present."
    )]
    BinaryNotFound { path: PathBuf },

    #[error("Mesh generation process terminated unexpectedly.")]
    ProcessTerminated,

    #[error("Could not load project: the file is not a valid project file.")]
    InvalidProjectFile,

    #[error(
        "EasyMesh source directory not found at {path}. \
         Cannot build from source."
    )]
    SourceDirNotFound { path: PathBuf },

    #[error(
        "The 'make' build tool was not found. \
         Please install a C++ build environment \
         (e.g., build-essential on Linux, Xcode Command Line Tools on macOS, \
         or MinGW/WSL on Windows) and try again."
    )]
    MakeNotFound,

    #[error("Build failed. Check the log for compiler errors.")]
    BuildFailed { exit_code: i32 },
}

// ---------------------------------------------------------------------------
// Request / message types (Req 8, Req 9, Req 14)
// ---------------------------------------------------------------------------

/// Commands sent from the UI thread to the worker thread.
pub enum RunRequest {
    Structured(StructuredRunArgs),
    EasyMesh(EasyMeshRunArgs),
    BuildEasyMesh(BuildEasyMeshArgs),
}

/// Messages sent from the worker thread back to the UI thread.
pub enum WorkerMsg {
    /// A single line of stdout/stderr captured from the subprocess.
    LogLine(String),
    /// Final status of a mesh generation run (Structured or EasyMesh).
    RunComplete {
        success: bool,
        exit_code: Option<i32>,
        error_text: Option<String>,
    },
    /// Final status of an EasyMesh build-from-source operation.
    BuildComplete {
        success: bool,
        binary_path: Option<PathBuf>,
        error_text: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Argument structs
// ---------------------------------------------------------------------------

/// Arguments forwarded to `structured_runner::run`.
pub struct StructuredRunArgs {
    /// Parsed mesh input (geometry + edge table).
    pub input: MeshInput,
    /// Directory where output files will be written.
    pub output_dir: PathBuf,
    // Output format flags (Req 6)
    pub fmt_fepoint: bool,
    pub fmt_zones: bool,
    pub fmt_vtk: bool,
    pub fmt_text: bool,
}

/// Arguments forwarded to `easymesh_runner::run`.
pub struct EasyMeshRunArgs {
    /// Serialised `.d` file content (ready to write to disk).
    pub dot_d_content: String,
    /// Directory where the `.d` file and output files will be written.
    pub output_dir: PathBuf,
    /// Absolute path to the `Easy` / `Easy.exe` binary.
    pub binary_path: PathBuf,
    /// Pre-constructed CLI argument list (format flags, aggressiveness, toggles).
    pub args: Vec<String>,
    /// Stem used for the input `.d` file and thus for all output filenames.
    /// Defaults to `"mesh"` when `None` (outputs: mesh.dat, mesh.vtk, …).
    /// Set to the input file's stem when running in File mode so outputs match the input name.
    pub input_stem: Option<String>,
}

/// Arguments forwarded to `easymesh_builder::build`.
pub struct BuildEasyMeshArgs {
    /// Path to the `EasyMesh/Src/` directory containing the `Makefile`.
    pub src_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// Worker thread (Task 7)
// ---------------------------------------------------------------------------

/// Spawns the background worker thread.
///
/// The thread blocks on `rx`, dispatching each [`RunRequest`] to the
/// appropriate engine-specific runner.  It terminates naturally when the
/// sender side of the channel is dropped (i.e. when `MeshApp` is dropped at
/// shutdown).
pub fn spawn_worker(
    rx: std::sync::mpsc::Receiver<RunRequest>,
    tx: std::sync::mpsc::Sender<WorkerMsg>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for request in rx {
            match request {
                RunRequest::Structured(args) => structured_runner::run(args, tx.clone()),
                RunRequest::EasyMesh(args) => easymesh_runner::run(args, tx.clone()),
                RunRequest::BuildEasyMesh(args) => easymesh_builder::build(args, tx.clone()),
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// AppError::BinaryNotFound formats correctly.
    #[test]
    fn binary_not_found_error_message() {
        let err = AppError::BinaryNotFound {
            path: PathBuf::from("/some/path/Easy"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("/some/path/Easy"),
            "error message should contain the path: {msg}"
        );
        assert!(
            msg.contains("not found"),
            "error message should mention 'not found': {msg}"
        );
    }

    /// AppError::ProcessTerminated formats correctly.
    #[test]
    fn process_terminated_error_message() {
        let msg = AppError::ProcessTerminated.to_string();
        assert!(msg.contains("terminated"), "{msg}");
    }

    /// AppError::InvalidProjectFile formats correctly.
    #[test]
    fn invalid_project_file_error_message() {
        let msg = AppError::InvalidProjectFile.to_string();
        assert!(msg.contains("project"), "{msg}");
    }

    /// AppError::SourceDirNotFound formats correctly.
    #[test]
    fn source_dir_not_found_error_message() {
        let err = AppError::SourceDirNotFound {
            path: PathBuf::from("/missing/src"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/missing/src"), "{msg}");
        assert!(msg.contains("not found"), "{msg}");
    }

    /// AppError::MakeNotFound message mentions 'make'.
    #[test]
    fn make_not_found_error_message() {
        let msg = AppError::MakeNotFound.to_string();
        assert!(
            msg.to_lowercase().contains("make"),
            "error message should mention 'make': {msg}"
        );
    }

    /// AppError::BuildFailed formats correctly and includes exit code context.
    #[test]
    fn build_failed_error_message() {
        let msg = AppError::BuildFailed { exit_code: 2 }.to_string();
        assert!(msg.contains("Build failed"), "{msg}");
    }

    /// expected_binary_name() re-exported from runner module returns correct value.
    #[test]
    fn reexported_expected_binary_name() {
        let name = crate::state::easymesh::expected_binary_name();
        #[cfg(target_os = "windows")]
        assert_eq!(name, "Easy.exe");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, "Easy");
    }
}

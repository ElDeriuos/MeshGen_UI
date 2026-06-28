// Top panel — engine selector (Structured / EasyMesh) plus Save/Load project buttons.
// Implements Req 1 AC1–4 and Req 11.
// All rfd dialogs are run on background threads to avoid freezing the UI.

use eframe::egui;

use crate::app::{dialog_start_dir, spawn_dialog, DialogTag, MeshApp};
use crate::state::project::Engine;

/// Render the top panel: engine selector + Save / Load project.
pub fn show_top_panel(app: &mut MeshApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Engine selector (Req 1)
            let is_structured = app.project.selected_engine == Engine::Structured;
            let is_easymesh = app.project.selected_engine == Engine::EasyMesh;

            if ui.add(egui::SelectableLabel::new(is_structured, "Structured Mesh")).clicked() {
                app.project.selected_engine = Engine::Structured;
            }
            if ui.add(egui::SelectableLabel::new(is_easymesh, "Unstructured Mesh (EasyMesh)")).clicked() {
                app.project.selected_engine = Engine::EasyMesh;
            }

            ui.separator();

            // Save Project (Req 11)
            let dialog_busy = app.dialog_open;
            if ui.add_enabled(!dialog_busy, egui::Button::new("Save Project")).clicked() {
                app.dialog_open = true;
                let tx = app.dialog_tx.clone();
                let ctx2 = ctx.clone();
                let start = dialog_start_dir();
                spawn_dialog(DialogTag::SaveProject, tx, ctx2, move || {
                    rfd::FileDialog::new()
                        .set_title("Save Project")
                        .add_filter("JSON project file", &["json"])
                        .set_file_name("project.json")
                        .set_directory(start)
                        .save_file()
                });
            }

            // Load Project (Req 11)
            if ui.add_enabled(!dialog_busy, egui::Button::new("Load Project")).clicked() {
                app.dialog_open = true;
                let tx = app.dialog_tx.clone();
                let ctx2 = ctx.clone();
                let start = dialog_start_dir();
                spawn_dialog(DialogTag::LoadProject, tx, ctx2, move || {
                    rfd::FileDialog::new()
                        .set_title("Load Project")
                        .add_filter("JSON project file", &["json"])
                        .set_directory(start)
                        .pick_file()
                });
            }

            if dialog_busy {
                ui.label(egui::RichText::new("(dialog open…)").weak());
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::state::project::{Engine, ProjectState};
    use crate::runner::AppError;

    #[test]
    fn default_project_selects_structured() {
        let project = ProjectState::default();
        assert_eq!(project.selected_engine, Engine::Structured);
    }

    #[test]
    fn switching_engine_does_not_reset_other_engine_state() {
        let mut project = ProjectState::default();
        project.structured.dx = "0.05".to_string();
        project.selected_engine = Engine::EasyMesh;
        project.selected_engine = Engine::Structured;
        assert_eq!(project.structured.dx, "0.05");
    }

    #[test]
    fn project_save_load_round_trip() {
        let mut original = ProjectState::default();
        original.selected_engine = Engine::EasyMesh;
        original.structured.dx = "0.25".to_string();
        original.structured.dy = "0.25".to_string();
        let json = serde_json::to_string_pretty(&original).expect("serialise");
        let restored: ProjectState = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }

    #[test]
    fn invalid_json_fails_to_deserialise() {
        let bad_json = r#"{ "selected_engine": "Unknown", "garbage": true }"#;
        let result = serde_json::from_str::<ProjectState>(bad_json);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_project_file_error_message() {
        let msg = AppError::InvalidProjectFile.to_string();
        assert!(msg.contains("Could not load project"));
        assert!(msg.contains("valid project file"));
    }
}

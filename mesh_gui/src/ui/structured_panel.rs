// Structured Mesh input panel — File/Manual mode, geometry tables, template generation.
// Implements Req 2, 3, 4.
// All rfd dialogs are run on background threads to avoid freezing the UI.

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::app::{dialog_start_dir, spawn_dialog, DialogTag, MeshApp};
use crate::state::project::InputMode;
use crate::state::structured::{EdgeRow, NodeRow};
use crate::ui::validation::{validate_structured, FieldId, ValidationResult};

// ---------------------------------------------------------------------------
// Helper — red-border frame for a table cell
// ---------------------------------------------------------------------------

/// Returns a `Frame` with a red stroke border when `has_error` is true,
/// or a plain frame with no visible border otherwise.
fn cell_frame(has_error: bool) -> egui::Frame {
    if has_error {
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.5, egui::Color32::RED))
            .inner_margin(egui::Margin::same(1_i8))
    } else {
        egui::Frame::new().inner_margin(egui::Margin::same(1_i8))
    }
}

/// Wrap a `TextEdit` in a red-border `Frame` when `has_error` is true.
fn text_edit_with_border(
    ui: &mut egui::Ui,
    has_error: bool,
    f: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    let frame = cell_frame(has_error);
    let mut resp: Option<egui::Response> = None;
    frame.show(ui, |inner_ui| {
        resp = Some(f(inner_ui));
    });
    resp.expect("closure must add a widget")
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Render the Structured Mesh input panel inside `ui`.
pub fn show_structured_panel(app: &mut MeshApp, ui: &mut egui::Ui) {
    // -----------------------------------------------------------------------
    // Run validation once per frame (Manual mode fields)
    // -----------------------------------------------------------------------
    let validation = validate_structured(&app.project.structured);

    // -----------------------------------------------------------------------
    // Mode radio buttons
    // -----------------------------------------------------------------------
    {
        let state = &mut app.project.structured;
        ui.horizontal(|ui| {
            ui.radio_value(&mut state.input_mode, InputMode::File, "File Mode");
            ui.radio_value(&mut state.input_mode, InputMode::Manual, "Manual Mode");
        });
    }

    ui.separator();

    let ctx = ui.ctx().clone();
    match app.project.structured.input_mode.clone() {
        InputMode::File => show_file_mode(ui, app, &ctx),
        InputMode::Manual => show_manual_mode(ui, app, &validation),
    }
}

// ---------------------------------------------------------------------------
// File mode
// ---------------------------------------------------------------------------

fn show_file_mode(ui: &mut egui::Ui, app: &mut MeshApp, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        let path_text = app
            .project
            .structured
            .file_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        ui.add_enabled(
            false,
            egui::TextEdit::singleline(&mut path_text.clone())
                .desired_width(320.0)
                .hint_text("No file selected"),
        );

        let dialog_busy = app.dialog_open;
        if ui.add_enabled(!dialog_busy, egui::Button::new("Browse")).clicked() {
            app.dialog_open = true;
            let tx = app.dialog_tx.clone();
            let ctx2 = ctx.clone();
            let start = dialog_start_dir();
            spawn_dialog(DialogTag::StructuredBrowse, tx, ctx2, move || {
                rfd::FileDialog::new()
                    .set_title("Open Structured Geometry File")
                    .add_filter("Text file", &["txt"])
                    .set_directory(start)
                    .pick_file()
            });
        }
    });

    if let Some(p) = app.project.structured.file_path.clone() {
        if !p.exists() {
            ui.colored_label(egui::Color32::RED, format!("File not found: {}", p.display()));
        }
    }

    ui.add_space(8.0);

    let dialog_busy = app.dialog_open;
    if ui.add_enabled(!dialog_busy, egui::Button::new("Generate Example Template")).clicked() {
        app.dialog_open = true;
        let tx = app.dialog_tx.clone();
        let ctx2 = ctx.clone();
        let start = dialog_start_dir();
        spawn_dialog(DialogTag::StructuredTemplateSave, tx, ctx2, move || {
            rfd::FileDialog::new()
                .set_title("Save Example Geometry Template")
                .add_filter("Text file", &["txt"])
                .set_file_name("geometry.txt")
                .set_directory(start)
                .save_file()
        });
    }
}

// ---------------------------------------------------------------------------
// Manual mode
// ---------------------------------------------------------------------------

fn show_manual_mode(
    ui: &mut egui::Ui,
    app: &mut MeshApp,
    validation: &ValidationResult,
) {
    // -----------------------------------------------------------------------
    // dx / dy text fields
    // -----------------------------------------------------------------------
    let dx_err = validation.errors.iter().any(|e| e.field == FieldId::Dx);
    let dy_err = validation.errors.iter().any(|e| e.field == FieldId::Dy);

    ui.horizontal(|ui| {
        ui.label("dx:");
        text_edit_with_border(ui, dx_err, |inner_ui| {
            inner_ui.add(
                egui::TextEdit::singleline(&mut app.project.structured.dx)
                    .desired_width(80.0)
                    .hint_text("e.g. 0.1"),
            )
        });

        ui.add_space(12.0);

        ui.label("dy:");
        text_edit_with_border(ui, dy_err, |inner_ui| {
            inner_ui.add(
                egui::TextEdit::singleline(&mut app.project.structured.dy)
                    .desired_width(80.0)
                    .hint_text("e.g. 0.1"),
            )
        });
    });

    // Show dx/dy inline errors once (same message covers both)
    for err in &validation.errors {
        if err.field == FieldId::Dx || err.field == FieldId::Dy {
            ui.colored_label(egui::Color32::RED, format!("⚠ {}", err.message));
            break;
        }
    }

    ui.add_space(8.0);

    // -----------------------------------------------------------------------
    // Nodes table
    // -----------------------------------------------------------------------
    ui.horizontal(|ui| {
        ui.strong("Nodes");
        ui.add_space(8.0);
        if ui.button("➕ Add Row").clicked() {
            app.project.structured.nodes.push(NodeRow::default());
        }
        let remove_enabled = app.project.structured.selected_node.is_some();
        if ui
            .add_enabled(remove_enabled, egui::Button::new("➖ Remove Row"))
            .clicked()
        {
            let state = &mut app.project.structured;
            if let Some(idx) = state.selected_node {
                if idx < state.nodes.len() {
                    state.nodes.remove(idx);
                    state.selected_node = if state.nodes.is_empty() {
                        None
                    } else {
                        Some(idx.saturating_sub(1).min(state.nodes.len() - 1))
                    };
                }
            }
        }
    });

    // Pre-compute per-row error flags (node)
    let node_x_errs: Vec<bool> = (0..app.project.structured.nodes.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::NodeX(i)))
        .collect();
    let node_y_errs: Vec<bool> = (0..app.project.structured.nodes.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::NodeY(i)))
        .collect();

    let node_count = app.project.structured.nodes.len();
    let node_table_h = (node_count as f32 * 22.0 + 26.0).max(60.0).min(250.0);

    // Clone to satisfy borrow checker inside the TableBuilder closure
    let mut nodes_snap = app.project.structured.nodes.clone();
    let mut sel_node = app.project.structured.selected_node;

    TableBuilder::new(ui)
        .id_salt("structured_nodes_table")
        .striped(true)
        .resizable(false)
        .min_scrolled_height(node_table_h)
        .max_scroll_height(node_table_h)
        .column(Column::exact(32.0))   // #
        .column(Column::exact(120.0))  // x
        .column(Column::exact(120.0))  // y
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("#"); });
            header.col(|ui| { ui.strong("x"); });
            header.col(|ui| { ui.strong("y"); });
        })
        .body(|mut body| {
            for i in 0..nodes_snap.len() {
                let x_err = *node_x_errs.get(i).unwrap_or(&false);
                let y_err = *node_y_errs.get(i).unwrap_or(&false);
                let is_selected = sel_node == Some(i);

                body.row(22.0, |mut row| {
                    row.col(|ui| {
                        if ui.selectable_label(is_selected, format!("{}", i + 1)).clicked() {
                            sel_node = Some(i);
                        }
                    });
                    row.col(|ui| {
                        cell_frame(x_err).show(ui, |ui| {
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut nodes_snap[i].x)
                                        .desired_width(110.0),
                                )
                                .clicked()
                            {
                                sel_node = Some(i);
                            }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(y_err).show(ui, |ui| {
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut nodes_snap[i].y)
                                        .desired_width(110.0),
                                )
                                .clicked()
                            {
                                sel_node = Some(i);
                            }
                        });
                    });
                });
            }
        });

    // Write back
    app.project.structured.nodes = nodes_snap;
    app.project.structured.selected_node = sel_node;

    ui.add_space(8.0);

    // -----------------------------------------------------------------------
    // Edges table
    // -----------------------------------------------------------------------
    ui.horizontal(|ui| {
        ui.strong("Edges");
        ui.add_space(8.0);
        if ui.button("➕ Add Row").clicked() {
            app.project.structured.edges.push(EdgeRow::default());
        }
        let remove_enabled = app.project.structured.selected_edge.is_some();
        if ui
            .add_enabled(remove_enabled, egui::Button::new("➖ Remove Row"))
            .clicked()
        {
            let state = &mut app.project.structured;
            if let Some(idx) = state.selected_edge {
                if idx < state.edges.len() {
                    state.edges.remove(idx);
                    state.selected_edge = if state.edges.is_empty() {
                        None
                    } else {
                        Some(idx.saturating_sub(1).min(state.edges.len() - 1))
                    };
                }
            }
        }
    });

    // Pre-compute per-row error flags (edge)
    let edge_start_errs: Vec<bool> = (0..app.project.structured.edges.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::EdgeStart(i)))
        .collect();
    let edge_end_errs: Vec<bool> = (0..app.project.structured.edges.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::EdgeEnd(i)))
        .collect();
    let edge_tag_errs: Vec<bool> = (0..app.project.structured.edges.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::EdgeTag(i)))
        .collect();

    let edge_count = app.project.structured.edges.len();
    let edge_table_h = (edge_count as f32 * 22.0 + 26.0).max(60.0).min(250.0);

    let mut edges_snap = app.project.structured.edges.clone();
    let mut sel_edge = app.project.structured.selected_edge;

    TableBuilder::new(ui)
        .id_salt("structured_edges_table")
        .striped(true)
        .resizable(false)
        .min_scrolled_height(edge_table_h)
        .max_scroll_height(edge_table_h)
        .column(Column::exact(32.0))  // #
        .column(Column::exact(92.0))  // start
        .column(Column::exact(92.0))  // end
        .column(Column::exact(92.0))  // tag
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("#"); });
            header.col(|ui| { ui.strong("start"); });
            header.col(|ui| { ui.strong("end"); });
            header.col(|ui| { ui.strong("tag"); });
        })
        .body(|mut body| {
            for i in 0..edges_snap.len() {
                let start_err = *edge_start_errs.get(i).unwrap_or(&false);
                let end_err   = *edge_end_errs.get(i).unwrap_or(&false);
                let tag_err   = *edge_tag_errs.get(i).unwrap_or(&false);
                let is_selected = sel_edge == Some(i);

                body.row(22.0, |mut row| {
                    row.col(|ui| {
                        if ui.selectable_label(is_selected, format!("{}", i + 1)).clicked() {
                            sel_edge = Some(i);
                        }
                    });
                    row.col(|ui| {
                        cell_frame(start_err).show(ui, |ui| {
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut edges_snap[i].start)
                                        .desired_width(82.0),
                                )
                                .clicked()
                            {
                                sel_edge = Some(i);
                            }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(end_err).show(ui, |ui| {
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut edges_snap[i].end)
                                        .desired_width(82.0),
                                )
                                .clicked()
                            {
                                sel_edge = Some(i);
                            }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(tag_err).show(ui, |ui| {
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut edges_snap[i].tag)
                                        .desired_width(82.0),
                                )
                                .clicked()
                            {
                                sel_edge = Some(i);
                            }
                        });
                    });
                });
            }
        });

    // Write back
    app.project.structured.edges = edges_snap;
    app.project.structured.selected_edge = sel_edge;

    // -----------------------------------------------------------------------
    // Inline field errors shown below the tables
    // -----------------------------------------------------------------------
    for err in &validation.errors {
        match err.field {
            FieldId::Dx | FieldId::Dy => {} // already shown above dx/dy fields
            FieldId::NodeX(i) | FieldId::NodeY(i) => {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("⚠ Node {}: {}", i + 1, err.message),
                );
            }
            FieldId::EdgeStart(i) | FieldId::EdgeEnd(i) | FieldId::EdgeTag(i) => {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("⚠ Edge {}: {}", i + 1, err.message),
                );
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Winding warning below tables (Req 4 AC8)
    // -----------------------------------------------------------------------
    for warn in &validation.warnings {
        ui.colored_label(
            egui::Color32::from_rgb(200, 130, 0),
            format!("⚠ {warn}"),
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::state::project::InputMode;
    use crate::state::structured::{EdgeRow, NodeRow, StructuredState};
    use crate::ui::validation::{validate_structured, FieldId};

    /// Default StructuredState is in File mode.
    #[test]
    fn default_mode_is_file() {
        let s = StructuredState::default();
        assert_eq!(s.input_mode, InputMode::File);
    }

    /// Adding a NodeRow default-fills with "0.0"/"0.0".
    #[test]
    fn default_node_row_values() {
        let row = NodeRow::default();
        assert_eq!(row.x, "0.0");
        assert_eq!(row.y, "0.0");
    }

    /// Adding an EdgeRow default-fills with start=1, end=2, tag=1.
    #[test]
    fn default_edge_row_values() {
        let row = EdgeRow::default();
        assert_eq!(row.start, "1");
        assert_eq!(row.end, "2");
        assert_eq!(row.tag, "1");
    }

    /// validate_structured returns errors for blank dx/dy (Req 4 AC5).
    #[test]
    fn validation_errors_for_blank_dx_dy() {
        let s = StructuredState::default(); // dx="", dy=""
        let result = validate_structured(&s);
        let fields: Vec<_> = result.errors.iter().map(|e| &e.field).collect();
        assert!(fields.contains(&&FieldId::Dx), "expected Dx error");
        assert!(fields.contains(&&FieldId::Dy), "expected Dy error");
    }

    /// validate_structured passes with no errors for a CCW unit square (Req 4).
    #[test]
    fn validation_passes_for_unit_square() {
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
        let result = validate_structured(&s);
        assert!(result.errors.is_empty(), "expected no errors: {:?}", result.errors);
    }

    /// validate_structured emits EdgeEnd error when edge references non-existent node (Req 4 AC7).
    #[test]
    fn validation_edge_out_of_range() {
        let mut s = StructuredState::default();
        s.dx = "0.1".to_string();
        s.dy = "0.1".to_string();
        s.nodes = vec![NodeRow::default()]; // only 1 node
        s.edges = vec![EdgeRow {
            start: "1".to_string(),
            end: "99".to_string(), // out of range
            tag: "1".to_string(),
        }];
        let result = validate_structured(&s);
        assert!(
            result.errors.iter().any(|e| e.field == FieldId::EdgeEnd(0)),
            "expected EdgeEnd(0) error"
        );
    }

    /// validate_structured emits EdgeTag error for tag = 0 (Req 4 AC6).
    #[test]
    fn validation_edge_tag_zero_is_error() {
        let mut s = StructuredState::default();
        s.dx = "0.1".to_string();
        s.dy = "0.1".to_string();
        s.nodes = vec![
            NodeRow { x: "0.0".to_string(), y: "0.0".to_string() },
            NodeRow { x: "1.0".to_string(), y: "0.0".to_string() },
        ];
        s.edges = vec![EdgeRow {
            start: "1".to_string(),
            end: "2".to_string(),
            tag: "0".to_string(), // invalid
        }];
        let result = validate_structured(&s);
        assert!(
            result.errors.iter().any(|e| e.field == FieldId::EdgeTag(0)),
            "expected EdgeTag(0) error"
        );
    }

    /// validate_structured emits a winding warning for a CW polygon (Req 4 AC8).
    #[test]
    fn validation_winding_warning_for_cw_polygon() {
        let mut s = StructuredState::default();
        s.dx = "0.1".to_string();
        s.dy = "0.1".to_string();
        // CW square: going down-right instead of CCW
        s.nodes = vec![
            NodeRow { x: "0.0".to_string(), y: "0.0".to_string() },
            NodeRow { x: "0.0".to_string(), y: "1.0".to_string() },
            NodeRow { x: "1.0".to_string(), y: "1.0".to_string() },
            NodeRow { x: "1.0".to_string(), y: "0.0".to_string() },
        ];
        s.edges = vec![
            EdgeRow { start: "1".to_string(), end: "2".to_string(), tag: "1".to_string() },
            EdgeRow { start: "2".to_string(), end: "3".to_string(), tag: "1".to_string() },
            EdgeRow { start: "3".to_string(), end: "4".to_string(), tag: "1".to_string() },
            EdgeRow { start: "4".to_string(), end: "1".to_string(), tag: "1".to_string() },
        ];
        let result = validate_structured(&s);
        assert!(!result.warnings.is_empty(), "expected a CW winding warning");
        assert!(
            result.warnings[0].contains("clockwise"),
            "warning should mention clockwise: {}",
            result.warnings[0]
        );
    }

    /// Removing a selected node updates selection to stay in bounds.
    #[test]
    fn remove_selected_node_adjusts_selection() {
        let mut state = StructuredState::default();
        state.nodes = vec![
            NodeRow::default(),
            NodeRow::default(),
            NodeRow::default(),
        ];
        state.selected_node = Some(2);

        // Simulate remove logic
        if let Some(idx) = state.selected_node {
            state.nodes.remove(idx);
            state.selected_node = if state.nodes.is_empty() {
                None
            } else {
                Some(idx.saturating_sub(1).min(state.nodes.len() - 1))
            };
        }

        assert_eq!(state.nodes.len(), 2);
        assert_eq!(state.selected_node, Some(1));
    }

    /// Removing the last node sets selection to None.
    #[test]
    fn remove_last_node_clears_selection() {
        let mut state = StructuredState::default();
        state.nodes = vec![NodeRow::default()];
        state.selected_node = Some(0);

        if let Some(idx) = state.selected_node {
            state.nodes.remove(idx);
            state.selected_node = if state.nodes.is_empty() {
                None
            } else {
                Some(idx.saturating_sub(1).min(state.nodes.len() - 1))
            };
        }

        assert!(state.nodes.is_empty());
        assert_eq!(state.selected_node, None);
    }
}

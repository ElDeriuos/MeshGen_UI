// EasyMesh input panel — File/Manual mode, points/segments tables, template generation.
// Implements Req 2, 3, 5.
// All rfd dialogs are run on background threads to avoid freezing the UI.

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::app::{dialog_start_dir, spawn_dialog, DialogTag, MeshApp};
use crate::state::project::InputMode;
use crate::state::easymesh::{PointRow, SegmentRow};
use crate::ui::validation::{validate_easymesh, FieldId, ValidationResult};

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

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Render the EasyMesh input panel inside `ui`.
pub fn show_easymesh_panel(app: &mut MeshApp, ui: &mut egui::Ui) {
    let validation = validate_easymesh(&app.project.easymesh);

    {
        let state = &mut app.project.easymesh;
        ui.horizontal(|ui| {
            ui.radio_value(&mut state.input_mode, InputMode::File, "File Mode");
            ui.radio_value(&mut state.input_mode, InputMode::Manual, "Manual Mode");
        });
    }

    ui.separator();

    let ctx = ui.ctx().clone();
    match app.project.easymesh.input_mode.clone() {
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
            .easymesh
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
            spawn_dialog(DialogTag::EasyMeshBrowse, tx, ctx2, move || {
                rfd::FileDialog::new()
                    .set_title("Open EasyMesh Input File")
                    .add_filter("EasyMesh input file", &["d"])
                    .set_directory(start)
                    .pick_file()
            });
        }
    });

    if let Some(p) = app.project.easymesh.file_path.clone() {
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
        spawn_dialog(DialogTag::EasyMeshTemplateSave, tx, ctx2, move || {
            rfd::FileDialog::new()
                .set_title("Save EasyMesh Example Template")
                .add_filter("EasyMesh input file", &["d"])
                .set_file_name("example.d")
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
    // Points table
    // -----------------------------------------------------------------------
    ui.horizontal(|ui| {
        ui.strong("Points");
        ui.add_space(8.0);
        if ui.button("➕ Add Row").clicked() {
            app.project.easymesh.points.push(PointRow::default());
        }
        let remove_enabled = app.project.easymesh.selected_point.is_some();
        if ui
            .add_enabled(remove_enabled, egui::Button::new("➖ Remove Row"))
            .clicked()
        {
            let state = &mut app.project.easymesh;
            if let Some(idx) = state.selected_point {
                if idx < state.points.len() {
                    state.points.remove(idx);
                    state.selected_point = if state.points.is_empty() {
                        None
                    } else {
                        Some(idx.saturating_sub(1).min(state.points.len() - 1))
                    };
                }
            }
        }
    });

    // Pre-compute per-row error flags for points
    let pt_x_errs: Vec<bool> = (0..app.project.easymesh.points.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::PointX(i)))
        .collect();
    let pt_y_errs: Vec<bool> = (0..app.project.easymesh.points.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::PointY(i)))
        .collect();
    let pt_sp_errs: Vec<bool> = (0..app.project.easymesh.points.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::PointSpacing(i)))
        .collect();
    let pt_mk_errs: Vec<bool> = (0..app.project.easymesh.points.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::PointMarker(i)))
        .collect();

    let pt_count = app.project.easymesh.points.len();
    let pt_table_h = (pt_count as f32 * 22.0 + 26.0).max(60.0).min(250.0);

    let mut pts_snap = app.project.easymesh.points.clone();
    let mut sel_pt = app.project.easymesh.selected_point;

    TableBuilder::new(ui)
        .id_salt("easymesh_points_table")
        .striped(true)
        .resizable(false)
        .min_scrolled_height(pt_table_h)
        .max_scroll_height(pt_table_h)
        .column(Column::exact(32.0))   // #
        .column(Column::exact(90.0))   // x
        .column(Column::exact(90.0))   // y
        .column(Column::exact(90.0))   // spacing
        .column(Column::exact(70.0))   // marker
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("#"); });
            header.col(|ui| { ui.strong("x"); });
            header.col(|ui| { ui.strong("y"); });
            header.col(|ui| { ui.strong("spacing"); });
            header.col(|ui| { ui.strong("marker"); });
        })
        .body(|mut body| {
            for i in 0..pts_snap.len() {
                let x_err  = *pt_x_errs.get(i).unwrap_or(&false);
                let y_err  = *pt_y_errs.get(i).unwrap_or(&false);
                let sp_err = *pt_sp_errs.get(i).unwrap_or(&false);
                let mk_err = *pt_mk_errs.get(i).unwrap_or(&false);
                let is_sel = sel_pt == Some(i);

                body.row(22.0, |mut row| {
                    row.col(|ui| {
                        if ui.selectable_label(is_sel, format!("{i}")).clicked() {
                            sel_pt = Some(i);
                        }
                    });
                    row.col(|ui| {
                        cell_frame(x_err).show(ui, |ui| {
                            if ui.add(egui::TextEdit::singleline(&mut pts_snap[i].x)
                                .desired_width(80.0)).clicked()
                            { sel_pt = Some(i); }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(y_err).show(ui, |ui| {
                            if ui.add(egui::TextEdit::singleline(&mut pts_snap[i].y)
                                .desired_width(80.0)).clicked()
                            { sel_pt = Some(i); }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(sp_err).show(ui, |ui| {
                            if ui.add(egui::TextEdit::singleline(&mut pts_snap[i].spacing)
                                .desired_width(80.0)).clicked()
                            { sel_pt = Some(i); }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(mk_err).show(ui, |ui| {
                            if ui.add(egui::TextEdit::singleline(&mut pts_snap[i].marker)
                                .desired_width(60.0)).clicked()
                            { sel_pt = Some(i); }
                        });
                    });
                });
            }
        });

    // Write back points
    app.project.easymesh.points = pts_snap;
    app.project.easymesh.selected_point = sel_pt;

    ui.add_space(8.0);

    // -----------------------------------------------------------------------
    // Segments table
    // -----------------------------------------------------------------------
    ui.horizontal(|ui| {
        ui.strong("Segments");
        ui.add_space(8.0);
        if ui.button("➕ Add Row").clicked() {
            app.project.easymesh.segments.push(SegmentRow::default());
        }
        let remove_enabled = app.project.easymesh.selected_segment.is_some();
        if ui
            .add_enabled(remove_enabled, egui::Button::new("➖ Remove Row"))
            .clicked()
        {
            let state = &mut app.project.easymesh;
            if let Some(idx) = state.selected_segment {
                if idx < state.segments.len() {
                    state.segments.remove(idx);
                    state.selected_segment = if state.segments.is_empty() {
                        None
                    } else {
                        Some(idx.saturating_sub(1).min(state.segments.len() - 1))
                    };
                }
            }
        }
    });

    // Pre-compute per-row error flags for segments
    let seg_start_errs: Vec<bool> = (0..app.project.easymesh.segments.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::SegmentStart(i)))
        .collect();
    let seg_end_errs: Vec<bool> = (0..app.project.easymesh.segments.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::SegmentEnd(i)))
        .collect();
    let seg_mk_errs: Vec<bool> = (0..app.project.easymesh.segments.len())
        .map(|i| validation.errors.iter().any(|e| e.field == FieldId::SegmentMarker(i)))
        .collect();

    let seg_count = app.project.easymesh.segments.len();
    let seg_table_h = (seg_count as f32 * 22.0 + 26.0).max(60.0).min(250.0);

    let mut segs_snap = app.project.easymesh.segments.clone();
    let mut sel_seg = app.project.easymesh.selected_segment;

    TableBuilder::new(ui)
        .id_salt("easymesh_segments_table")
        .striped(true)
        .resizable(false)
        .min_scrolled_height(seg_table_h)
        .max_scroll_height(seg_table_h)
        .column(Column::exact(32.0))   // #
        .column(Column::exact(90.0))   // start
        .column(Column::exact(90.0))   // end
        .column(Column::exact(70.0))   // marker
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("#"); });
            header.col(|ui| { ui.strong("start"); });
            header.col(|ui| { ui.strong("end"); });
            header.col(|ui| { ui.strong("marker"); });
        })
        .body(|mut body| {
            for i in 0..segs_snap.len() {
                let start_err = *seg_start_errs.get(i).unwrap_or(&false);
                let end_err   = *seg_end_errs.get(i).unwrap_or(&false);
                let mk_err    = *seg_mk_errs.get(i).unwrap_or(&false);
                let is_sel    = sel_seg == Some(i);

                body.row(22.0, |mut row| {
                    row.col(|ui| {
                        if ui.selectable_label(is_sel, format!("{i}")).clicked() {
                            sel_seg = Some(i);
                        }
                    });
                    row.col(|ui| {
                        cell_frame(start_err).show(ui, |ui| {
                            if ui.add(egui::TextEdit::singleline(&mut segs_snap[i].start)
                                .desired_width(80.0)).clicked()
                            { sel_seg = Some(i); }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(end_err).show(ui, |ui| {
                            if ui.add(egui::TextEdit::singleline(&mut segs_snap[i].end)
                                .desired_width(80.0)).clicked()
                            { sel_seg = Some(i); }
                        });
                    });
                    row.col(|ui| {
                        cell_frame(mk_err).show(ui, |ui| {
                            if ui.add(egui::TextEdit::singleline(&mut segs_snap[i].marker)
                                .desired_width(60.0)).clicked()
                            { sel_seg = Some(i); }
                        });
                    });
                });
            }
        });

    // Write back segments
    app.project.easymesh.segments = segs_snap;
    app.project.easymesh.selected_segment = sel_seg;

    // -----------------------------------------------------------------------
    // Inline field errors shown below the tables
    // -----------------------------------------------------------------------
    for err in &validation.errors {
        match err.field {
            FieldId::PointSpacing(i) => {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("⚠ Point {i}: {}", err.message),
                );
            }
            FieldId::PointX(i) | FieldId::PointY(i) | FieldId::PointMarker(i) => {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("⚠ Point {i}: {}", err.message),
                );
            }
            FieldId::SegmentStart(i) | FieldId::SegmentEnd(i) | FieldId::SegmentMarker(i) => {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("⚠ Segment {i}: {}", err.message),
                );
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Winding and marker warnings below tables (Req 5 AC6, 8, 9)
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
    use crate::state::easymesh::{EasyMeshState, PointRow, SegmentRow};
    use crate::state::project::InputMode;
    use crate::ui::validation::{validate_easymesh, FieldId};

    /// Default EasyMeshState is in File mode.
    #[test]
    fn default_mode_is_file() {
        let s = EasyMeshState::default();
        assert_eq!(s.input_mode, InputMode::File);
    }

    /// Default PointRow values are sensible (non-zero spacing).
    #[test]
    fn default_point_row_has_nonzero_spacing() {
        let row = PointRow::default();
        let spacing: f64 = row.spacing.parse().expect("spacing is a valid float");
        assert!(spacing > 0.0, "default spacing must be > 0");
    }

    /// Default SegmentRow values.
    #[test]
    fn default_segment_row_values() {
        let row = SegmentRow::default();
        assert_eq!(row.start, "0");
        assert_eq!(row.end, "1");
        assert_eq!(row.marker, "1");
    }

    /// validate_easymesh returns a spacing error for zero spacing (Req 5 AC5).
    #[test]
    fn validation_error_for_zero_spacing() {
        let mut s = EasyMeshState::default();
        s.points.push(PointRow {
            x: "0.0".to_string(),
            y: "0.0".to_string(),
            spacing: "0.0".to_string(), // invalid
            marker: "1".to_string(),
        });
        let result = validate_easymesh(&s);
        assert!(
            result.errors.iter().any(|e| e.field == FieldId::PointSpacing(0)),
            "expected PointSpacing(0) error"
        );
    }

    /// validate_easymesh returns a spacing error for negative spacing (Req 5 AC5).
    #[test]
    fn validation_error_for_negative_spacing() {
        let mut s = EasyMeshState::default();
        s.points.push(PointRow {
            x: "1.0".to_string(),
            y: "0.0".to_string(),
            spacing: "-0.1".to_string(),
            marker: "1".to_string(),
        });
        let result = validate_easymesh(&s);
        assert!(
            result.errors.iter().any(|e| e.field == FieldId::PointSpacing(0)),
            "expected PointSpacing(0) error for negative spacing"
        );
    }

    /// validate_easymesh warns for marker < 1 on a point (Req 5 AC6).
    #[test]
    fn validation_warning_for_marker_zero() {
        let mut s = EasyMeshState::default();
        s.points.push(PointRow {
            x: "0.0".to_string(),
            y: "0.0".to_string(),
            spacing: "0.25".to_string(),
            marker: "0".to_string(), // advisory warning
        });
        let result = validate_easymesh(&s);
        assert!(
            !result.warnings.is_empty(),
            "expected a warning for marker = 0"
        );
    }

    /// validate_easymesh returns segment index error when start is out of range (Req 5 AC7).
    #[test]
    fn validation_error_for_segment_start_out_of_range() {
        let mut s = EasyMeshState::default();
        s.points.push(PointRow::default()); // only one point (index 0)
        s.segments.push(SegmentRow {
            start: "5".to_string(), // out of range
            end: "0".to_string(),
            marker: "1".to_string(),
        });
        let result = validate_easymesh(&s);
        assert!(
            result.errors.iter().any(|e| e.field == FieldId::SegmentStart(0)),
            "expected SegmentStart(0) error"
        );
    }

    /// validate_easymesh returns segment index error when end is out of range (Req 5 AC7).
    #[test]
    fn validation_error_for_segment_end_out_of_range() {
        let mut s = EasyMeshState::default();
        s.points.push(PointRow::default());
        s.segments.push(SegmentRow {
            start: "0".to_string(),
            end: "99".to_string(), // out of range
            marker: "1".to_string(),
        });
        let result = validate_easymesh(&s);
        assert!(
            result.errors.iter().any(|e| e.field == FieldId::SegmentEnd(0)),
            "expected SegmentEnd(0) error"
        );
    }

    /// validate_easymesh emits a CW winding warning for a CW outer boundary (Req 5 AC8).
    #[test]
    fn validation_winding_warning_for_cw_outer_boundary() {
        let mut s = EasyMeshState::default();
        // CW square: (0,0) → (0,1) → (1,1) → (1,0)
        for (x, y) in &[("0.0", "0.0"), ("0.0", "1.0"), ("1.0", "1.0"), ("1.0", "0.0")] {
            s.points.push(PointRow {
                x: x.to_string(),
                y: y.to_string(),
                spacing: "0.25".to_string(),
                marker: "1".to_string(),
            });
        }
        let result = validate_easymesh(&s);
        let cw_warn = result.warnings.iter().any(|w| w.contains("clockwise"));
        assert!(cw_warn, "expected a clockwise winding warning; got: {:?}", result.warnings);
    }

    /// validate_easymesh passes with no errors for a valid CCW polygon (Req 5).
    #[test]
    fn validation_passes_for_valid_ccw_polygon() {
        let mut s = EasyMeshState::default();
        // CCW square: (0,0) → (1,0) → (1,1) → (0,1)
        for (x, y) in &[("0.0", "0.0"), ("1.0", "0.0"), ("1.0", "1.0"), ("0.0", "1.0")] {
            s.points.push(PointRow {
                x: x.to_string(),
                y: y.to_string(),
                spacing: "0.25".to_string(),
                marker: "1".to_string(),
            });
        }
        for (st, en) in &[(0usize, 1usize), (1, 2), (2, 3), (3, 0)] {
            s.segments.push(SegmentRow {
                start: st.to_string(),
                end: en.to_string(),
                marker: "1".to_string(),
            });
        }
        let result = validate_easymesh(&s);
        assert!(result.errors.is_empty(), "expected no errors: {:?}", result.errors);
    }

    /// Removing a selected point updates selection to stay in bounds.
    #[test]
    fn remove_selected_point_adjusts_selection() {
        let mut state = EasyMeshState::default();
        state.points = vec![PointRow::default(), PointRow::default(), PointRow::default()];
        state.selected_point = Some(2);

        if let Some(idx) = state.selected_point {
            state.points.remove(idx);
            state.selected_point = if state.points.is_empty() {
                None
            } else {
                Some(idx.saturating_sub(1).min(state.points.len() - 1))
            };
        }

        assert_eq!(state.points.len(), 2);
        assert_eq!(state.selected_point, Some(1));
    }

    /// Removing the last segment sets selection to None.
    #[test]
    fn remove_last_segment_clears_selection() {
        let mut state = EasyMeshState::default();
        state.segments = vec![SegmentRow::default()];
        state.selected_segment = Some(0);

        if let Some(idx) = state.selected_segment {
            state.segments.remove(idx);
            state.selected_segment = if state.segments.is_empty() {
                None
            } else {
                Some(idx.saturating_sub(1).min(state.segments.len() - 1))
            };
        }

        assert!(state.segments.is_empty());
        assert_eq!(state.selected_segment, None);
    }
}

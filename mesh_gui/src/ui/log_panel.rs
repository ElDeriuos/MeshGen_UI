// Log panel — fixed-height bottom panel displaying stdout/stderr from mesh generation runs.
// Always visible. Supports auto-scroll with manual-scroll detection. (Task 15, Req 9)

use eframe::egui;

use crate::app::{MeshApp, RunStatus};

/// Render the log panel as a fixed-height `TopBottomPanel` pinned to the bottom of the window.
///
/// Features:
/// - "Clear Log" button: always available; clears existing lines even while a job runs
///   (Req 9 AC4 — clearing during a run leaves the buffer empty but new lines continue to append).
/// - Monospace scrollable log area using `egui::ScrollArea::vertical()`.
/// - Auto-scroll: when `app.log_auto_scroll` is `true` the view is pinned to the bottom each
///   frame.  A manual upward scroll sets `log_auto_scroll = false`.  Reaching the bottom again
///   resets it to `true`.
pub fn show_log_panel(app: &mut MeshApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("log_panel")
        .resizable(true)
        .min_height(80.0)
        .default_height(160.0)
        .show(ctx, |ui| {
            // ------------------------------------------------------------------
            // Header row: label + Clear Log button
            // ------------------------------------------------------------------
            ui.horizontal(|ui| {
                ui.strong("Log");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let job_running = app.run_status == RunStatus::Running;
                    let clear_tooltip = if job_running {
                        "Clear log (job is running — new lines will continue to appear)"
                    } else {
                        "Clear log"
                    };

                    if ui.button("Clear Log").on_hover_text(clear_tooltip).clicked() {
                        // Req 9 AC4: clear whether or not a job is running.
                        // If a job is running, the channel will keep delivering LogLine messages
                        // which will be appended to the now-empty Vec.
                        app.log.clear();
                        // Keep auto-scroll pinned so the next lines scroll into view.
                        app.log_auto_scroll = true;
                    }

                    // Scroll-lock indicator: small icon showing auto-scroll state.
                    let lock_label = if app.log_auto_scroll { "⏬" } else { "🔒" };
                    let lock_tip = if app.log_auto_scroll {
                        "Auto-scroll enabled — following new output"
                    } else {
                        "Auto-scroll paused — scrolled up (scroll to bottom to resume)"
                    };
                    ui.label(lock_label).on_hover_text(lock_tip);
                });
            });

            ui.separator();

            // ------------------------------------------------------------------
            // Scrollable log area
            // ------------------------------------------------------------------

            // We need to know the content height to decide whether the user has
            // reached the bottom.  We compare the scroll offset *after* the
            // ScrollArea renders so we can detect manual scrolling.
            let scroll_id = egui::Id::new("log_scroll_area");

            let mut scroll_area = egui::ScrollArea::vertical()
                .id_salt(scroll_id)
                .auto_shrink([false, false])
                .stick_to_bottom(app.log_auto_scroll);

            // Legacy f32::MAX pinning removed — stick_to_bottom handles it correctly
            // even on the first frame when content height is not yet known.

            let output = scroll_area.show(ui, |ui| {
                // Capture the monospace font id once.
                let monospace = egui::TextStyle::Monospace.resolve(ui.style());

                for line in &app.log {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(line).font(monospace.clone()),
                        )
                        .wrap(),
                    );
                }

                // Place a zero-size widget at the very end so `scroll_to_cursor`
                // has a target when we want to scroll to bottom.
                ui.allocate_space(egui::Vec2::ZERO);
            });

            // ------------------------------------------------------------------
            // Auto-scroll and manual-scroll detection
            // ------------------------------------------------------------------
            let current_offset = output.state.offset.y;
            let content_height = output.content_size.y;
            let viewport_height = output.inner_rect.height();

            // The scroll area is "at the bottom" when the remaining distance to
            // scroll is within a small epsilon (1 px handles float rounding).
            let at_bottom = content_height <= viewport_height
                || (current_offset + viewport_height + 1.0) >= content_height;

            if at_bottom {
                // User has scrolled to (or is at) the bottom — resume auto-scroll.
                app.log_auto_scroll = true;
            } else if !app.log_auto_scroll {
                // Already paused — nothing to change.
            } else {
                // auto_scroll was true but we are not at the bottom.  This can
                // only happen if the user dragged the scrollbar upward *while*
                // auto_scroll was true (the f32::MAX pin hasn't taken full effect
                // yet, or the user grabbed the bar in the same frame).  Pause.
                app.log_auto_scroll = false;
            }
        });
}

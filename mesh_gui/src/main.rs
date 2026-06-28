// Entry point for the Mesh Generator GUI application.
// Opens an eframe window titled "Mesh Generator" at 1200×800.

mod app;
mod state;
mod ui;
mod runner;
mod templates;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Mesh Generator"),
        ..Default::default()
    };

    eframe::run_native(
        "Mesh Generator",
        native_options,
        Box::new(|cc| Ok(Box::new(app::MeshApp::new(cc)))),
    )
}

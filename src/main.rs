#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod blueprint;
mod export;
mod file_filter;
mod file_picker;
mod file_saver;
mod model;
mod preview;
#[cfg(test)]
mod tests;
mod tileset;
mod ui;

#[cfg(not(target_arch = "wasm32"))]
const APP_TITLE: &str = env!("CARGO_PKG_NAME");

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|_cc| Ok(Box::new(ui::AutomapperApp::default()))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    console_log::init_with_level(log::Level::Info).expect("failed to install the console logger");
    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .expect("no window")
            .document()
            .expect("no document")
            .get_element_by_id("app_canvas")
            .expect("no element with id `app_canvas`")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("`app_canvas` is not a <canvas>");

        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_cc| Ok(Box::new(ui::AutomapperApp::default()))),
            )
            .await
            .expect("failed to start eframe");
    });
}

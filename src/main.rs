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
#[derive(clap::Parser)]
#[command(version, about)]
struct Args {
    /// Tileset image to open
    #[arg(value_parser = read_file)]
    image: Option<file_picker::PickedFile>,

    /// Blueprint to load onto the image
    #[arg(short, long, requires = "image", value_parser = read_file)]
    blueprint: Option<file_picker::PickedFile>,
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file(path: &str) -> Result<file_picker::PickedFile, file_picker::PickError> {
    file_picker::read_file(std::path::Path::new(path))
}

#[cfg(not(target_arch = "wasm32"))]
const ICON: &[u8] = include_bytes!("../assets/ddnet-automap-creator.ico");

#[cfg(not(target_arch = "wasm32"))]
fn app_icon() -> egui::IconData {
    let rgba = image::load_from_memory_with_format(ICON, image::ImageFormat::Ico)
        .expect("bundled icon is a valid ico")
        .into_rgba8();
    egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use clap::Parser as _;

    let args = Args::parse();
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            let app = ui::AutomapperApp::new(&cc.egui_ctx, args.image, args.blueprint);
            Ok(Box::new(app))
        }),
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
                Box::new(|cc| {
                    cc.egui_ctx
                        .options_mut(|options| options.zoom_with_keyboard = false); // zoom is already handled by web, so we disable native zoom
                    Ok(Box::new(ui::AutomapperApp::new(&cc.egui_ctx, None, None)))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}

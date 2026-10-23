mod app;
mod grid;
mod group_panel;
mod preview;
mod status;
mod tile_panel;
mod tile_state;

pub use app::AutomapperApp;

const ADD_ICON: &str = "+";
const DUPLICATE_ICON: &str = "🗐";
const REMOVE_ICON: &str = "🗑";

fn icon_button(ui: &egui::Ui, icon: &'static str) -> egui::Button<'static> {
    egui::Button::new(icon).min_size(egui::Vec2::splat(ui.spacing().interact_size.y))
}

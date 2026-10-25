mod app;
mod grid;
mod group_panel;
mod picking;
mod preview;
mod status;
mod tile_panel;
mod tile_state;

pub use app::AutomapperApp;

const ADD_ICON: &str = "+";
const DUPLICATE_ICON: &str = "🗐";
const REMOVE_ICON: &str = "🗑";
const COPY_ICON: &str = "📋";
const PASTE_ICON: &str = "📥";
const DRAW_ICON: &str = "✏";

fn icon_button(icon: &'static str) -> impl egui::Widget {
    move |ui: &mut egui::Ui| {
        let size = egui::Vec2::splat(ui.spacing().interact_size.y);
        ui.add_sized(size, egui::Button::new(icon))
    }
}

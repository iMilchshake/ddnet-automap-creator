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

const SWITCH_ICON: &str = "⏷";
const BUTTON_GAP: f32 = 2.0;
const BUTTON_ICON_FRACTION: f32 = 0.8;

fn compact_button(icon: &'static str, size: f32) -> impl egui::Widget {
    move |ui: &mut egui::Ui| {
        let text = egui::RichText::new(icon).size(size * BUTTON_ICON_FRACTION);
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = egui::Vec2::ZERO;
            ui.add_sized(egui::Vec2::splat(size), egui::Button::new(text))
        })
        .inner
    }
}

mod app;
mod code_frame;
mod discard;
mod export;
mod grid;
mod group_panel;
mod help;
pub mod history;
mod hover_card;
mod modal_header;
mod modal_section;
mod neighbor_paint;
pub mod overview;
mod picking;
mod preview;
mod save_failure;
mod status;
mod tile_panel;
mod tile_state;
mod welcome;

pub use app::AutomapperApp;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

const ADD_ICON: &str = "+";
const DUPLICATE_ICON: &str = "🗐";
const APPEND_ICON: &str = "📥";
const REMOVE_ICON: &str = "🗑";
const DRAW_ICON: &str = "✏";
const MENU_ICON: &str = "☰";

const SWITCH_ICON: &str = "⏷";
const BUTTON_GAP: f32 = 2.0;
const GITHUB_ICON: char = egui::special_emojis::GITHUB;
const DDNET_AUTOMAP_PATHS: [(&str, &str); 3] = [
    ("Windows", r"%APPDATA%\DDNet\editor\automap"),
    ("Linux", "~/.local/share/ddnet/editor/automap"),
    (
        "macOS",
        "~/Library/Application Support/DDNet/editor/automap",
    ),
];
const BUTTON_ICON_FRACTION: f32 = 0.8;

const SECTION_SPACING: f32 = 8.0;
const SECTION_HEADER_SIZE: f32 = 12.5;

fn section_header(ui: &mut egui::Ui, title: &str) {
    section_header_with(ui, title, |_| {});
}

fn section_header_with(ui: &mut egui::Ui, title: &str, add_note: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(SECTION_SPACING);
    ui.horizontal(|ui| {
        let title = egui::RichText::new(title.to_uppercase())
            .size(SECTION_HEADER_SIZE)
            .strong();
        ui.label(title);
        add_note(ui);
    });
    ui.separator();
}

fn centered_top_space(ui: &egui::Ui, id: egui::Id) -> f32 {
    let content_height = ui.data(|data| data.get_temp::<f32>(id)).unwrap_or_default();

    ((ui.available_height() - content_height) / 2.0).max(0.0)
}

fn remember_height(ui: &egui::Ui, id: egui::Id, height: f32) {
    if ui.data(|data| data.get_temp::<f32>(id)) != Some(height) {
        ui.data_mut(|data| data.insert_temp(id, height));
        ui.ctx().request_repaint();
    }
}

fn show_vertically_centered<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.add_space(centered_top_space(ui, id));
    let shown = ui.vertical_centered(add_contents);
    remember_height(ui, id, shown.response.rect.height());

    shown.inner
}

fn external_link(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    url: impl ToString,
) -> egui::Response {
    ui.add(egui::Hyperlink::from_label_and_url(text, url).open_in_new_tab(true))
}

fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        )
        .size()
        .x
}

fn allocate_centered(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    sense: egui::Sense,
) -> (egui::Rect, egui::Response) {
    let free = ui.available_rect_before_wrap();
    let offset = (free.size() - size).max(egui::Vec2::ZERO) / 2.0;
    let rect = egui::Rect::from_min_size(free.min + offset, size);

    (rect, ui.allocate_rect(rect, sense))
}

fn icon_button_size(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y
}

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

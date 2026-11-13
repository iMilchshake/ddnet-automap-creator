use egui::{
    Align, Button, ColorImage, Context, Frame, Id, Image, Layout, Margin, RichText, ScrollArea,
    Stroke, TextureHandle, TextureOptions, Ui, Vec2,
};

use crate::ui::{APP_NAME, centered_top_space, remember_height};

const ICON_BYTES: &[u8] = include_bytes!("../../assets/ddnet-automap-creator.ico");
const ICON_TEXTURE_ID: &str = "welcome_icon";
const ICON_SIZE: f32 = 96.0;
const ICON_GAP: f32 = 8.0;

const WELCOME_WIDTH: f32 = 520.0;
const WELCOME_ID: &str = "welcome";
const TITLE_SIZE: f32 = 32.0;
const BLOCK_GAP: f32 = 20.0;
const DISCLAIMER_GAP: f32 = 40.0;
const DISCLAIMER_FILL_OPACITY: f32 = 0.08;
const DISCLAIMER_CORNER_RADIUS: u8 = 6;
const DISCLAIMER_MARGIN: Margin = Margin::symmetric(14, 10);
const OPEN_BUTTON_SIZE: Vec2 = Vec2::new(240.0, 40.0);
const OPEN_BUTTON_TEXT_SIZE: f32 = 16.0;
const DISCLAIMER_STROKE_WIDTH: f32 = 1.0;

const OPEN_LABEL: &str = "Open tileset image…";
const FIRST_STEPS_LABEL: &str = "New here? Read the first steps";
const DISCLAIMER: &str = "This is a preview build, intended for testing and feedback. Automappers you save now may not open in newer versions. From v1.0 on, every saved automapper will open in all later versions.";

pub enum WelcomeAction {
    OpenImage,
    OpenFirstSteps,
}

pub fn show(ui: &mut Ui, open_shortcut: &str) -> Option<WelcomeAction> {
    let id = Id::new(WELCOME_ID);
    let top_space = centered_top_space(ui, id);
    let width = ui.available_width().min(WELCOME_WIDTH);
    let column = Vec2::new(width, 0.0);

    ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add_space(top_space);
            let shown = ui.vertical_centered(|ui| {
                ui.allocate_ui_with_layout(column, Layout::top_down(Align::Min), |ui| {
                    ui.set_width(width);
                    show_column(ui, open_shortcut)
                })
                .inner
            });
            remember_height(ui, id, shown.response.rect.height());

            shown.inner
        })
        .inner
}

fn show_column(ui: &mut Ui, open_shortcut: &str) -> Option<WelcomeAction> {
    ui.vertical_centered(|ui| {
        let icon = icon_texture(ui.ctx());
        ui.add(Image::new(&icon).fit_to_exact_size(Vec2::splat(ICON_SIZE)));
        ui.add_space(ICON_GAP);
        ui.label(RichText::new(APP_NAME).size(TITLE_SIZE).strong());
        ui.add_space(BLOCK_GAP);
        let open = show_open_button(ui, open_shortcut);
        ui.add_space(BLOCK_GAP);
        let first_steps = ui
            .link(FIRST_STEPS_LABEL)
            .clicked()
            .then_some(WelcomeAction::OpenFirstSteps);
        ui.add_space(DISCLAIMER_GAP);
        show_disclaimer(ui);

        open.or(first_steps)
    })
    .inner
}

fn icon_texture(ctx: &Context) -> TextureHandle {
    let id = Id::new(ICON_TEXTURE_ID);
    if let Some(texture) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return texture;
    }
    let rgba = app_icon();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color_image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let texture = ctx.load_texture(ICON_TEXTURE_ID, color_image, TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));

    texture
}

pub fn app_icon() -> image::RgbaImage {
    image::load_from_memory_with_format(ICON_BYTES, image::ImageFormat::Ico)
        .expect("bundled icon is a valid ico")
        .into_rgba8()
}

fn show_open_button(ui: &mut Ui, open_shortcut: &str) -> Option<WelcomeAction> {
    let label = RichText::new(OPEN_LABEL)
        .size(OPEN_BUTTON_TEXT_SIZE)
        .strong();
    let button = Button::new(label)
        .fill(ui.visuals().selection.bg_fill)
        .min_size(OPEN_BUTTON_SIZE);
    let clicked = ui.add(button).clicked();
    ui.weak(open_shortcut);

    clicked.then_some(WelcomeAction::OpenImage)
}

fn show_disclaimer(ui: &mut Ui) {
    let warn = ui.visuals().warn_fg_color;
    Frame::new()
        .fill(warn.gamma_multiply(DISCLAIMER_FILL_OPACITY))
        .stroke(Stroke::new(DISCLAIMER_STROKE_WIDTH, warn))
        .corner_radius(DISCLAIMER_CORNER_RADIUS)
        .inner_margin(DISCLAIMER_MARGIN)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(DISCLAIMER);
        });
}

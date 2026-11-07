use egui::{
    Align, Layout, Rect, Response, RichText, Sense, Sides, Ui, UiBuilder, Vec2, WidgetInfo,
    WidgetType,
};

const CLOSE_HOVER: &str = "Close";
const CLOSE_INFO: &str = "Close window";
const CLOSE_ID: &str = "window_close_button";
const CLOSE_SHRINK: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TitleAlign {
    Start,
    Center,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Warning,
}

pub struct ModalHeader<'a> {
    pub title: &'a str,
    pub align: TitleAlign,
    pub tone: Tone,
}

pub fn show(ui: &mut Ui, header: &ModalHeader) -> bool {
    let closed = match header.align {
        TitleAlign::Start => {
            Sides::new()
                .show(ui, |ui| show_title(ui, header.title), show_close)
                .1
        }
        TitleAlign::Center => show_centered(ui, header.title),
    };

    ui.scope(|ui| {
        if header.tone == Tone::Warning {
            ui.visuals_mut().widgets.noninteractive.bg_stroke.color = ui.visuals().warn_fg_color;
        }
        ui.separator();
    });

    closed
}

fn show_title(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).heading());
}

fn show_centered(ui: &mut Ui, title: &str) -> bool {
    let x_range = ui.available_rect_before_wrap().x_range();
    let centered = UiBuilder::new().layout(Layout::top_down(Align::Center));
    let title_row = ui
        .scope_builder(centered, |ui| show_title(ui, title))
        .response
        .rect;

    let row = Rect::from_x_y_ranges(x_range, title_row.y_range());
    let close_side = UiBuilder::new()
        .max_rect(row)
        .layout(Layout::right_to_left(Align::Center));

    show_close(&mut ui.new_child(close_side))
}

fn show_close(ui: &mut Ui) -> bool {
    let (_, rect) = ui.allocate_space(Vec2::splat(ui.spacing().icon_width));

    close_button(ui, rect).on_hover_text(CLOSE_HOVER).clicked()
}

fn close_button(ui: &mut Ui, rect: Rect) -> Response {
    let close_id = ui.auto_id_with(CLOSE_ID);
    let response = ui.interact(rect, close_id, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), CLOSE_INFO));

    let visuals = ui.style().interact(&response);
    let rect = rect.shrink(CLOSE_SHRINK).expand(visuals.expansion);
    let stroke = visuals.fg_stroke;
    ui.painter()
        .line_segment([rect.left_top(), rect.right_bottom()], stroke);
    ui.painter()
        .line_segment([rect.right_top(), rect.left_bottom()], stroke);

    response
}

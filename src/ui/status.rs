use std::time::Duration;

use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};

const AUTO_CLEAR_SECONDS: f64 = 8.0;
const TOAST_SECONDS: f64 = 5.0;
const TOAST_AREA_MARGIN: f32 = 10.0;
const TOAST_FILL_TINT: f32 = 0.2;
const TOAST_STROKE_WIDTH: f32 = 1.0;
const TOAST_WIDTH: f32 = 240.0;
const TOASTS_ID_SOURCE: &str = "warning_toasts";
const TOAST_PADDING: f32 = 10.0;
const TOAST_PROGRESS_HEIGHT: f32 = 2.0;

struct Entry {
    message: String,
    shown_at: f64,
}

#[derive(Default)]
pub struct StatusLine {
    entry: Option<Entry>,
    pending_toasts: Vec<Toast>,
}

impl StatusLine {
    pub fn info(&mut self, ctx: &egui::Context, message: impl Into<String>) {
        let message = message.into();
        log::info!("{message}");

        let shown_at = ctx.input(|input| input.time);
        self.entry = Some(Entry { message, shown_at });
        ctx.request_repaint_after(Duration::from_secs_f64(AUTO_CLEAR_SECONDS));
    }

    pub fn warning(&mut self, ctx: &egui::Context, message: impl Into<String>) {
        let message = message.into();
        log::warn!("{message}");

        self.pending_toasts.push(Toast {
            text: message.into(),
            kind: ToastKind::Warning,
            options: ToastOptions::default()
                .duration_in_seconds(TOAST_SECONDS)
                .show_progress(true),
            ..Default::default()
        });
        ctx.request_repaint();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let now = ui.input(|input| input.time);
        let Some(entry) = &self.entry else {
            ui.label("");
            return;
        };

        let age = now - entry.shown_at;
        if age >= AUTO_CLEAR_SECONDS {
            self.entry = None;
            ui.label("");
            return;
        }

        ui.colored_label(ui.visuals().weak_text_color(), &entry.message);

        let remaining = AUTO_CLEAR_SECONDS - age;
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(remaining));
    }

    pub fn show_toasts(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let screen = ui.ctx().content_rect();
        let offset = egui::pos2(
            area.right() - screen.right() - TOAST_AREA_MARGIN,
            area.top() - screen.top() + TOAST_AREA_MARGIN,
        );
        let id = egui::Id::new(TOASTS_ID_SOURCE);
        let pending = unique_by_text(self.pending_toasts.drain(..));
        ui.data_mut(|data| {
            data.get_temp_mut_or_default::<Vec<Toast>>(id)
                .retain(|stored| {
                    !pending
                        .iter()
                        .any(|new| new.text.text() == stored.text.text())
                })
        });

        let mut toasts = Toasts::with_id(id)
            .anchor(egui::Align2::RIGHT_TOP, offset)
            .direction(egui::Direction::TopDown)
            .custom_contents(ToastKind::Warning, warning_contents);
        for toast in pending {
            toasts.add(toast);
        }
        toasts.show(ui);
    }
}

fn unique_by_text(toasts: impl Iterator<Item = Toast>) -> Vec<Toast> {
    let mut unique: Vec<Toast> = Vec::new();
    for toast in toasts {
        unique.retain(|kept| kept.text.text() != toast.text.text());
        unique.push(toast);
    }
    unique
}

fn warning_contents(ui: &mut egui::Ui, toast: &mut Toast) -> egui::Response {
    let visuals = ui.visuals();
    let accent = visuals.warn_fg_color;
    let fill = visuals.window_fill().lerp_to_gamma(accent, TOAST_FILL_TINT);
    let frame = egui::Frame::window(ui.style())
        .fill(fill)
        .stroke(egui::Stroke::new(TOAST_STROKE_WIDTH, accent))
        .shadow(visuals.popup_shadow)
        .corner_radius(visuals.window_corner_radius)
        .inner_margin(TOAST_PADDING);
    let rect = frame
        .show(ui, |ui| {
            ui.set_width(TOAST_WIDTH);
            ui.label(toast.text.clone())
        })
        .response
        .rect;

    let mut bar = rect;
    bar.set_top(rect.bottom() - TOAST_PROGRESS_HEIGHT);
    bar.set_right(rect.left() + rect.width() * toast.options.progress() as f32);
    ui.painter()
        .with_clip_rect(bar)
        .rect_filled(rect, ui.visuals().window_corner_radius, accent);

    let response = ui
        .interact(rect, ui.id().with("toast"), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        toast.close();
    }
    response
}

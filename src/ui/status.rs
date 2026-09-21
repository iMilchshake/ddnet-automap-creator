//! The single status line at the bottom of the window. Failures the user can
//! trigger are reported here instead of crashing.

use std::time::Duration;

const AUTO_CLEAR_SECONDS: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusLevel {
    Info,
    Warning,
}

struct Entry {
    message: String,
    level: StatusLevel,
    /// Context time, which is monotonic on both targets unlike
    /// `std::time::Instant`.
    shown_at: f64,
}

#[derive(Default)]
pub struct StatusLine {
    entry: Option<Entry>,
}

impl StatusLine {
    pub fn info(&mut self, ctx: &egui::Context, message: impl Into<String>) {
        self.set(ctx, StatusLevel::Info, message);
    }

    pub fn warning(&mut self, ctx: &egui::Context, message: impl Into<String>) {
        self.set(ctx, StatusLevel::Warning, message);
    }

    fn set(&mut self, ctx: &egui::Context, level: StatusLevel, message: impl Into<String>) {
        let message = message.into();
        match level {
            StatusLevel::Info => log::info!("{message}"),
            StatusLevel::Warning => log::warn!("{message}"),
        }

        let shown_at = ctx.input(|input| input.time);
        self.entry = Some(Entry {
            message,
            level,
            shown_at,
        });
        ctx.request_repaint_after(Duration::from_secs_f64(AUTO_CLEAR_SECONDS));
    }

    /// Draws the current message and expires it once it is old enough.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let now = ui.input(|input| input.time);
        let Some(entry) = &self.entry else {
            // Empty label keeps the panel height stable.
            ui.label("");
            return;
        };

        let age = now - entry.shown_at;
        if age >= AUTO_CLEAR_SECONDS {
            self.entry = None;
            ui.label("");
            return;
        }

        let color = match entry.level {
            StatusLevel::Info => ui.visuals().weak_text_color(),
            StatusLevel::Warning => ui.visuals().warn_fg_color,
        };
        ui.colored_label(color, &entry.message);

        let remaining = AUTO_CLEAR_SECONDS - age;
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(remaining));
    }
}

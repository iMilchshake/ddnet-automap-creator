use egui::{Sense, TextureHandle, Ui, Vec2};

use super::orientations::drawn_name;
use super::{TileContext, TilePanel};
use crate::model::pool::{ChanceMode, Pool};
use crate::model::tile::{Centre, Chance};
use crate::model::transform::Transform;
use crate::tileset::Tileset;
use crate::ui::grid;
use crate::ui::neighbor_paint::paint_turned_tile;

const SECTION_SPACING: f32 = 6.0;
const MEMBER_SIZE: f32 = 20.0;
const POOL_TOOLTIP: &str = "All tiles that have exactly these neighbors, including rotated and \
                            flipped ones.";
const UNCHANGED: &str = "stays as it is";
const SHARED_POOLS_HEADING: &str = "Shared with other tiles";

struct PoolView<'a> {
    tileset: &'a Tileset,
    texture: &'a TextureHandle,
    mode: ChanceMode,
    own_share: String,
}

impl TilePanel {
    pub(super) fn show_pools(
        &self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
    ) {
        let shared: Vec<&Pool> = context
            .pools
            .iter()
            .copied()
            .filter(|pool| pool.members.len() > 1)
            .collect();
        if shared.is_empty() {
            return;
        }

        let own_pool = context
            .pools
            .iter()
            .copied()
            .find(|pool| {
                pool.member_of(self.tile)
                    .is_some_and(|member| member.rule == self.index)
            })
            .unwrap_or(context.pools[0]);
        let view = PoolView {
            tileset,
            texture,
            mode: context.mode,
            own_share: self.share_label(own_pool, context.mode),
        };
        ui.add_space(SECTION_SPACING);
        ui.strong(SHARED_POOLS_HEADING);
        for (index, pool) in shared.into_iter().enumerate() {
            self.show_pool(ui, &view, pool, index == 0);
        }
    }

    fn show_pool(&self, ui: &mut Ui, view: &PoolView, pool: &Pool, open: bool) {
        let transform = pool
            .member_of(self.tile)
            .map_or(Transform::IDENTITY, |member| member.transform);
        let share = self.share_label(pool, view.mode);
        let title = match pool.centre {
            Centre::Full => format!("{}: {share}", drawn_name(transform)),
            Centre::Empty => format!("{}, empty center: {share}", drawn_name(transform)),
        };
        let title = match share == view.own_share {
            true => egui::RichText::new(title),
            false => egui::RichText::new(title).color(ui.visuals().warn_fg_color),
        };

        let header = egui::CollapsingHeader::new(title)
            .id_salt(("pool", self.tile, pool.neighborhood, pool.centre))
            .default_open(open)
            .show(ui, |ui| self.show_members(ui, view, pool));
        header.header_response.on_hover_text(POOL_TOOLTIP);
    }

    fn show_members(&self, ui: &mut Ui, view: &PoolView, pool: &Pool) {
        egui::Grid::new(("pool_members", pool.neighborhood, pool.centre)).show(ui, |ui| {
            for (member, share) in pool.members.iter().zip(pool.shares(view.mode)) {
                let (rect, _response) =
                    ui.allocate_exact_size(Vec2::splat(MEMBER_SIZE), Sense::hover());
                let slice = &view.tileset.tiles[member.tile];
                paint_turned_tile(ui.painter(), rect, view.texture, slice, member.transform);

                let name = egui::RichText::new(format!("Tile {}", member.tile));
                ui.label(match member.tile == self.tile {
                    true => name.strong(),
                    false => name,
                });
                ui.label(grid::format_percent(share));
                ui.end_row();
            }

            let unchanged = Chance::FULL.percent() - pool.coverage(view.mode);
            if unchanged > 0.0 {
                let warn = ui.visuals().warn_fg_color;
                ui.label("");
                ui.colored_label(warn, UNCHANGED);
                ui.colored_label(warn, grid::format_percent(unchanged));
                ui.end_row();
            }
        });
    }

    fn share_label(&self, pool: &Pool, mode: ChanceMode) -> String {
        pool.share_of(self.tile, mode)
            .map(grid::format_percent)
            .unwrap_or_default()
    }
}

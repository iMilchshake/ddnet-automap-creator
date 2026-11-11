use egui::Ui;

use crate::model::tile::TileRule;
use crate::ui::grid::{GridView, describe_group, group_of};
use crate::ui::neighbor_paint::{paint_centre, paint_neighbor, show_pattern};
use crate::ui::tile_state::{TileState, tile_state};

const CARD_CELL_SIZE: f32 = 18.0;
const RULES_PER_ROW: usize = 4;

const IN_GROUP: &str = "is in group";
const TURNED_OFF: &str = "turned off";

pub fn show(ui: &mut Ui, view: &GridView, tile: usize) {
    let state = tile_state(view.tileset, view.project, tile);
    if matches!(
        state,
        TileState::Configured(_) | TileState::Deactivated(Some(_))
    ) {
        show_rule_pictures(ui, view, tile);
    }

    ui.label(summary(view, tile, &state));
    if let Some(group) = group_of(view.project, tile) {
        ui.weak(describe_group(group));
    }
}

fn show_rule_pictures(ui: &mut Ui, view: &GridView, tile: usize) {
    let rules = view.project.rules_of(tile);
    for (row, row_rules) in rules.chunks(RULES_PER_ROW).enumerate() {
        ui.horizontal_top(|ui| {
            for (column, rule) in row_rules.iter().enumerate() {
                let number = row * RULES_PER_ROW + column + 1;
                ui.vertical(|ui| {
                    show_rule_picture(ui, view, tile, rule);
                    ui.weak(caption(number, rule));
                });
            }
        });
    }
}

fn show_rule_picture(ui: &mut Ui, view: &GridView, tile: usize, rule: &TileRule) {
    let reach = rule.neighborhood.reach();
    show_pattern(
        ui,
        reach,
        CARD_CELL_SIZE,
        |ui, rect, neighbor| match neighbor {
            Some(offset) => paint_neighbor(
                ui,
                rect,
                rule.neighborhood.state(offset),
                view.texture,
                view.tileset,
            ),
            None => paint_centre(
                ui,
                rect,
                view.texture,
                &view.tileset.tiles[tile],
                rule.centre,
                rule.drawn,
            ),
        },
    );
}

fn caption(number: usize, rule: &TileRule) -> String {
    match rule.chance.is_full() {
        true => format!("Rule {number}"),
        false => format!("Rule {number}, {}%", rule.chance.percent()),
    }
}

fn summary(view: &GridView, tile: usize, state: &TileState) -> String {
    match state {
        TileState::Grouped => {
            let name = group_of(view.project, tile).map_or("", |group| group.name.as_str());
            format!("Tile {tile} {IN_GROUP} {name}")
        }
        TileState::Deactivated(_) => format!("Tile {tile}, {TURNED_OFF}"),
        TileState::Locked | TileState::Unconfigured | TileState::Configured(_) => {
            format!("Tile {tile}")
        }
    }
}

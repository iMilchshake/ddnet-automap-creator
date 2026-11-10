use egui::{TextureHandle, Ui};

use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood};
use crate::model::pool::{ChanceMode, Pool};
use crate::model::tile::{Centre, Chance, TileMods, TileRule};
use crate::model::transform::{Orientations, Transform};
use crate::tileset::Tileset;

mod header;
mod icons;
mod neighbors;
pub(super) mod orientations;
mod pools;

const CELL_GAP: f32 = 3.0;
const HEADING_SPACING: f32 = 4.0;
const SECTION_SPACING: f32 = 6.0;

pub struct TileContext<'a> {
    pub pools: Vec<&'a Pool>,
    pub mode: ChanceMode,
    pub rule_count: usize,
    pub deactivated: bool,
    pub copied: Option<&'a CopiedRule>,
    pub flash: Option<GridFlash>,
}

#[derive(Clone, Copy)]
pub enum FlashKind {
    Copy,
    Paste,
}

#[derive(Clone, Copy)]
pub struct GridFlash {
    pub kind: FlashKind,
    pub remaining: f32,
}

pub struct CopiedRule {
    pub rule: TileRule,
    pub tile: usize,
    pub index: usize,
}

impl CopiedRule {
    fn paste_tooltip(&self) -> String {
        format!("Paste rule {} of tile {} (V)", self.index + 1, self.tile)
    }
}

pub enum TileEdit {
    Apply { index: usize, rule: TileRule },
    Select(usize),
    Add,
    Duplicate(usize),
    Remove(usize),
    Copy(usize),
    Paste(usize),
    ToggleActive,
}

pub struct TilePanel {
    tile: usize,
    index: usize,
    neighborhood: Neighborhood,
    centre: Centre,
    mods: TileMods,
    custom: Option<Orientations>,
    drawn: Transform,
    /// A plain percentage while editing; only a valid one becomes a `Chance`.
    chance_percent: f32,
    has_rule: bool,
    error: Option<String>,
    picking: Option<usize>,
}

impl TilePanel {
    pub fn new(tile: usize, index: usize, rule: TileRule, has_rule: bool) -> Self {
        Self {
            tile,
            index,
            neighborhood: rule.neighborhood,
            centre: rule.centre,
            mods: rule.mods,
            custom: rule.custom,
            drawn: rule.drawn,
            chance_percent: rule.chance.percent(),
            has_rule,
            error: None,
            picking: None,
        }
    }

    pub fn tile(&self) -> usize {
        self.tile
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn is_picking(&self) -> bool {
        self.picking.is_some()
    }

    pub fn picking_label(&self) -> Option<&'static str> {
        self.picking.map(|index| NEIGHBORS[index].label)
    }

    pub fn picked(&self) -> Option<usize> {
        self.neighborhood.state(self.picking?).tile()
    }

    pub fn stop_picking(&mut self) {
        self.picking = None;
    }

    pub fn pick(&mut self, tile: usize) -> Option<TileEdit> {
        let index = self.picking.take()?;
        self.neighborhood.set_state(index, NeighborState::Is(tile));
        self.has_rule = true;

        self.apply()
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
    ) -> Option<TileEdit> {
        let heading_edit = self.show_heading(ui, context);
        ui.add_space(HEADING_SPACING);

        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.stop_picking();
        }

        let changed = self.show_neighborhood(ui, tileset, texture, context);
        ui.add_space(SECTION_SPACING);
        let edit = self.show_options(ui, tileset, texture, changed);

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        self.show_pools(ui, tileset, texture, context);

        heading_edit.or(edit).or(self.key_edit(ui, context))
    }

    fn key_edit(&self, ui: &Ui, context: &TileContext) -> Option<TileEdit> {
        let ctx = ui.ctx();
        if ctx.egui_wants_keyboard_input()
            || ctx.memory(|memory| memory.top_modal_layer().is_some())
        {
            return None;
        }

        let (copy, paste) = ui.input(|input| {
            let plain = input.modifiers.is_none();
            (
                plain && input.key_pressed(egui::Key::C),
                plain && input.key_pressed(egui::Key::V),
            )
        });
        match (copy && self.has_rule, paste && context.copied.is_some()) {
            (true, _) => Some(TileEdit::Copy(self.index)),
            (_, true) => Some(TileEdit::Paste(self.index)),
            _ => None,
        }
    }

    fn apply_if_changed(&mut self, changed: bool) -> Option<TileEdit> {
        if !changed {
            return None;
        }

        self.has_rule = true;
        self.apply()
    }

    fn rule(&self, chance: Chance) -> TileRule {
        TileRule {
            neighborhood: self.neighborhood,
            centre: self.centre,
            mods: self.mods,
            custom: self.custom,
            drawn: self.drawn,
            chance,
        }
    }

    fn turn(&mut self, turn: fn(TileRule, Transform) -> TileRule, by: Transform) {
        let turned = turn(self.rule(Chance::FULL), by);
        self.neighborhood = turned.neighborhood;
        self.drawn = turned.drawn;
        self.picking = None;
    }

    fn apply(&mut self) -> Option<TileEdit> {
        match Chance::new(self.chance_percent) {
            Ok(chance) => {
                self.error = None;
                Some(TileEdit::Apply {
                    index: self.index,
                    rule: self.rule(chance),
                })
            }
            Err(error) => {
                self.error = Some(error.to_string());
                None
            }
        }
    }
}

use egui::{Context, Id, KeyboardShortcut, ScrollArea, Ui};

use crate::ui::app::{
    export_shortcut, open_image_shortcut, redo_alt_shortcut, redo_shortcut,
    save_blueprint_shortcut, undo_shortcut,
};
use crate::ui::modal_header::{self, ModalHeader, TitleAlign, Tone};
use crate::ui::modal_section::{self, show_section};
use crate::ui::neighbor_paint::show_legend;
use crate::ui::{GITHUB_ICON, REPOSITORY};

const HELP_WIDTH: f32 = 520.0;
const MAX_HEIGHT_FRACTION: f32 = 0.6;
const MODAL_ID: &str = "help";
const TAB_ID: &str = "help_tab";
const KEYBINDS_GRID_ID: &str = "keybinds";
const KEY_COLUMN_WIDTH: f32 = 160.0;
const HELP_TITLE: &str = "Help";
const OPEN_PLACEHOLDER: &str = "{open}";
const SAVE_PLACEHOLDER: &str = "{save}";

const GETTING_STARTED_TAB: &str = "Getting started";
const CONCEPTS_TAB: &str = "Concepts";
const KEYBINDS_TAB: &str = "Keybinds";
const ABOUT_TAB: &str = "About";

const ABOUT_HEADING: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));
const ABOUT_SUMMARY: &str = "A visual creator for DDNet automappers.";
const GITHUB_LABEL: &str = "GitHub repository";
const ORIGINAL_PROJECT: &str = "https://github.com/AssassinTee/SimpleDDNetAutomapper";
const RPP: &str = "https://github.com/Aerll/rpp";
const DM1_SOURCE: &str = "https://github.com/teeworlds/teeworlds-maps";
const DM1_LICENSE: &str = "http://creativecommons.org/licenses/by-sa/3.0/";

const GETTING_STARTED: [(&str, &str); 7] = [
    (
        "1. Open a tileset",
        "Open a tileset image with 16×16 tiles ({open}).",
    ),
    (
        "2. Set the neighbors",
        "Click a tile. Then click the 8 cells around it to say which neighbors have to be solid, which have to be empty and which don't matter.",
    ),
    (
        "3. Turn on copies",
        "Turn on the rotated and flipped copies, so one tile works in every direction.",
    ),
    (
        "4. Share spots",
        "If several tiles have the same neighbors, they share the spot and DDNet picks one by chance.",
    ),
    (
        "5. Watch the preview",
        "An orange question mark means no rule fits that spot yet. Click any tile in the preview to jump to its rule.",
    ),
    (
        "6. Save your work",
        "Save a blueprint ({save}) so you can continue later.",
    ),
    (
        "7. Export",
        "Export the .rules file and put it into the editor/automap folder of your DDNet user folder. It needs the same name as the image, for example grass_main.rules. The export window shows where that folder is.",
    ),
];

const RULE_CONCEPT: (&str, &str) = (
    "Rule",
    "Tells DDNet which tile goes on a spot, based on the tiles around it. A tile can have more than one rule.",
);
const NEIGHBOR_STATES_HEADING: &str = "Neighbor states";
const OTHER_CONCEPTS: [(&str, &str); 7] = [
    (
        "Specific tile neighbor",
        "Ctrl+click a neighbor cell and pick a tile in the tileset. That neighbor then has to be exactly this tile.",
    ),
    (
        "Empty center",
        "Right-click the middle cell of a rule. The tile then goes into empty spots instead of solid ones, which is handy for grass on top of blocks.",
    ),
    (
        "Rotated and flipped copies",
        "The tile is also placed turned or mirrored wherever the neighbors match in the same turned way. So one rule covers every direction.",
    ),
    (
        "Chance and shared spots",
        "Tiles with the same neighbors share a spot, and DDNet picks between them using their chances. With Normalize chances on, the chances always add up to 100%.",
    ),
    (
        "Turning a tile off",
        "Right-click a tile in the tileset to skip it without losing its rule. Right-click again to turn it back on.",
    ),
    (
        "Groups",
        "A rectangle of tiles that gets placed as one piece. Fill only needs its own area to be solid. Decorate also needs the ring around it to be solid. Groups higher up in the list win.",
    ),
    (
        "Rule sets",
        "Each rule set is its own automapper. They all end up in the same .rules file, and you pick one in the DDNet editor.",
    ),
];

const ALTERNATIVE_KEYS: &str = " / ";
const OPEN_ACTION: &str = "Open image";
const SAVE_ACTION: &str = "Save blueprint";
const EXPORT_ACTION: &str = "Open the export dialog";
const UNDO_ACTION: &str = "Undo";
const REDO_ACTION: &str = "Redo";

const COPY_ACTION: &str = "Copy the selected rule";
const PASTE_ACTION: &str = "Paste over the selected rule";
const NEIGHBOR_STATE_ACTION: &str = "Set the neighbor under the mouse to empty, solid or any";
const SPECIFIC_TILE_ACTION: &str = "Pick a specific tile for a neighbor";
const DESELECT_ACTION: &str = "Deselect, back to the overview";
const SPACE_ACTION: &str = "Show the other view";
const ZOOM_ACTION: &str = "Zoom";
const RESET_ZOOM_ACTION: &str = "Reset zoom";
const FILES_HEADING: &str = "Files";
const EDITING_HEADING: &str = "Editing";
const VIEW_HEADING: &str = "View";

const ACKNOWLEDGEMENTS_HEADING: &str = "Acknowledgements";

enum SentencePart {
    Text(&'static str),
    Link(&'static str, &'static str),
}

const ACKNOWLEDGEMENTS: [&[SentencePart]; 3] = [
    &[
        SentencePart::Text("Based on the "),
        SentencePart::Link("PyQt6 project", ORIGINAL_PROJECT),
        SentencePart::Text(" by Assa, rewritten from scratch in Rust with many new features."),
    ],
    &[
        SentencePart::Text("Rules are exported via "),
        SentencePart::Link("r++", RPP),
        SentencePart::Text(
            " by HiPulsar, which makes generating .rules files much easier and adds advanced features such as grouped tiles.",
        ),
    ],
    &[
        SentencePart::Text("The preview map dm1 is from "),
        SentencePart::Link("teeworlds-maps", DM1_SOURCE),
        SentencePart::Text(", under "),
        SentencePart::Link("CC-BY-SA 3.0", DM1_LICENSE),
        SentencePart::Text("."),
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    GettingStarted,
    Concepts,
    Keybinds,
    About,
}

impl Tab {
    const ALL: [Self; 4] = [
        Self::GettingStarted,
        Self::Concepts,
        Self::Keybinds,
        Self::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::GettingStarted => GETTING_STARTED_TAB,
            Self::Concepts => CONCEPTS_TAB,
            Self::Keybinds => KEYBINDS_TAB,
            Self::About => ABOUT_TAB,
        }
    }
}

pub fn select_getting_started(ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(Id::new(TAB_ID), Tab::GettingStarted));
}

pub fn show_until_closed(ctx: &Context) -> bool {
    let max_height = ctx.content_rect().height() * MAX_HEIGHT_FRACTION;
    let mut closed = false;

    let modal = egui::Modal::new(Id::new(MODAL_ID))
        .frame(modal_section::modal_frame(ctx))
        .show(ctx, |ui| {
            ui.set_width(HELP_WIDTH);
            let header = ModalHeader {
                title: HELP_TITLE,
                align: TitleAlign::Start,
                tone: Tone::Plain,
            };
            closed = modal_header::show(ui, &header);
            let tab = show_tabs(ui);

            ui.separator();
            ScrollArea::vertical()
                .max_height(max_height)
                .auto_shrink(false)
                .show(ui, |ui| show_tab(ui, tab));
        });

    closed || modal.should_close()
}

fn show_tabs(ui: &mut Ui) -> Tab {
    let id = Id::new(TAB_ID);
    let mut tab = ui.data_mut(|data| *data.get_temp_mut_or_default::<Tab>(id));

    ui.horizontal(|ui| {
        for candidate in Tab::ALL {
            ui.selectable_value(&mut tab, candidate, candidate.label());
        }
    });

    ui.data_mut(|data| data.insert_temp(id, tab));
    tab
}

fn show_tab(ui: &mut Ui, tab: Tab) {
    match tab {
        Tab::GettingStarted => show_getting_started(ui),
        Tab::Concepts => show_concepts(ui),
        Tab::Keybinds => show_keybinds(ui),
        Tab::About => show_about(ui),
    }
}

fn show_getting_started(ui: &mut Ui) {
    let open = ui.ctx().format_shortcut(&open_image_shortcut());
    let save = ui.ctx().format_shortcut(&save_blueprint_shortcut());

    for (title, text) in GETTING_STARTED {
        let text = text
            .replace(OPEN_PLACEHOLDER, &open)
            .replace(SAVE_PLACEHOLDER, &save);
        show_section(ui, title, |ui| {
            ui.label(text);
        });
    }
}

fn show_concepts(ui: &mut Ui) {
    show_concept(ui, RULE_CONCEPT);
    show_section(ui, NEIGHBOR_STATES_HEADING, show_legend);

    for concept in OTHER_CONCEPTS {
        show_concept(ui, concept);
    }
}

fn show_concept(ui: &mut Ui, (term, definition): (&str, &str)) {
    show_section(ui, term, |ui| {
        ui.label(definition);
    });
}

fn format_shortcuts(ctx: &Context, shortcuts: &[KeyboardShortcut]) -> String {
    let keys: Vec<String> = shortcuts
        .iter()
        .map(|shortcut| ctx.format_shortcut(shortcut))
        .collect();
    keys.join(ALTERNATIVE_KEYS)
}

fn keybind_sections(ctx: &Context) -> [(&'static str, Vec<(String, &'static str)>); 3] {
    let keys = |shortcuts: &[KeyboardShortcut]| format_shortcuts(ctx, shortcuts);
    [
        (
            FILES_HEADING,
            vec![
                (keys(&[open_image_shortcut()]), OPEN_ACTION),
                (keys(&[save_blueprint_shortcut()]), SAVE_ACTION),
                (keys(&[export_shortcut()]), EXPORT_ACTION),
            ],
        ),
        (
            EDITING_HEADING,
            vec![
                (keys(&[undo_shortcut()]), UNDO_ACTION),
                (keys(&[redo_shortcut(), redo_alt_shortcut()]), REDO_ACTION),
                ("C".to_owned(), COPY_ACTION),
                ("V".to_owned(), PASTE_ACTION),
                ("1 / 2 / 3".to_owned(), NEIGHBOR_STATE_ACTION),
                ("Ctrl click".to_owned(), SPECIFIC_TILE_ACTION),
                ("Esc".to_owned(), DESELECT_ACTION),
            ],
        ),
        (
            VIEW_HEADING,
            vec![
                ("Space (hold)".to_owned(), SPACE_ACTION),
                ("Ctrl + / Ctrl -".to_owned(), ZOOM_ACTION),
                ("Ctrl 0".to_owned(), RESET_ZOOM_ACTION),
            ],
        ),
    ]
}

fn show_keybinds(ui: &mut Ui) {
    for (heading, rows) in keybind_sections(ui.ctx()) {
        show_section(ui, heading, |ui| {
            egui::Grid::new((KEYBINDS_GRID_ID, heading))
                .min_col_width(KEY_COLUMN_WIDTH)
                .show(ui, |ui| {
                    for (keys, action) in rows {
                        ui.label(keys);
                        ui.label(action);
                        ui.end_row();
                    }
                });
        });
    }
}

fn show_about(ui: &mut Ui) {
    show_section(ui, ABOUT_HEADING, |ui| {
        ui.label(ABOUT_SUMMARY);
        ui.hyperlink_to(format!("{GITHUB_ICON} {GITHUB_LABEL}"), REPOSITORY);
    });
    show_section(ui, ACKNOWLEDGEMENTS_HEADING, |ui| {
        for sentence in ACKNOWLEDGEMENTS {
            show_sentence(ui, sentence);
        }
    });
}

fn show_sentence(ui: &mut Ui, parts: &[SentencePart]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for part in parts {
            match part {
                SentencePart::Text(text) => ui.label(*text),
                SentencePart::Link(label, url) => ui.hyperlink_to(*label, *url),
            };
        }
    });
}

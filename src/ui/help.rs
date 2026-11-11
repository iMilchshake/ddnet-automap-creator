use egui::{Context, Id, KeyboardShortcut, ScrollArea, Ui, vec2};

use crate::ui::app::{
    export_shortcut, open_automapper_shortcut, open_image_shortcut, redo_alt_shortcut,
    redo_shortcut, save_automapper_shortcut, undo_shortcut,
};
use crate::ui::code_frame::{code_block, code_chip};
use crate::ui::modal_header::{self, ModalHeader, TitleAlign, Tone};
use crate::ui::modal_section::{self, paragraph_gap, show_section};
use crate::ui::neighbor_paint::show_legend;
use crate::ui::{
    ALTERNATIVE_KEYS, AUTHOR_CREDIT, DDNET_AUTOMAP_PATHS, GITHUB_ICON, REPOSITORY, external_link,
};

const HELP_WIDTH: f32 = 520.0;
const PATHS_GRID_ID: &str = "automap_paths";
const PATHS_COLUMN_GAP: f32 = 12.0;
const PATHS_ROW_GAP: f32 = 8.0;
const MAX_HEIGHT_FRACTION: f32 = 0.6;
const MODAL_ID: &str = "help";
const TAB_ID: &str = "help_tab";
const KEYBINDS_GRID_ID: &str = "keybinds";
const KEY_COLUMN_WIDTH: f32 = 180.0;
const HELP_TITLE: &str = "Help";
const SAVE_PLACEHOLDER: &str = "{save}";

const GETTING_STARTED_TAB: &str = "Getting started";
const CONCEPTS_TAB: &str = "Concepts";
const KEYBINDS_TAB: &str = "Keybinds";
const ABOUT_TAB: &str = "About";

const ABOUT_HEADING: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));
const ABOUT_SUMMARY: &str = "A visual creator for DDNet automappers.";
const GITHUB_LABEL: &str = "GitHub repository";
const ORIGINAL_PROJECT: &str = "https://github.com/AssassinTee/SimpleDDNetAutomapper";
const RPP: &str = "https://github.com/Aerll/rpp";
const DM1_SOURCE: &str = "https://github.com/teeworlds/teeworlds-maps";
const DM1_LICENSE: &str = "http://creativecommons.org/licenses/by-sa/3.0/";

const DESTINATION_HEADING: &str = "Where to put the automapper?";
const DESTINATION: &str =
    "Put the .rules file into your local automapper folder, depending on your system.";
const NAME_NOTE_START: &str = "For DDNet to recognise the automapper, it ";
const NAME_NOTE_EMPHASIS: &str = "must";
const NAME_NOTE_MIDDLE: &str = " be named like the tileset image, e.g. ";
const NAME_NOTE_EXAMPLE_RULES: &str = "grass_main.rules";
const NAME_NOTE_FOR: &str = " for ";
const NAME_NOTE_EXAMPLE_IMAGE: &str = "grass_main.png";
const NAME_NOTE_END: &str = ".";

const GETTING_STARTED: [(&str, &str); 5] = [
    ("1. Open a tileset", "Open a tileset image / mapres."),
    (
        "2. Define your first rule",
        "Click a tile in the tileset view. Then in the sidebar, click the 8 cells around it to say which neighbors have to be solid (left-click), which have to be empty (right-click) and which can be any.",
    ),
    (
        "3. Add copies of the rule",
        "Turn on rotated and/or flipped copies, so your rule also works in other orientations. This is important if you want to, for example, use a top-left corner tile for top-right corners as well, but flipped.",
    ),
    (
        "4. Check the preview",
        "The preview view (left) allows you to immediately check your rules. An orange question mark means no rule fits that spot yet. Click any tile in the preview to jump to its rule.",
    ),
    (
        "5. Save your rule set",
        "Save the automapper ({save}) as a .rules file. It also embeds a blueprint of your rules, so you can open it again and edit it later.",
    ),
];

const RULE_CONCEPT: (&str, &str) = (
    "Rule",
    "A rule is defined by the neighborhood around a tile. By default this is the 8 surrounding cells. Unlock the neighborhood size to add rows and columns on any side, where new cells start as any. Each cell must either be solid or empty, or can be set to any. A cell can also require a specific tile. When DDNet runs the automapper, it sweeps over every position of the map and places the tile wherever the neighborhood of one of its rules matches. If several rules match the same spot, the more specific one wins.",
);
const NEIGHBOR_STATES_HEADING: &str = "Neighbor states";
const NEIGHBOR_STATES_INTRO: &str = "The 3 basic states of a neighboring cell look like this:";
const OTHER_CONCEPTS: [(&str, &str); 8] = [
    (
        "Rotated and flipped copies",
        "For each rotated/flipped variant, another rule is added that applies the same transformation to the tile that should be placed and its neighborhood. This allows one rule to be reused for different orientations. If the neighbors look the same after turning, the turned tiles are mixed in randomly.",
    ),
    (
        "Multiple rules per tile",
        "If the rotated/flipped copies are not enough for your use case, you can also create more than one rule per tile, with different neighborhoods. Click + next to the rule tabs to add another rule and switch between them with the Rule 1, Rule 2 tabs. You can also transform just the selected center tile with buttons that appear once you hover over it. This is just a quality of life feature that makes certain rules easier to build.",
    ),
    (
        "Specific tile neighbor",
        "Ctrl+click a neighbor cell and pick a specific tile in the tileset view. That neighbor cell then has to be exactly this tile. These rules are applied in a following sweep after all other tiles are already placed (NewRun) and can overwrite what's already there.",
    ),
    (
        "Empty center",
        "Right-click the middle cell of a rule. The tile then goes into empty spots instead of solid ones, which is relevant for e.g. grass on top of blocks.",
    ),
    (
        "Chance and conflicting rules",
        "Tiles with the exact same neighbors conflict with each other. In this case DDNet picks between them using their chances. With Normalize chances on, the chances always add up to 100%. You can re-generate the preview view to retry the automapper for multiple seeds.",
    ),
    (
        "Turning a tile off",
        "Right-click a tile in the tileset to deactivate it without losing the configured rules. Right-click again to turn it back on.",
    ),
    (
        "Groups",
        "A rectangle of tiles that gets placed as one piece. Fill only needs its own area to be solid. Decorate also needs the blocks around it to be solid. Groups higher up in the list are applied first. Groups are applied after single tile rules.",
    ),
    (
        "Rule sets",
        "Each rule set is its own automapper. They all end up in the same .rules file, and you pick one in the DDNet editor.",
    ),
];

const OPEN_ACTION: &str = "Open image";
const OPEN_AUTOMAPPER_ACTION: &str = "Open automapper";
const SAVE_ACTION: &str = "Save automapper";
const EXPORT_ACTION: &str = "Advanced export";
const UNDO_ACTION: &str = "Undo";
const REDO_ACTION: &str = "Redo";

const COPY_ACTION: &str = "Copy the selected rule";
const PASTE_ACTION: &str = "Paste over the selected rule";
const NEIGHBOR_STATE_ACTION: &str = "Set the neighbor under the mouse to empty, solid or any";
const MOUSE_ROWS: [(&str, &str); 11] = [
    ("Left-click a tile", "Select it, click again to deselect"),
    ("Right-click a tile", "Turn the tile off or back on"),
    ("Left-click a neighbor", "Switch between solid and any"),
    ("Right-click a neighbor", "Set it to empty"),
    ("Ctrl+click a neighbor", "Pick a specific tile for it"),
    ("Left-click the center", "Place the tile on solid spots"),
    ("Right-click the center", "Place the tile on empty spots"),
    (
        "Left / right-click a copy",
        "Turn a rotated or flipped copy on or off",
    ),
    (
        "Click in the preview",
        "Jump to the rule that placed the tile",
    ),
    (
        "Shift+drag over tiles",
        "Add the tiles to the selected group",
    ),
    (
        "Ctrl+drag over tiles",
        "Remove the tiles from the selected group",
    ),
];
const DESELECT_ACTION: &str = "Deselect, back to the overview";
const SPACE_ACTION: &str = "Show the other view";
const ZOOM_ACTION: &str = "Zoom";
const RESET_ZOOM_ACTION: &str = "Reset zoom";
const FILES_HEADING: &str = "Files";
const MOUSE_HEADING: &str = "Mouse";
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
    let save = ui.ctx().format_shortcut(&save_automapper_shortcut());

    for (title, text) in GETTING_STARTED {
        let text = text.replace(SAVE_PLACEHOLDER, &save);
        show_section(ui, title, |ui| {
            ui.label(text);
        });
    }
    let heading = format!("{}. {DESTINATION_HEADING}", GETTING_STARTED.len() + 1);
    show_section(ui, &heading, show_destination);
}

fn show_destination(ui: &mut Ui) {
    ui.label(DESTINATION);
    paragraph_gap(ui);
    egui::Grid::new(PATHS_GRID_ID)
        .spacing(vec2(PATHS_COLUMN_GAP, PATHS_ROW_GAP))
        .show(ui, |ui| {
            for (system, path) in DDNET_AUTOMAP_PATHS {
                ui.label(system);
                code_block(ui, path);
                ui.end_row();
            }
        });
    paragraph_gap(ui);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(NAME_NOTE_START);
        ui.strong(NAME_NOTE_EMPHASIS);
        ui.label(NAME_NOTE_MIDDLE);
        code_chip(ui, NAME_NOTE_EXAMPLE_RULES);
        ui.label(NAME_NOTE_FOR);
        code_chip(ui, NAME_NOTE_EXAMPLE_IMAGE);
        ui.label(NAME_NOTE_END);
    });
}

fn show_concepts(ui: &mut Ui) {
    show_concept(ui, RULE_CONCEPT);
    show_section(ui, NEIGHBOR_STATES_HEADING, |ui| {
        ui.label(NEIGHBOR_STATES_INTRO);
        show_legend(ui);
    });

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

fn keybind_sections(ctx: &Context) -> [(&'static str, Vec<(String, &'static str)>); 4] {
    let keys = |shortcuts: &[KeyboardShortcut]| format_shortcuts(ctx, shortcuts);
    [
        (
            FILES_HEADING,
            vec![
                (keys(&[open_image_shortcut()]), OPEN_ACTION),
                (keys(&[open_automapper_shortcut()]), OPEN_AUTOMAPPER_ACTION),
                (keys(&[save_automapper_shortcut()]), SAVE_ACTION),
                (keys(&[export_shortcut()]), EXPORT_ACTION),
            ],
        ),
        (
            MOUSE_HEADING,
            MOUSE_ROWS
                .into_iter()
                .map(|(button, action)| (button.to_owned(), action))
                .collect(),
        ),
        (
            EDITING_HEADING,
            vec![
                (keys(&[undo_shortcut()]), UNDO_ACTION),
                (keys(&[redo_shortcut(), redo_alt_shortcut()]), REDO_ACTION),
                ("C".to_owned(), COPY_ACTION),
                ("V".to_owned(), PASTE_ACTION),
                ("1 / 2 / 3".to_owned(), NEIGHBOR_STATE_ACTION),
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
    show_section(ui, &format!("{ABOUT_HEADING} {AUTHOR_CREDIT}"), |ui| {
        ui.label(ABOUT_SUMMARY);
        external_link(ui, format!("{GITHUB_ICON} {GITHUB_LABEL}"), REPOSITORY);
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
                SentencePart::Link(label, url) => external_link(ui, *label, *url),
            };
        }
    });
}

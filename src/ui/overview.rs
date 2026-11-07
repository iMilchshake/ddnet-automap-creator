use egui::Ui;

use crate::model::project::Project;

const OVERVIEW_GRID_ID: &str = "overview";
const TILES_LABEL: &str = "Tiles";
const RULES_LABEL: &str = "Rules";
const GROUPS_LABEL: &str = "Groups";

pub struct Overview {
    pub tiles_with_rules: usize,
    pub rules: usize,
    pub deactivated_rules: usize,
    pub groups: usize,
}

impl Overview {
    pub fn new(project: &Project) -> Self {
        let rules = project.rules().len();

        Self {
            tiles_with_rules: project
                .configured_tiles()
                .filter(|(tile, _)| !project.is_deactivated(*tile))
                .count(),
            rules,
            deactivated_rules: project.all_rules().len() - rules,
            groups: project.groups().len(),
        }
    }

    pub fn rules_text(&self) -> String {
        match self.deactivated_rules {
            0 => self.rules.to_string(),
            deactivated => format!("{} (+{deactivated} deactivated)", self.rules),
        }
    }

    fn rows(&self) -> [(&'static str, String); 3] {
        [
            (TILES_LABEL, self.tiles_with_rules.to_string()),
            (RULES_LABEL, self.rules_text()),
            (GROUPS_LABEL, self.groups.to_string()),
        ]
    }
}

pub fn show(ui: &mut Ui, overview: &Overview) {
    egui::Grid::new(OVERVIEW_GRID_ID).show(ui, |ui| {
        for (label, value) in overview.rows() {
            ui.label(label);
            ui.label(value);
            ui.end_row();
        }
    });
}

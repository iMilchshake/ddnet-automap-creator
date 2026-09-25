#[derive(Debug, Clone, Copy)]
pub struct FileFilter {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
}

impl FileFilter {
    pub fn matches(&self, file_name: &str) -> bool {
        let Some((_stem, extension)) = file_name.rsplit_once('.') else {
            return false;
        };
        self.extensions
            .iter()
            .any(|known| known.eq_ignore_ascii_case(extension))
    }
}

pub const TILESET_IMAGE: FileFilter = FileFilter {
    name: "Tileset image",
    extensions: &["png", "jpg", "jpeg", "bmp"],
};

pub const RPP_SOURCE: FileFilter = FileFilter {
    name: "rpp source",
    extensions: &["r"],
};

pub const BLUEPRINT: FileFilter = FileFilter {
    name: "Blueprint",
    extensions: &["json"],
};

pub const RULES: FileFilter = FileFilter {
    name: "DDNet automapper rules",
    extensions: &["rules"],
};

pub const BUNDLE: FileFilter = FileFilter {
    name: "Bundle",
    extensions: &["zip"],
};

#[derive(Debug, Clone, Copy)]
pub struct FileFilter {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
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

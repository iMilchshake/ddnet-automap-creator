use crate::file_filter::{BLUEPRINT, TILESET_IMAGE};

#[test]
fn a_file_matches_its_filter_by_extension_ignoring_case() {
    assert!(TILESET_IMAGE.matches("grass_main.PNG"));
    assert!(BLUEPRINT.matches("/home/user/grass_main.json"));
    assert!(!TILESET_IMAGE.matches("grass_main.json"));
    assert!(!TILESET_IMAGE.matches("png"));
}

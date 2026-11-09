use crate::file_filter::{AUTOMAPPER, BLUEPRINT, TILESET_IMAGE};

#[test]
fn a_file_matches_its_filter_by_extension_ignoring_case() {
    assert!(TILESET_IMAGE.matches("grass_main.PNG"));
    assert!(BLUEPRINT.matches("/home/user/grass_main.json"));
    assert!(!TILESET_IMAGE.matches("grass_main.json"));
    assert!(!TILESET_IMAGE.matches("png"));
}

#[test]
fn the_automapper_filter_accepts_rules_and_json_but_no_images() {
    assert!(AUTOMAPPER.matches("grass_main.rules"));
    assert!(AUTOMAPPER.matches("grass_main.JSON"));
    assert!(!AUTOMAPPER.matches("grass_main.png"));
    assert!(!AUTOMAPPER.matches("grass_main.r"));
}

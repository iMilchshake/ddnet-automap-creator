use std::io::{Cursor, Read};

use crate::export::bundle::{self, Bundle};

const IMAGE: &str = "grass_main";

fn bundled() -> Vec<u8> {
    let bundle = Bundle {
        image_stem: IMAGE,
        rules: "[main]",
        source: "#include \"base.r\"",
        blueprint: "{\"image\": \"grass_main\"}",
    };

    bundle::archive(&bundle).unwrap()
}

#[test]
fn the_archive_holds_the_rules_the_source_and_the_blueprint() {
    let archive = zip::ZipArchive::new(Cursor::new(bundled())).unwrap();

    let mut names: Vec<String> = archive.file_names().map(ToOwned::to_owned).collect();
    names.sort();

    assert_eq!(
        names,
        ["grass_main.json", "grass_main.r", "grass_main.rules"]
    );
}

#[test]
fn every_entry_keeps_the_contents_it_was_given() {
    let mut archive = zip::ZipArchive::new(Cursor::new(bundled())).unwrap();

    for (name, expected) in [
        ("grass_main.rules", "[main]"),
        ("grass_main.r", "#include \"base.r\""),
        ("grass_main.json", "{\"image\": \"grass_main\"}"),
    ] {
        let mut contents = String::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        assert_eq!(contents, expected);
    }
}

#[test]
fn the_bundle_is_named_after_the_image() {
    assert_eq!(bundle::file_name(IMAGE), "grass_main.zip");
}

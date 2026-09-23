use std::io::{Cursor, Write};

use thiserror::Error;
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::export::r_source;

#[derive(Debug, Error)]
#[error("could not write the bundle: {0}")]
pub struct BundleError(String);

pub struct Bundle<'a> {
    pub image_stem: &'a str,
    pub rules: &'a str,
    pub source: &'a str,
    pub blueprint: &'a str,
}

pub fn file_name(image_stem: &str) -> String {
    format!("{image_stem}.zip")
}

pub fn archive(bundle: &Bundle) -> Result<Vec<u8>, BundleError> {
    let entries = [
        (r_source::output_file(bundle.image_stem), bundle.rules),
        (format!("{}.r", bundle.image_stem), bundle.source),
        (format!("{}.json", bundle.image_stem), bundle.blueprint),
    ];

    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));

    for (name, contents) in entries {
        writer.start_file(name, options).map_err(describe)?;
        writer.write_all(contents.as_bytes()).map_err(describe)?;
    }

    writer.finish().map(Cursor::into_inner).map_err(describe)
}

fn describe(error: impl std::fmt::Display) -> BundleError {
    BundleError(error.to_string())
}

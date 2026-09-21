use std::sync::mpsc::{Receiver, Sender, channel};

use thiserror::Error;

use crate::file_filter::FileFilter;

#[derive(Debug, Error)]
#[error("could not save `{name}`: {message}")]
pub struct SaveError {
    pub name: String,
    pub message: String,
}

type SaveResult = Result<String, SaveError>;

/// Cancelling sends nothing; it is not an outcome the UI reports.
pub struct FileSaver {
    sender: Sender<SaveResult>,
    receiver: Receiver<SaveResult>,
}

impl FileSaver {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    pub fn save_text(&self, filter: FileFilter, file_name: &str, contents: String) {
        spawn_dialog(self.sender.clone(), filter, file_name.to_owned(), contents);
    }

    pub fn poll(&self) -> Option<SaveResult> {
        self.receiver.try_recv().ok()
    }
}

impl Default for FileSaver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_dialog(
    sender: Sender<SaveResult>,
    filter: FileFilter,
    file_name: String,
    contents: String,
) {
    std::thread::spawn(move || {
        let dialog = rfd::FileDialog::new()
            .add_filter(filter.name, filter.extensions)
            .set_file_name(&file_name);
        let Some(path) = dialog.save_file() else {
            return;
        };

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or(file_name);
        let result = match std::fs::write(&path, contents) {
            Ok(()) => Ok(name),
            Err(error) => Err(SaveError {
                name,
                message: error.to_string(),
            }),
        };

        // The receiver is gone only if the app shut down.
        let _ = sender.send(result);
    });
}

#[cfg(target_arch = "wasm32")]
fn spawn_dialog(
    sender: Sender<SaveResult>,
    _filter: FileFilter,
    file_name: String,
    contents: String,
) {
    let result = match download(&file_name, &contents) {
        Ok(()) => Ok(file_name),
        Err(message) => Err(SaveError {
            name: file_name,
            message,
        }),
    };

    // The receiver is gone only if the app shut down.
    let _ = sender.send(result);
}

/// The browser has no save dialog rfd can drive, so the file leaves as a
/// download instead.
#[cfg(target_arch = "wasm32")]
fn download(file_name: &str, contents: &str) -> Result<(), String> {
    use eframe::wasm_bindgen::{JsCast as _, JsValue};

    let describe = |value: JsValue| format!("{value:?}");

    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(contents));

    let blob = web_sys::Blob::new_with_str_sequence(&parts).map_err(describe)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(describe)?;

    let document = web_sys::window()
        .ok_or_else(|| "the page has no window".to_owned())?
        .document()
        .ok_or_else(|| "the page has no document".to_owned())?;
    let anchor: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(describe)?
        .dyn_into()
        .map_err(|_| "the created element is not an anchor".to_owned())?;

    anchor.set_href(&url);
    anchor.set_download(file_name);
    anchor.click();

    web_sys::Url::revoke_object_url(&url).map_err(describe)
}

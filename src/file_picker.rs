use std::sync::mpsc::{Receiver, Sender, channel};

use thiserror::Error;

use crate::file_filter::FileFilter;

#[derive(Debug, Clone)]
pub struct PickedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum PickError {
    /// On the web the browser hands over the bytes, with no read step to fail.
    #[cfg(not(target_arch = "wasm32"))]
    #[error("could not read `{name}`: {message}")]
    Read { name: String, message: String },
}

type PickResult = Result<PickedFile, PickError>;

/// Cancelling sends nothing; it is not an outcome the UI reports.
pub struct FilePicker {
    filter: FileFilter,
    sender: Sender<PickResult>,
    receiver: Receiver<PickResult>,
}

impl FilePicker {
    pub fn new(filter: FileFilter) -> Self {
        let (sender, receiver) = channel();
        Self {
            filter,
            sender,
            receiver,
        }
    }

    pub fn open(&self) {
        spawn_dialog(self.sender.clone(), self.filter);
    }

    pub fn poll(&self) -> Option<PickResult> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_dialog(sender: Sender<PickResult>, filter: FileFilter) {
    std::thread::spawn(move || {
        let dialog = rfd::FileDialog::new().add_filter(filter.name, filter.extensions);
        let Some(path) = dialog.pick_file() else {
            return;
        };

        // The receiver is gone only if the app shut down.
        let _ = sender.send(read_file(&path));
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_file(path: &std::path::Path) -> PickResult {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    match std::fs::read(path) {
        Ok(bytes) => Ok(PickedFile { name, bytes }),
        Err(error) => Err(PickError::Read {
            name,
            message: error.to_string(),
        }),
    }
}

#[cfg(target_arch = "wasm32")]
fn spawn_dialog(sender: Sender<PickResult>, filter: FileFilter) {
    wasm_bindgen_futures::spawn_local(async move {
        let dialog = rfd::AsyncFileDialog::new().add_filter(filter.name, filter.extensions);
        let Some(handle) = dialog.pick_file().await else {
            return;
        };

        let picked = PickedFile {
            name: handle.file_name(),
            bytes: handle.read().await,
        };

        // The receiver is gone only if the app shut down.
        let _ = sender.send(Ok(picked));
    });
}

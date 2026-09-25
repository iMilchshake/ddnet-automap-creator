use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{Context, DroppedFileHandle};

use thiserror::Error;

use crate::file_filter::FileFilter;

#[derive(Debug, Clone)]
pub struct PickedFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum PickError {
    #[error("could not read `{name}`: {message}")]
    Read { name: String, message: String },
}

type PickResult = Result<PickedFile, PickError>;

/// Cancelling sends nothing; it is not an outcome the UI reports.
pub struct FilePicker {
    sender: Sender<PickResult>,
    receiver: Receiver<PickResult>,
}

impl FilePicker {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    pub fn open(&self, filter: FileFilter) {
        spawn_dialog(self.sender.clone(), filter);
    }

    pub fn read_dropped(&self, ctx: &Context, files: Vec<DroppedFileHandle>) {
        if files.is_empty() {
            return;
        }
        spawn_drop_reader(self.sender.clone(), ctx.clone(), files);
    }

    pub fn poll(&self) -> Option<PickResult> {
        self.receiver.try_recv().ok()
    }
}

fn picked_file(path: &Path, bytes: Result<Vec<u8>, String>) -> PickResult {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    match bytes {
        Ok(bytes) => Ok(PickedFile { name, bytes }),
        Err(message) => Err(PickError::Read { name, message }),
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
pub fn read_file(path: &Path) -> PickResult {
    picked_file(path, std::fs::read(path).map_err(|error| error.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_drop_reader(sender: Sender<PickResult>, ctx: Context, files: Vec<DroppedFileHandle>) {
    std::thread::spawn(move || {
        for file in files {
            let _ = sender.send(picked_file(file.path(), file.bytes()));
            ctx.request_repaint();
        }
    });
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

#[cfg(target_arch = "wasm32")]
fn spawn_drop_reader(sender: Sender<PickResult>, ctx: Context, files: Vec<DroppedFileHandle>) {
    wasm_bindgen_futures::spawn_local(async move {
        for file in files {
            let bytes = file.bytes_async().await;
            let _ = sender.send(picked_file(file.path(), bytes));
            ctx.request_repaint();
        }
    });
}

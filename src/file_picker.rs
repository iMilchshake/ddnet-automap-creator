use std::sync::mpsc::{Receiver, Sender, channel};

use thiserror::Error;

const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "bmp"];

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
    sender: Sender<PickResult>,
    receiver: Receiver<PickResult>,
}

impl FilePicker {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    pub fn open_image(&self) {
        spawn_dialog(self.sender.clone());
    }

    pub fn poll(&self) -> Option<PickResult> {
        self.receiver.try_recv().ok()
    }
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_dialog(sender: Sender<PickResult>) {
    std::thread::spawn(move || {
        let dialog = rfd::FileDialog::new().add_filter("Tileset image", &IMAGE_EXTENSIONS);
        let Some(path) = dialog.pick_file() else {
            return;
        };

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let result = match std::fs::read(&path) {
            Ok(bytes) => Ok(PickedFile { name, bytes }),
            Err(error) => Err(PickError::Read {
                name,
                message: error.to_string(),
            }),
        };

        // The receiver is gone only if the app shut down.
        let _ = sender.send(result);
    });
}

#[cfg(target_arch = "wasm32")]
fn spawn_dialog(sender: Sender<PickResult>) {
    wasm_bindgen_futures::spawn_local(async move {
        let dialog = rfd::AsyncFileDialog::new().add_filter("Tileset image", &IMAGE_EXTENSIONS);
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

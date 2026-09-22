use std::sync::mpsc::{Receiver, Sender, channel};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("rpp rejected the rule set:\n{0}")]
    Rpp(String),

    #[error("{0}")]
    Unavailable(String),
}

pub struct Compiled {
    pub file_name: String,
    pub rules: String,
}

type CompileResult = Result<Compiled, CompileError>;

/// Hands a generated `.r` source to rpp and collects the `.rules` it writes.
///
/// The run happens off the frame on both targets, so results arrive by `poll`
/// rather than from `start`.
pub struct RulesCompiler {
    sender: Sender<CompileResult>,
    receiver: Receiver<CompileResult>,
    running: bool,
}

impl RulesCompiler {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            running: false,
        }
    }

    pub fn start(&mut self, source: String, output_file: String) {
        self.running = true;
        spawn(self.sender.clone(), source, output_file);
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn poll(&mut self) -> Option<CompileResult> {
        let result = self.receiver.try_recv().ok()?;
        self.running = false;
        Some(result)
    }
}

impl Default for RulesCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{CompileError, CompileResult, Compiled};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::sync::mpsc::Sender;

    const BASE_SOURCE: &str = include_str!("../../vendor/rpp/rules++/base.r");
    const RPP_BINARY: &str = "rpp";

    pub fn spawn(sender: Sender<CompileResult>, source: String, output_file: String) {
        std::thread::spawn(move || {
            let _ = sender.send(run(&source, output_file));
        });
    }

    fn run(source: &str, output_file: String) -> CompileResult {
        let directory = work_directory()
            .map_err(|error| CompileError::Unavailable(format!("no work directory: {error}")))?;

        let result = stage(&directory, source).and_then(|()| invoke(&directory, output_file));
        let _ = std::fs::remove_dir_all(&directory);

        result
    }

    fn work_directory() -> std::io::Result<PathBuf> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or_default();

        let directory = std::env::temp_dir().join(format!("ddnet-automap-creator-{stamp}"));
        std::fs::create_dir_all(&directory)?;
        Ok(directory)
    }

    fn stage(directory: &Path, source: &str) -> Result<(), CompileError> {
        std::fs::write(directory.join("base.r"), BASE_SOURCE)
            .and_then(|()| std::fs::write(directory.join("input.r"), source))
            .map_err(|error| CompileError::Unavailable(format!("could not stage rpp: {error}")))
    }

    fn invoke(directory: &Path, output_file: String) -> CompileResult {
        let finished = Command::new(RPP_BINARY)
            .args(["-p", "input.r"])
            .current_dir(directory)
            .output()
            .map_err(|_| {
                CompileError::Unavailable(format!(
                    "`{RPP_BINARY}` is not on PATH, build it from vendor/rpp to compile rules \
                     on desktop"
                ))
            })?;

        collect(finished, directory, output_file)
    }

    fn collect(finished: Output, directory: &Path, output_file: String) -> CompileResult {
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&finished.stdout),
            String::from_utf8_lossy(&finished.stderr)
        );

        if !finished.status.success() {
            return Err(CompileError::Rpp(log));
        }

        match std::fs::read_to_string(directory.join(&output_file)) {
            Ok(rules) => Ok(Compiled {
                file_name: output_file,
                rules,
            }),
            Err(_) => Err(CompileError::Rpp(log)),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use native::spawn;

#[cfg(target_arch = "wasm32")]
mod web {
    use super::{CompileError, CompileResult, Compiled};
    use std::sync::mpsc::Sender;
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsCast as _, JsValue};

    #[wasm_bindgen(module = "/assets/rpp_bridge.js")]
    extern "C" {
        #[wasm_bindgen(catch)]
        async fn compile_rules(source: &str, output_file: &str) -> Result<JsValue, JsValue>;
    }

    pub fn spawn(sender: Sender<CompileResult>, source: String, output_file: String) {
        wasm_bindgen_futures::spawn_local(async move {
            let _ = sender.send(run(&source, output_file).await);
        });
    }

    async fn run(source: &str, output_file: String) -> CompileResult {
        let rules = match compile_rules(source, &output_file).await {
            Ok(rules) => rules,
            Err(error) => return Err(CompileError::Rpp(error_message(error))),
        };

        match rules.as_string() {
            Some(rules) => Ok(Compiled {
                file_name: output_file,
                rules,
            }),
            None => Err(CompileError::Unavailable(
                "rpp returned something other than text".to_owned(),
            )),
        }
    }

    fn error_message(error: JsValue) -> String {
        match error.dyn_ref::<js_sys::Error>() {
            Some(error) => error.message().into(),
            None => format!("{error:?}"),
        }
    }
}

#[cfg(target_arch = "wasm32")]
use web::spawn;

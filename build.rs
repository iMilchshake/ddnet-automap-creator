use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE_DIR: &str = "vendor/rpp";

fn main() {
    println!("cargo::rerun-if-changed={SOURCE_DIR}");

    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    assert!(
        Path::new(SOURCE_DIR).join("CMakeLists.txt").exists(),
        "{SOURCE_DIR} is empty, run: git submodule update --init"
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let build_dir = out_dir.join("rpp-build");
    let binary_dir = out_dir.join("bin");

    run(Command::new("cmake")
        .args(["-S", SOURCE_DIR, "-B"])
        .arg(&build_dir)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!(
            "-DCMAKE_RUNTIME_OUTPUT_DIRECTORY={}",
            binary_dir.display()
        ))
        .arg(format!(
            "-DCMAKE_RUNTIME_OUTPUT_DIRECTORY_RELEASE={}",
            binary_dir.display()
        )));

    run(Command::new("cmake").arg("--build").arg(&build_dir).args([
        "--config",
        "Release",
        "--parallel",
    ]));

    let binary = binary_dir.join(format!("rpp{}", std::env::consts::EXE_SUFFIX));
    println!("cargo::rustc-env=RPP_BINARY={}", binary.display());
}

fn run(command: &mut Command) {
    let program = command.get_program().to_owned();
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("could not run {program:?}: {error}, is cmake installed?"));

    assert!(status.success(), "{program:?} failed while building rpp");
}

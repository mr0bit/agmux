//! Embed the production browser application into agmux/agmux.exe.
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("Read companion UI assets") {
        let path = entry.expect("Read companion UI entry").path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

fn main() {
    let ui = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("src/companion_gui/ui");
    let dist = ui.join("dist");
    println!("cargo:rerun-if-changed={}", dist.display());
    assert!(
        dist.join("index.html").is_file(),
        "Build the application UI before compiling agmux: cd crates/agmux-cli/src/companion_gui/ui && npm ci && npm run build"
    );
    let mut files = Vec::new();
    collect(&dist, &mut files);
    files.sort();
    let mut generated = String::from("static COMPANION_ASSETS: &[(&str, &[u8])] = &[\n");
    for file in files {
        let name = file
            .strip_prefix(&dist)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        generated.push_str(&format!(
            "({name:?}, include_bytes!({:?})),\n",
            file.to_string_lossy()
        ));
    }
    generated.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("companion_assets.rs"),
        generated,
    )
    .expect("Write embedded companion manifest");
}

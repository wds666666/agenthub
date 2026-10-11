use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn visit(root: &Path, path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit(root, &path, files);
        } else {
            files.push(path.strip_prefix(root).unwrap().into());
        }
    }
}
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../skills/agenthub-manager")
        .canonicalize()
        .unwrap();
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = vec![];
    visit(&root, &root, &mut files);
    files.sort();
    let mut output = String::from("pub const FILES: &[(&str, &[u8])] = &[\n");
    for file in files {
        let relative = file.to_string_lossy().replace('\\', "/");
        output.push_str(&format!(
            "({relative:?}, include_bytes!({:?})),\n",
            root.join(file).to_str().unwrap()
        ));
    }
    output.push_str("];\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("manager_bundle.rs"),
        output,
    )
    .unwrap();
}

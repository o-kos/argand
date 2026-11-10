#[path = "src/build_metadata.rs"]
mod build_metadata;

fn main() {
    let manifest =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest.join("../..");
    let metadata = build_metadata::read(&root);
    for path in build_metadata::inputs(&root) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    for (name, value) in [
        ("HASH", metadata.hash),
        ("DATE", metadata.date),
        ("TAG", metadata.tag),
    ] {
        println!("cargo:rustc-env=ARGAND_GIT_{name}={value}");
    }
}

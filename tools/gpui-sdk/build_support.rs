//! Validate an explicitly selected SDK; never build or install one.
use std::{env, fs, path::PathBuf, process::Command};

pub fn validate() {
    println!("cargo:rerun-if-env-changed=GPUI_KIT_SDK");
    if env::var_os("CARGO_FEATURE_SOURCE_TOOLKIT").is_some() {
        return;
    }
    let directory = env::var_os("GPUI_KIT_SDK").map(PathBuf::from).unwrap_or_else(|| {
        panic!("No GPUI Kit SDK selected. Run python3 tools/gpui-sdk/sdk.py select <installed-sdk>, or use --features source-toolkit for a source build.")
    });
    let read = |name| {
        let path = directory.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("GPUI Kit SDK is missing or incomplete: {}: {error}. Install/select it explicitly; Cargo will not build it.", path.display())
        })
    };
    println!("cargo:rerun-if-changed={}", directory.join("lib").display());
    let compiler = Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("-vV")
        .output()
        .expect("query the compiler already selected by Cargo");
    assert!(
        compiler.status.success(),
        "cannot identify Cargo's compiler"
    );
    assert_eq!(
        read("rustc-version.txt").as_bytes(),
        compiler.stdout,
        "GPUI Kit SDK compiler mismatch; build/select a matching SDK explicitly"
    );
    assert_eq!(
        read("target.txt").trim(),
        env::var("TARGET").unwrap_or_default(),
        "GPUI Kit SDK target mismatch"
    );
    assert_eq!(
        read("kit-version.txt").trim(),
        "0.9.1",
        "GPUI Kit SDK toolkit mismatch"
    );
    for name in ["lib/libgpui_kit_sdk.so", "manifest.json"] {
        println!("cargo:rerun-if-changed={}", directory.join(name).display());
        assert!(
            directory.join(name).is_file(),
            "GPUI Kit SDK is incomplete: missing {name}; install/select it explicitly"
        );
    }
}

#!/usr/bin/env python3
"""Exercise Cargo-owned runtime paths with a shared, existing build directory.

Use the configured common build.build-dir; do not create or clear build caches.
Temporary packages have no dependencies and final target outputs stay local.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


MAIN = r'''fn main() {
    let marker = std::env::current_exe().unwrap().parent().unwrap().join("binary-marker");
    print!("{}", std::fs::read_to_string(marker).unwrap());
}
'''
TEST = r'''#[test]
fn current_identity() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    assert_eq!(root.canonicalize().unwrap(), std::env::current_dir().unwrap());
    let exe = std::env::var_os("CARGO_BIN_EXE_checkout-identity-regression").unwrap();
    let output = std::process::Command::new(&exe).output().unwrap();
    assert!(output.status.success());
    let expected = std::fs::read(root.join("fixture")).unwrap();
    assert_eq!(output.stdout, expected);
    println!("source={} executable={}", root.display(), std::path::Path::new(&exe).display());
}

#[test]
fn compile_time_negative_control() {
    assert_eq!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize().unwrap(),
        std::env::current_dir().unwrap());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_checkout-identity-regression"))
        .output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, std::fs::read("fixture").unwrap());
}
'''


def invoke(cargo, root, *args, succeeds=True):
    result = subprocess.run([cargo, *args], cwd=root, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout, end="")
    if (result.returncode == 0) != succeeds:
        raise AssertionError(f"unexpected exit {result.returncode}: {args}")
    return result.stdout


def main():
    cargo = os.environ.get("CARGO", "cargo")
    with tempfile.TemporaryDirectory(prefix="argand-checkout-identity-") as tmp:
        base = Path(tmp)
        old, current = base / "old checkout", base / "current checkout"
        old.mkdir()
        (old / "src").mkdir()
        (old / "tests").mkdir()
        (old / "Cargo.toml").write_text(
            '[package]\nname="checkout-identity-regression"\n'
            'version="0.0.0"\nedition="2024"\n'
            '[workspace]\nmembers=["."]\n', encoding="utf-8")
        (old / "src/main.rs").write_text(MAIN, encoding="utf-8")
        (old / "tests/identity.rs").write_text(TEST, encoding="utf-8")
        shutil.copytree(old, current, copy_function=shutil.copy2)
        for root, label in [(old, "old"), (current, "current")]:
            (root / "fixture").write_text(label, encoding="utf-8")
            (root / "target/debug").mkdir(parents=True)
            (root / "target/debug/binary-marker").write_text(label, encoding="utf-8")
        metadata = json.loads(invoke(cargo, current, "metadata", "--offline", "--no-deps",
                                     "--format-version=1"))
        build_dir = Path(metadata["build_directory"]).resolve()
        if build_dir.is_relative_to(base):
            raise AssertionError("configure the existing common build.build-dir first")
        invoke(cargo, old, "--version")
        invoke(cargo, old, "test", "--offline", "--", "--nocapture")
        artifacts = invoke(cargo, old, "test", "--offline", "--test", "identity",
                           "--no-run", "--message-format=json")
        messages = [json.loads(line) for line in artifacts.splitlines() if line.startswith("{")]
        harness = next(Path(message["executable"]) for message in messages
                       if message.get("reason") == "compiler-artifact"
                       and message["target"]["name"] == "identity")
        harness_hash = hashlib.sha256(harness.read_bytes()).hexdigest()
        for selection in [("--workspace",), ("-p", "checkout-identity-regression"),
                          ("--test", "identity")]:
            invoke(cargo, current, "test", "--offline", *selection, "--",
                   "current_identity", "--nocapture")
        # On affected Cargo versions the same artifact still contains old paths.
        control = subprocess.run([cargo, "test", "--offline", "--test", "identity", "--",
                                  "compile_time_negative_control", "--nocapture"],
                                 cwd=current, text=True, stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT)
        print("Compile-time negative control (failure demonstrates relocation bug):")
        print(control.stdout, end="")
        print(f"negative_control_exit={control.returncode}")
        print(f"harness={harness}; sha256={harness_hash}")
        print(f"common_build_dir={build_dir}")
        print("source_sha256=" + hashlib.sha256(TEST.encode()).hexdigest())
        shutil.rmtree(old)
        invoke(cargo, current, "test", "--offline", "--test", "identity", "--",
               "current_identity", "--nocapture")
        # Changing final outputs without moving sources must retain current paths too.
        alternate = current / "alternate final outputs"
        (alternate / "debug").mkdir(parents=True)
        (alternate / "debug/binary-marker").write_text("current", encoding="utf-8")
        invoke(cargo, current, "test", "--offline", "--target-dir", str(alternate),
               "--test", "identity", "--", "current_identity", "--nocapture")
        assert hashlib.sha256(harness.read_bytes()).hexdigest() == harness_hash, "harness changed"
        print("PASS: workspace/package/test selection, live/removed old checkout, final-output transition")


if __name__ == "__main__":
    main()

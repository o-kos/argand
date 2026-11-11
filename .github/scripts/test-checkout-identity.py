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
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let executable = std::path::Path::new(env!("CARGO_BIN_EXE_checkout-identity-regression"));
    let current_source = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let current_executable = std::env::var_os("CARGO_BIN_EXE_checkout-identity-regression").unwrap();
    assert!(source == std::path::Path::new(&current_source)
        && executable == std::path::Path::new(&current_executable),
        "STALE_COMPILE_TIME_PATHS: source={} executable={}", source.display(), executable.display());
}
'''


def invoke(cargo, root, *args, succeeds=True):
    result = subprocess.run([cargo, *args], cwd=root, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout, end="")
    if (result.returncode == 0) != succeeds:
        raise AssertionError(f"unexpected exit {result.returncode}: {args}")
    return result.stdout


def harness_identity(output):
    messages = [json.loads(line) for line in output.splitlines() if line.startswith("{")]
    artifact = next(message for message in messages
                    if message.get("reason") == "compiler-artifact"
                    and message["target"]["name"] == "identity")
    path = Path(artifact["executable"])
    return path, hashlib.sha256(path.read_bytes()).hexdigest(), artifact["fresh"]


def main():
    # Resolve outside the temporary packages; never let their cwd select another
    # rustup default instead of the repository's installed toolchain.
    for variable in ["CARGO", "RUSTC", "RUSTDOC"]:
        value = os.environ.get(variable, "")
        if not value or not Path(value).is_absolute() or Path(value).resolve().stem == "rustup":
            raise AssertionError(f"set {variable} to an absolute installed toolchain executable")
    cargo = os.environ["CARGO"]
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
        output = invoke(cargo, old, "test", "--offline", "--message-format=json",
                        "--", "--nocapture")
        previous = harness_identity(output)
        observed_reuse = 0

        def transition(*args):
            nonlocal previous, observed_reuse
            output = invoke(cargo, current, "test", "--offline", "--message-format=json",
                            *args, "--", "current_identity", "--nocapture")
            active = harness_identity(output)
            reused = active[0:2] == previous[0:2] and active[2]
            if active[2] and active[0] == previous[0]:
                assert active[1] == previous[1], "fresh harness content changed"
            observed_reuse += reused
            print(f"active_harness={active[0]}; sha256={active[1]}; fresh={active[2]}; reused={reused}")
            previous = active

        for selection in [("--workspace",), ("-p", "checkout-identity-regression"),
                          ("--test", "identity")]:
            transition(*selection)
        control = subprocess.run([cargo, "test", "--offline", "--test", "identity", "--",
                                  "compile_time_negative_control", "--nocapture"],
                                 cwd=current, text=True, stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT)
        print("Compile-time negative control:")
        print(control.stdout, end="")
        if control.returncode:
            assert control.returncode == 101 and "STALE_COMPILE_TIME_PATHS:" in control.stdout, (
                "negative control failed for an unexpected Cargo/test error")
            assert observed_reuse, "stale compile-time paths without observed harness reuse"
        print(f"negative_control_exit={control.returncode}; observed_reuse={observed_reuse}")
        print(f"common_build_dir={build_dir}")
        print("source_sha256=" + hashlib.sha256(TEST.encode()).hexdigest())
        shutil.rmtree(old)
        transition("--test", "identity")
        # The new marker must reject the previous current/target executable too.
        alternate = current / "alternate final outputs"
        (alternate / "debug").mkdir(parents=True)
        (alternate / "debug/binary-marker").write_text("alternate", encoding="utf-8")
        (current / "fixture").write_text("alternate", encoding="utf-8")
        transition("--target-dir", str(alternate), "--test", "identity")
        print("PASS: workspace/package/test selection, live/removed old checkout, final-output transition")


if __name__ == "__main__":
    main()

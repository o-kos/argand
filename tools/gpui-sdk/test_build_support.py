"""Exercise missing SDK and Cargo freshness without toolkit/dependency compilation."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("sdk", HERE / "sdk.py")
sdk = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sdk)
sdk.safe_selection(Path.cwd())
configuration = __import__("tomllib").loads((sdk.ROOT / ".cargo/config.toml").read_text())
installed = Path(configuration["env"]["GPUI_KIT_SDK"])
results = []
with tempfile.TemporaryDirectory(prefix="argand-sdk-validation-") as temporary:
    root = Path(temporary)
    (root / "src").mkdir()
    (root / "Cargo.toml").write_text('[package]\nname="argand-sdk-validation-control"\nversion="0.0.0"\nedition="2024"\n[features]\nsource-toolkit=[]\n[workspace]\n')
    (root / "src/main.rs").write_text("fn main() {}\n")
    (root / "build.rs").write_text('#[path=' + json.dumps(str(HERE / "build_support.rs")) + ']\nmod sdk;\nfn main(){sdk::validate();}\n')
    fixture = root / "sdk"
    (fixture / "lib").mkdir(parents=True)
    for name in ["rustc-version.txt", "target.txt", "kit-version.txt", "manifest.json"]:
        (fixture / name).write_bytes((installed / name).read_bytes())
    library = fixture / "lib/libgpui_kit_sdk.so"
    library.write_bytes(b"existence-only validator fixture")
    environment = os.environ.copy()
    environment["GPUI_KIT_SDK"] = str(fixture)

    def run(name, expected, text="", extra=()):
        command = ["cargo", "build", "--offline", *extra]
        result = subprocess.run(command, cwd=root, env=environment, capture_output=True, text=True)
        assert (result.returncode == 0) == expected, result.stderr
        assert text in result.stderr, result.stderr
        results.append(dict(case=name,exit=result.returncode,stderr=result.stderr))

    run("valid fixture", True)
    library.unlink()
    run("removed artifact invalidates fresh Cargo build", False, "SDK is incomplete")
    library.write_bytes(b"existence-only validator fixture")
    (fixture / "rustc-version.txt").write_text("another compiler\n")
    run("compiler mismatch", False, "SDK compiler mismatch")
    environment.pop("GPUI_KIT_SDK")
    run("missing selection", False, "No GPUI Kit SDK selected")
    run("source feature needs no SDK", True, extra=["--features", "source-toolkit"])
Path("/tmp/argand-248-build-support-results.json").write_text(json.dumps(results, indent=2))
print("5 build-support regressions passed; only dependency-free fixture code compiled")

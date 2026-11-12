"""Bounded installer safety regressions; never invoke Rust or rebuild dependencies."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("sdk", Path(__file__).with_name("sdk.py"))
sdk = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sdk)


class SdkSafety(unittest.TestCase):
    def test_native_collection_excludes_system_catalog_and_unrequested_outputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            out, system = root / "out", root / "system"
            out.mkdir()
            system.mkdir()
            for p in [out / "librequested.a", out / "libunrelated.a", system / "librequested.so"]:
                p.write_bytes(b"fixture")
            records = [{"reason": "build-script-executed", "out_dir": str(out),
                        "linked_paths": ["native=" + str(out), "native=" + str(system)],
                        "linked_libs": ["static=requested"]}]
            self.assertEqual(sdk.collect_artifacts(records), {out / "librequested.a"})

    def test_verify_rejects_manifest_identity_and_locked_graph_before_artifact_reads(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "incorrect-id"
            root.mkdir()
            manifest = dict(format=1, compiler="fixture", target="fixture", kit="0.9.1",
                            lock_sha256="wrong", artifacts={}, build_id="incorrect-id")
            (root / "manifest.json").write_text(json.dumps(manifest))
            with patch.object(sdk, "approved_graph"), patch.object(sdk, "compiler", return_value=("fixture", "fixture")):
                with self.assertRaisesRegex(ValueError, "identity"):
                    sdk.verify(root)

    def test_select_does_not_replace_user_configuration(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / ".cargo").mkdir()
            config = root / ".cargo/config.toml"
            config.write_text("# owner's configuration\n")
            with patch.object(sdk, "ROOT", root), patch.object(sdk, "verify", return_value=(root, {})):
                with self.assertRaisesRegex(ValueError, "overwrite"):
                    sdk.select(root)
            self.assertEqual(config.read_text(), "# owner's configuration\n")

    def test_missing_installed_pin_is_rejected_without_invoking_proxy(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel="1.98.1"\ncomponents=["rustfmt","clippy"]\n')
            rustup = root / "rustup"
            rustup.write_text("")
            (root / "settings.toml").write_text('default_toolchain="1.98.1-x86_64-unknown-linux-gnu"\n')
            with patch.object(sdk, "ROOT", root), patch.dict(os.environ, {"RUSTUP_HOME": str(root)}, clear=True), patch.object(sdk.shutil, "which", return_value=str(rustup)), patch.object(sdk.subprocess, "check_output") as invoke:
                with self.assertRaises((ValueError, OSError)):
                    sdk.safe_selection(root)
                invoke.assert_not_called()

    def test_hardlinked_rustup_proxy_is_checked_without_execution(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel="1.98.1"\ncomponents=["rustfmt","clippy"]\n')
            rustup = root / "rustup"
            rustup.write_text("must never execute")
            os.link(rustup, root / "rustc")
            os.link(rustup, root / "cargo")
            (root / "settings.toml").write_text('default_toolchain="1.98.1-x86_64-unknown-linux-gnu"\n')
            with patch.object(sdk, "ROOT", root), patch.dict(os.environ, {"RUSTUP_HOME": str(root)}, clear=True), patch.object(sdk.shutil, "which", side_effect=lambda name: str(root / name)), patch.object(sdk.subprocess, "check_output") as invoke:
                with self.assertRaises((ValueError, OSError)):
                    sdk.safe_selection(root)
                invoke.assert_not_called()


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Check direct executable selection and fail-fast behavior of the actual hook."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="argand-hook-") as tmp:
    base = Path(tmp)
    log = base / "calls"
    env = os.environ.copy()
    env["HOOK_TEST_LOG"] = str(log)
    for variable, name in [("CARGO", "cargo"), ("CARGO_FMT", "cargo-fmt"),
                           ("CARGO_CLIPPY", "cargo-clippy")]:
        tool = base / name
        tool.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
with open(os.environ["HOOK_TEST_LOG"], "a") as log:
    log.write(json.dumps([pathlib.Path(sys.argv[0]).name, *sys.argv[1:]]) + "\\n")
sys.exit(23 if os.environ.get("HOOK_TEST_FAIL") == sys.argv[1] else 0)
''', encoding="utf-8")
        tool.chmod(0o755)
        env[variable] = str(tool)
    command = ["bash", str(root / ".githooks/pre-push")]
    subprocess.run(command, cwd=root, env=env, check=True)
    expected = [["cargo-fmt", "fmt", "--all", "--", "--check"],
                ["cargo-clippy", "clippy", "--all-targets", "--locked"],
                ["cargo", "test", "--locked"]]
    assert [json.loads(line) for line in log.read_text().splitlines()] == expected
    log.unlink()
    inherited = env.copy()
    del inherited["CARGO_FMT"], inherited["CARGO_CLIPPY"]
    subprocess.run(command, cwd=root, env=inherited, check=True)
    assert [json.loads(line) for line in log.read_text().splitlines()] == [
        ["cargo", *args[1:]] for args in expected]
    log.unlink()
    env["HOOK_TEST_FAIL"] = "clippy"
    result = subprocess.run(command, cwd=root, env=env)
    assert result.returncode != 0
    assert [json.loads(line) for line in log.read_text().splitlines()] == expected[:2]
print("PASS: direct tool selection, exact gate arguments and fail-fast hook")

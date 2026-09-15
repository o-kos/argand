"""Exercise real APT with signed local repositories; never install host packages."""

from email.utils import formatdate
import hashlib
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("install-ubuntu-packages.sh").resolve()
PACKAGE = "argand-ci-fixture"


def run(*args, **kwargs):
    return subprocess.run(args, capture_output=True, text=True, check=True, **kwargs)


class UbuntuPackagesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = tempfile.TemporaryDirectory()
        cls.root = Path(cls.fixture.name)
        cls.root.chmod(0o755)
        cls.key_home = cls.root / "gnupg"
        cls.key_home.mkdir(mode=0o700)
        run("gpg", "--homedir", str(cls.key_home), "--batch", "--pinentry-mode",
            "loopback", "--passphrase", "", "--quick-generate-key",
            "Argand CI fixture <fixture@example.invalid>", "rsa2048", "sign", "0")
        cls.keyring = cls.root / "fixture.gpg"
        run("gpg", "--homedir", str(cls.key_home), "--batch", "--output",
            str(cls.keyring), "--export")
        package = cls.root / "package"
        (package / "DEBIAN").mkdir(parents=True)
        (package / "DEBIAN/control").write_text(
            f"Package: {PACKAGE}\nVersion: 1.0\nArchitecture: all\n"
            "Maintainer: CI <fixture@example.invalid>\nDescription: APT test fixture\n"
        )
        cls.deb = cls.root / "fixture.deb"
        run("dpkg-deb", "--build", str(package), str(cls.deb))

    @classmethod
    def tearDownClass(cls):
        run("gpgconf", "--homedir", str(cls.key_home), "--kill", "gpg-agent")
        cls.fixture.cleanup()

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.work = Path(self.temporary.name)
        self.work.chmod(0o755)
        for name in ("sources", "empty", "lists/partial", "archives/partial", "scratch"):
            (self.work / name).mkdir(parents=True)
        (self.work / "status").touch()
        self.config = self.work / "apt.conf"
        self.config.write_text(f'''
Dir::Etc::parts "{self.work}/empty";
Dir::Etc::main "/dev/null";
Dir::Etc::sourcelist "{self.work}/ubuntu.sources";
Dir::Etc::sourceparts "{self.work}/sources";
Dir::State::status "{self.work}/status";
Dir::State::lists "{self.work}/lists";
Dir::State::extended_states "{self.work}/extended_states";
Dir::Cache::archives "{self.work}/archives";
Dir::Cache::pkgcache "";
Dir::Cache::srcpkgcache "";
Dir::Log "{self.work}";
APT::Get::Download-Only "true";
Acquire::Retries "0";
Acquire::http::Timeout "2";
''')
        self.env = dict(os.environ, APT_CONFIG=str(self.config), LC_ALL="C",
                        TMPDIR=str(self.work / "scratch"))
        self.repo = self.repository("ubuntu")
        self.sources = self.work / "ubuntu.sources"
        self.write_source(self.sources, self.repo.as_uri())

    def repository(self, name):
        repo = self.work / name
        repo.mkdir()
        shutil.copyfile(self.deb, repo / "fixture.deb")
        data = self.deb.read_bytes()
        packages = (
            f"Package: {PACKAGE}\nVersion: 1.0\nArchitecture: all\n"
            "Maintainer: CI <fixture@example.invalid>\n"
            "Description: APT test fixture\nFilename: ./fixture.deb\n"
            f"Size: {len(data)}\nSHA256: {hashlib.sha256(data).hexdigest()}\n\n"
        ).encode()
        (repo / "Packages").write_bytes(packages)
        self.sign_repository(repo)
        return repo

    def sign_repository(self, repo):
        packages = (repo / "Packages").read_bytes()
        (repo / "Release").write_text(
            f"Date: {formatdate(usegmt=True)}\nSHA256:\n"
            f" {hashlib.sha256(packages).hexdigest()} {len(packages)} Packages\n"
        )
        run("gpg", "--homedir", str(self.key_home), "--batch", "--yes",
            "--digest-algo", "SHA256", "--clearsign", "--output",
            str(repo / "InRelease"), str(repo / "Release"))

    def write_source(self, path, uri):
        path.write_text(f"Types: deb\nURIs: {uri}\nSuites: ./\nSigned-By: {self.keyring}\n")

    def apt(self, *args):
        return subprocess.run(["apt-get", *args], env=self.env, capture_output=True,
                              text=True, check=False)

    def install(self, source=None, package=PACKAGE):
        result = subprocess.run(["bash", str(SCRIPT), str(source or self.sources), package],
                                env=self.env, capture_output=True, text=True, check=False)
        self.assertEqual(list((self.work / "scratch").iterdir()), [], "temporary lists leaked")
        return result

    def assert_failed(self, result, message):
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(message, result.stdout + result.stderr)
        self.assertEqual(list((self.work / "archives").glob("*.deb")), [])

    def test_broken_unrelated_index_is_ignored_without_modifying_sources(self):
        other = self.repository("chrome")
        (other / "Packages").write_bytes(b"broken index\n")
        extra = self.work / "sources/chrome.sources"
        self.write_source(extra, other.as_uri())
        original = {p: p.read_bytes() for p in (self.sources, extra)}
        baseline = self.apt("-o", "APT::Update::Error-Mode=any", "update")
        self.assertNotEqual(baseline.returncode, 0)
        self.assertIn("Hash Sum mismatch", baseline.stdout + baseline.stderr)
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn(str(other), result.stdout + result.stderr)
        self.assertIn("Download complete and in download only mode", result.stdout)
        self.assertEqual(original, {p: p.read_bytes() for p in original})

    def test_cached_third_party_packages_are_not_install_candidates(self):
        other = self.repository("third-party")
        self.write_source(self.work / "sources/third-party.sources", other.as_uri())
        (self.repo / "Packages").write_bytes(b"")
        self.sign_repository(self.repo)
        cached = self.apt("update")
        self.assertEqual(cached.returncode, 0, cached.stderr)
        self.assert_failed(self.install(), "Unable to locate package")

    def test_install_does_not_use_runner_archive_cache(self):
        blocked = self.work / "blocked-archives"
        blocked.write_text("The runner archive cache must remain untouched.\n")
        with self.config.open("a") as config:
            config.write(f'Dir::Cache::archives "{blocked}";\n')
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(blocked.read_text(), "The runner archive cache must remain untouched.\n")

    def test_required_package_hash_failure_is_fatal(self):
        (self.repo / "fixture.deb").write_bytes(b"corrupt package")
        self.assert_failed(self.install(), "Hash Sum mismatch")

    def test_required_index_hash_failure_is_fatal(self):
        (self.repo / "Packages").write_bytes(b"broken index\n")
        self.assert_failed(self.install(), "Hash Sum mismatch")

    def test_unsigned_required_repository_is_fatal(self):
        (self.repo / "InRelease").unlink()
        self.assert_failed(self.install(), "not signed")

    def test_required_connection_failure_cannot_use_cached_indexes(self):
        cached = self.apt("update")
        self.assertEqual(cached.returncode, 0, cached.stderr)
        with socket.socket() as unavailable:
            unavailable.bind(("127.0.0.1", 0))
            port = unavailable.getsockname()[1]
            self.write_source(self.sources, f"http://127.0.0.1:{port}")
            self.assert_failed(self.install(), "Failed to fetch")

    def test_missing_package_is_fatal(self):
        self.assert_failed(self.install(package="argand-ci-missing"), "Unable to locate package")

    def test_missing_or_empty_source_is_fatal(self):
        for source in (self.work / "missing.sources", self.work / "empty.sources"):
            if source.name == "empty.sources":
                source.touch()
            with self.subTest(source=source):
                self.assert_failed(self.install(source=source), "missing or empty")


if __name__ == "__main__":
    unittest.main()

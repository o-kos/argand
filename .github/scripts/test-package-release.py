"""Validate release packaging without compiling Rust or starting a GUI."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('package-release.py')
SPEC = importlib.util.spec_from_file_location('packaging', SCRIPT)
packaging = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(packaging)


def pe_fixture(dll, delay=False):
    data = bytearray(1024)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 0x3c, 128)
    data[128:132] = b'PE\0\0'
    struct.pack_into('<HH', data, 132, 0x8664, 1)
    struct.pack_into('<H', data, 148, 240)
    struct.pack_into('<H', data, 152, 0x20b)
    struct.pack_into('<II', data, 264 + (13 if delay else 1) * 8, 0x1000, 64)
    struct.pack_into('<IIII', data, 400, 512, 0x1000, 512, 512)
    if delay:
        struct.pack_into('<II', data, 512, 1, 0x1080)
    else:
        struct.pack_into('<I', data, 524, 0x1080)
    encoded = dll.encode('ascii') + b'\0'
    data[640:640 + len(encoded)] = encoded
    return data


class PackagingTests(unittest.TestCase):
    def setUp(self):
        lucide = SCRIPT.with_name('fixtures') / 'LICENSE-LUCIDE'
        self.notice_patch = patch.object(packaging, 'rust_notices', return_value=(
            [(Path('gpui-kit-assets-0.9.1/LICENSE-LUCIDE'), lucide),
             (Path('gpui-kit-0.9.1/LICENSE'), packaging.ROOT / 'LICENSE'),
             (Path('taffy-0.15.0/LICENSE'), SCRIPT.with_name('notices') / 'taffy-0.15.0.LICENSE')],
            [{'name': 'gpui-kit-assets', 'version': '0.9.1', 'license': 'ISC AND MIT'},
             {'name': 'gpui-kit', 'version': '0.9.1', 'license': 'MIT'}]))
        self.notice_patch.start()
        self.addCleanup(self.notice_patch.stop)

    def test_locked_asset_and_dependency_notices_are_collected(self):
        self.notice_patch.stop()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            packages = []
            for name in ('gpui-kit-assets', 'gpui-kit'):
                package = root / 'registry/src/fixture' / (name + '-0.9.1')
                package.mkdir(parents=True)
                (package / 'Cargo.toml').write_text('[package]\nlicense = "MIT"\n')
                archive = root / 'registry/cache/fixture' / (name + '-0.9.1.crate')
                archive.parent.mkdir(parents=True, exist_ok=True)
                archive.write_bytes(b'fixture package archive')
                checksum = packaging.hashlib.sha256(archive.read_bytes()).hexdigest()
                if name == 'gpui-kit-assets':
                    shutil.copyfile(SCRIPT.with_name('fixtures') / 'LICENSE-LUCIDE', package / 'LICENSE-LUCIDE')
                else:
                    shutil.copyfile(packaging.ROOT / 'LICENSE', package / 'LICENSE')
                packages.append({'name': name, 'version': '0.9.1', 'source': 'registry+fixture',
                                 'checksum': checksum})
            fake_lock = root / 'Cargo.lock'
            fake_lock.write_text(''.join('[[package]]\n' + ''.join(f'{key} = "{value}"\n' for key,value in entry.items()) for entry in packages))
            with patch.object(packaging, 'ROOT', root):
                files, inventory = packaging.rust_notices(root)
                paths = {relative.as_posix(): source for relative, source in files}
                lucide = paths['gpui-kit-assets-0.9.1/LICENSE-LUCIDE'].read_text()
                self.assertIn('Lucide Icons and Contributors', lucide)
                self.assertIn('Cole Bemis', lucide)
                self.assertTrue(any(entry['name'] == 'gpui-kit' for entry in inventory))
                self.assertIn('gpui-kit-0.9.1/LICENSE', paths)
                archive.write_bytes(b'wrong archive')
                with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                    packaging.rust_notices(root)
                archive.write_bytes(b'fixture package archive')
                (root / 'registry/src/fixture/gpui-kit-assets-0.9.1/LICENSE-LUCIDE').unlink()
                with self.assertRaisesRegex(ValueError, 'missing locked GPUI'):
                    packaging.rust_notices(root)
        self.notice_patch.start()

    def test_supplemental_notice_for_published_crate_without_license(self):
        with (packaging.ROOT / 'Cargo.lock').open('rb') as locked:
            package = next(item for item in packaging.tomllib.load(locked)['package']
                           if item['name'] == 'taffy')
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp)
            vcs = source / '.cargo_vcs_info.json'
            vcs.write_text('{"git":{"sha1":"eb6450694f33e2b7413e40677f230ff63f0fc194"}}')
            self.assertFalse((source / 'LICENSE').exists())
            notices = packaging.supplemental_notices(package, source)
            license_file = next(path for relative, path in notices if relative.name == 'LICENSE')
            text = license_file.read_text()
            self.assertIn('Copyright (c) 2018 Visly Inc.', text)
            self.assertIn('Copyright (c) 2026 Taffy Authors', text)
            self.assertEqual(packaging.supplemental_notices(dict(package, version='0.15.1'), source), [])
            with self.assertRaisesRegex(ValueError, 'package mismatch'):
                packaging.supplemental_notices(dict(package, checksum='incorrect'), source)
            vcs.write_text('{"git":{"sha1":"incorrect"}}')
            with self.assertRaisesRegex(ValueError, 'revision mismatch'):
                packaging.supplemental_notices(package, source)

    def test_pinned_notice_bytes_survive_windows_style_checkout(self):
        notice_path = Path('.github/scripts/notices/taffy-0.15.0.LICENSE')
        expected = 'f97daf1a0124413dccf399a4e6626b4b74acd05282f80b6d64ac82225650b77a'
        original = (packaging.ROOT / notice_path).read_bytes()
        self.assertEqual(packaging.hashlib.sha256(original).hexdigest(), expected)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            def git(*args):
                return subprocess.run(['git', '-C', str(root), *args], check=True,
                                      text=True, capture_output=True)
            git('init', '--quiet')
            git('config', 'core.autocrlf', 'true')
            target = root / notice_path
            target.parent.mkdir(parents=True)
            target.write_bytes(original)
            git('add', '--', notice_path.as_posix())
            target.unlink()
            git('checkout-index', '--all')
            self.assertNotEqual(packaging.hashlib.sha256(target.read_bytes()).hexdigest(), expected)
            shutil.copyfile(packaging.ROOT / '.gitattributes', root / '.gitattributes')
            git('add', '--', '.gitattributes')
            target.unlink()
            git('checkout-index', '--all')
            self.assertEqual(packaging.hashlib.sha256(target.read_bytes()).hexdigest(), expected)
            attributes = git('check-attr', 'text', 'eol', '--', notice_path.as_posix()).stdout
            self.assertIn('text: set', attributes)
            self.assertIn('eol: lf', attributes)

    def test_pe_imports_accept_os_and_reject_non_os_in_both_tables(self):
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / 'fixture.exe'
            for delay in (False, True):
                for name in ('KERNEL32.dll', 'gdiplus.dll', 'api-ms-win-core-synch-l1-2-0.dll'):
                    binary.write_bytes(pe_fixture(name, delay))
                    self.assertEqual(packaging.check_windows(binary), [name.lower()])
                for name in ('VCRUNTIME140.dll', 'MSVCP140.dll', 'gpui_kit_sdk.dll'):
                    binary.write_bytes(pe_fixture(name, delay))
                    with self.assertRaisesRegex(ValueError, 'non-OS DLL'):
                        packaging.check_windows(binary)

    def test_windows_archives_keep_names_and_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binaries = root / 'build with spaces'
            binaries.mkdir()
            for name in ('argand', 'aspec'):
                (binaries / f'{name}.exe').write_bytes(pe_fixture('KERNEL32.dll'))
            archives = packaging.package(binaries, root / 'output', packaging.TARGETS[1],
                                         '0.2.0', 'fixture-commit', smoke=False)
            for product, archive in zip(('argand', 'aspec'), archives):
                self.assertEqual(archive.name, f'{product}-v0.2.0-{packaging.TARGETS[1]}.zip')
                with packaging.zipfile.ZipFile(archive) as zipped:
                    stem = archive.stem
                    self.assertIn(f'{stem}/{product}.exe', zipped.namelist())
                    self.assertIn(f'{stem}/licenses/rust/INDEX.json', zipped.namelist())
                    lucide = zipped.read(f'{stem}/licenses/rust/gpui-kit-assets-0.9.1/LICENSE-LUCIDE').decode()
                    self.assertIn('Lucide Icons and Contributors', lucide)
                    self.assertIn('Cole Bemis', lucide)
                    self.assertIn(f'{stem}/licenses/rust/gpui-kit-0.9.1/LICENSE', zipped.namelist())
                    taffy = zipped.read(f'{stem}/licenses/rust/taffy-0.15.0/LICENSE').decode()
                    self.assertIn('2018 Visly Inc.', taffy)
                    self.assertIn('2026 Taffy Authors', taffy)
                    if product == 'aspec':
                        font = zipped.read(f'{stem}/licenses/DejaVuSans.LICENSE').decode()
                        self.assertIn('Bitstream', font)
                    identity = json.loads(zipped.read(f'{stem}/BUILD-INFO.json'))
                    self.assertEqual(identity['commit'], 'fixture-commit')
                    self.assertEqual(identity['native_dependencies'], ['kernel32.dll'])

    def test_existing_output_is_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'output'
            output.mkdir()
            marker = output / 'preserve-me'
            marker.write_text('unchanged')
            with self.assertRaises(FileExistsError):
                packaging.package(Path(temp), output, packaging.TARGETS[0], '0.2.0', 'fixture')
            self.assertEqual(marker.read_text(), 'unchanged')

    @unittest.skipUnless(os.name == 'posix' and Path('/bin/true').exists(), 'Linux ELF fixture')
    def test_linux_archive_launcher_and_bundled_closure(self):
        with tempfile.TemporaryDirectory(prefix='argand package with spaces ') as temp:
            root = Path(temp)
            binaries = root / 'binaries'
            binaries.mkdir()
            for name in ('argand', 'aspec'):
                shutil.copy2('/bin/true', binaries / name)
            archives = packaging.package(binaries, root / 'output', packaging.TARGETS[0],
                                         '0.2.0', 'fixture-commit', smoke=False)
            with tarfile.open(archives[0]) as archive:
                archive.extractall(root / 'unpacked', filter='data')
            app = root / 'unpacked' / archives[0].name.removesuffix('.tar.gz')
            subprocess.run([str(app / 'argand'), 'argument with spaces'], check=True)
            self.assertTrue((app / 'licenses').is_dir())
            self.assertIn('Cole Bemis', (app / 'licenses/rust/gpui-kit-assets-0.9.1/LICENSE-LUCIDE').read_text())
            self.assertTrue((app / 'licenses/rust/gpui-kit-0.9.1/LICENSE').is_file())
            taffy = (app / 'licenses/rust/taffy-0.15.0/LICENSE').read_text()
            self.assertIn('2018 Visly Inc.', taffy)
            self.assertIn('2026 Taffy Authors', taffy)
            with tarfile.open(archives[1]) as archive:
                font_path = archives[1].name.removesuffix('.tar.gz') + '/licenses/DejaVuSans.LICENSE'
                self.assertIn(b'Bitstream', archive.extractfile(font_path).read())
            for name in packaging.DLOPEN_LIBS:
                self.assertTrue((app / 'lib' / name).is_file(), name)
            for library in (app / 'lib').iterdir():
                self.assertFalse(library.is_symlink())
                self.assertFalse(packaging.BASE_LIBS.match(library.name))
            environment = dict(os.environ, LD_LIBRARY_PATH=str(app / 'lib'))
            for binary in [app / 'argand-bin', *(app / 'lib').iterdir()]:
                linked = subprocess.check_output(['ldd', str(binary)], env=environment, text=True)
                self.assertNotIn('not found', linked)
                for line in linked.splitlines():
                    match = packaging.re.match(r'\s*(\S+) => (/\S+) \(', line)
                    if match and not packaging.BASE_LIBS.match(match[1]):
                        self.assertEqual(Path(match[2]).parent, app / 'lib')

    def test_release_notes_and_version_rejections_without_cargo(self):
        notes = SCRIPT.with_name('release-notes.sh')
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            scripts = root / '.github/scripts'
            scripts.mkdir(parents=True)
            shutil.copyfile(notes, scripts / notes.name)
            (root / 'Cargo.toml').write_text('[workspace.package]\nversion = "0.2.0"\n')
            changelog = root / 'CHANGELOG.md'
            changelog.write_text('## [Unreleased]\n\n## [0.2.0] - 2026-11-15\n\n### Added\n\n- Fixture\n\n## [0.1.0]\nOld\n')
            for tag, succeeds in (('v0.2.0', True), ('v0.1.0', False), ('0.2.0', False),
                                  ('v0.2.0-rc1', False), ('v0.2.0/../../file', False)):
                result = subprocess.run(['bash', str(scripts / notes.name), tag],
                                        text=True, capture_output=True)
                self.assertEqual(result.returncode == 0, succeeds, result.stderr)
                if succeeds:
                    self.assertEqual(result.stdout, '### Added\n\n- Fixture\n')
            changelog.write_text('## [Unreleased]\n')
            result = subprocess.run(['bash', str(scripts / notes.name), 'v0.2.0'], capture_output=True)
            self.assertNotEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()

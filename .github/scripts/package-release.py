#!/usr/bin/env python3
"""Package trusted release builds; shared by full CI and tag releases."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
TARGETS = ('x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc')
# These belong to the supported OS baseline, including the ELF interpreter.
BASE_LIBS = re.compile(r'^(linux-vdso|ld-linux|lib(c|m|dl|pthread|rt|resolv|util)\.so)')
DLOPEN_LIBS = ('libfontconfig.so.1', 'libwayland-client.so.0',
               'libwayland-cursor.so.0', 'libvulkan.so.1')
WINDOWS_LIBS = set('''advapi32 bcrypt bcryptprimitives cfgmgr32 comctl32 comdlg32
    combase crypt32 cryptbase cryptnet cryptui d2d1 d3d11 d3d12 d3dcompiler_47 dbghelp dcomp
    dwmapi dwrite dxgi gdi32 gdiplus icuin icuuc imm32 iphlpapi kernel32 kernelbase mpr msimg32
    msvcrt ncrypt netapi32 normaliz ntdll ole32 oleacc oleaut32 opengl32 powrprof
    propsys psapi rpcrt4 secur32 setupapi shell32 shcore shlwapi synchronization
    uiautomationcore ucrtbase user32 userenv uxtheme version windowscodecs winhttp wininet winmm
    winscard winspool wintrust wldap32 ws2_32 wtsapi32'''.split())


def run(*args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT).strip()


def windows_imports(binary):
    """Read normal and delay-load import tables without a host-specific SDK tool."""
    data = binary.read_bytes()
    if data[:2] != b'MZ':
        raise ValueError(f'not a PE executable: {binary}')
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    if data[pe:pe + 4] != b'PE\0\0':
        raise ValueError(f'invalid PE signature: {binary}')
    machine, count = struct.unpack_from('<HH', data, pe + 4)
    size = struct.unpack_from('<H', data, pe + 20)[0]
    optional = pe + 24
    if machine != 0x8664 or struct.unpack_from('<H', data, optional)[0] != 0x20b:
        raise ValueError(f'expected x86-64 PE32+: {binary}')
    sections = optional + size
    def offset(rva):
        for index in range(count):
            at = sections + index * 40
            virtual_size, start, raw_size, raw = struct.unpack_from('<IIII', data, at + 8)
            if start <= rva < start + max(virtual_size, raw_size):
                return raw + rva - start
        raise ValueError(f'unmapped PE RVA {rva:x}')
    imports = set()
    for directory, width, name_index in ((1, 20, 3), (13, 32, 1)):
        rva, length = struct.unpack_from('<II', data, optional + 112 + directory * 8)
        if not rva:
            continue
        table = offset(rva)
        for at in range(table, table + length, width):
            fields = struct.unpack_from('<' + 'I' * (width // 4), data, at)
            if not any(fields):
                break
            if directory == 13 and fields[0] != 1:
                raise ValueError('unsupported delay import addressing')
            name = offset(fields[name_index])
            end = data.index(b'\0', name)
            imports.add(data[name:end].decode('ascii').lower())
    return imports


def check_windows(binary):
    imports = windows_imports(binary)
    for name in imports:
        stem = name.removesuffix('.dll')
        if stem not in WINDOWS_LIBS and not stem.startswith(('api-ms-win-', 'ext-ms-win-')):
            raise ValueError(f'{binary.name} requires non-OS DLL {name}; use a static CRT build; '
                             f"all PE imports: {', '.join(sorted(imports))}")
    return sorted(imports)


def linux_dependencies(binary):
    output = run('ldd', str(binary))
    if 'not found' in output:
        raise ValueError(f'unresolved dependency of {binary}:\n{output}')
    result = {}
    for line in output.splitlines():
        match = re.match(r'\s*(\S+) => (/\S+) \(', line)
        if match and not BASE_LIBS.match(match[1]):
            result[match[1]] = Path(match[2])
    return result


def native_copyright(library, destination):
    real = library.resolve()
    candidates = [str(real)]
    if str(real).startswith('/usr/'):
        candidates.append(str(real)[4:])
    owner = None
    for candidate in candidates:
        found = subprocess.run(['dpkg-query', '-S', candidate], text=True, capture_output=True)
        if found.returncode == 0:
            owner = found.stdout.split(': ', 1)[0].split(':', 1)[0]
            break
    if not owner:
        raise ValueError(f'cannot identify license owner for {library}')
    copyright_file = Path('/usr/share/doc') / owner / 'copyright'
    shutil.copyfile(copyright_file, destination / f'{owner}.copyright')
    for name in re.findall(r'/usr/share/common-licenses/([A-Za-z0-9_.+-]+)',
                           copyright_file.read_text()):
        name = name.rstrip('.')
        shutil.copyfile(Path('/usr/share/common-licenses') / name, destination / name)


def bundle_linux(binary, stage):
    # Explicit roots cover libraries the toolkit loads with dlopen instead of DT_NEEDED.
    cache = run('ldconfig', '-p')
    roots = [binary]
    for name in DLOPEN_LIBS:
        match = re.search(r'^\s*' + re.escape(name) + r' \(libc6,x86-64[^\n]* => (\S+)', cache, re.M)
        if not match:
            raise ValueError(f'missing runtime library {name}')
        roots.append(Path(match[1]))
    libraries = {}
    for library in roots:
        libraries.update(linux_dependencies(library))
    for library in roots[1:]:
        libraries[library.name] = library
    lib = stage / 'lib'
    licenses = stage / 'licenses'
    lib.mkdir()
    licenses.mkdir()
    for name, path in sorted(libraries.items()):
        shutil.copyfile(path, lib / name)
        native_copyright(path, licenses)
    # Keep configuration beside the real executable and scope the library path to this process.
    shutil.copy2(binary, stage / 'argand-bin')
    launcher = stage / 'argand'
    launcher.write_text('''#!/bin/sh
set -eu
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export LD_LIBRARY_PATH="$app_dir/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$app_dir/argand-bin" "$@"
''')
    launcher.chmod(0o755)
    return sorted(libraries)


def supplemental_notices(package, source):
    support = Path(__file__).with_name('notices')
    with (support / 'manifest.json').open() as provenance:
        entries = json.load(provenance)
    name = f"{package['name']}-{package['version']}"
    entry = entries.get(name)
    if entry is None:
        return []
    if package['checksum'] != entry['package_checksum']:
        raise ValueError(f'supplemental notice package mismatch for {name}')
    with (source / '.cargo_vcs_info.json').open() as vcs:
        if json.load(vcs)['git']['sha1'] != entry['source_revision']:
            raise ValueError(f'supplemental notice source revision mismatch for {name}')
    license_file = support / entry['license_file']
    if hashlib.sha256(license_file.read_bytes()).hexdigest() != entry['license_sha256']:
        raise ValueError(f'supplemental notice checksum mismatch for {name}')
    return [(Path(name) / 'LICENSE', license_file),
            (Path(name) / 'PROVENANCE.json', support / 'manifest.json')]


def rust_notices(cargo_home=None):
    """Keep upstream notices for the downloaded locked graph, including embedded assets."""
    cargo_home = cargo_home or Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo'))
    with (ROOT / 'Cargo.lock').open('rb') as locked:
        packages = tomllib.load(locked)['package']
    files = []
    inventory = []
    for package in packages:
        if not package.get('source', '').startswith('registry+'):
            continue
        name = f"{package['name']}-{package['version']}"
        sources = list((cargo_home / 'registry/src').glob('*/' + name))
        if not sources:
            # Other targets and unused optional packages need not have been downloaded.
            continue
        source = sources[0]
        archive = cargo_home / 'registry/cache' / source.parent.name / (name + '.crate')
        if hashlib.sha256(archive.read_bytes()).hexdigest() != package['checksum']:
            raise ValueError(f'locked source checksum mismatch for {name}')
        with (source / 'Cargo.toml').open('rb') as manifest:
            metadata = tomllib.load(manifest)['package']
        notices = []
        explicit = metadata.get('license-file')
        for path in source.rglob('*'):
            if path.is_file() and (re.fullmatch(r'(licen[cs]e|copying|copyright|notice|ofl|ufl|unlicense)([._-].*)?',
                                              path.name, re.I) or path.relative_to(source).as_posix() == explicit):
                if path.suffix.lower() in ('.rs', '.svg', '.png', '.ttf', '.otf'):
                    continue
                relative = path.relative_to(source)
                files.append((Path(name) / relative, path))
                notices.append(relative.as_posix())
        extra = supplemental_notices(package, source)
        files.extend(extra)
        notices.extend(relative.name for relative, _ in extra)
        inventory.append({'name': package['name'], 'version': package['version'],
                          'license': metadata.get('license'), 'authors': metadata.get('authors', []),
                          'repository': metadata.get('repository'), 'readme': metadata.get('readme'),
                          'notices': sorted(notices)})
    lucide = Path('gpui-kit-assets-0.9.1/LICENSE-LUCIDE')
    if not any(relative == lucide for relative, _ in files):
        raise ValueError('missing locked GPUI Lucide/Feather asset notice')
    return files, inventory


def package(binary_dir, output, target, version, commit, smoke=True):
    if target not in TARGETS or not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise ValueError('unsupported target or version')
    windows = target.endswith('msvc')
    suffix = '.exe' if windows else ''
    notice_files, notice_inventory = rust_notices()
    output = output.resolve()
    # Never reuse or clear a caller's existing staging/output directory.
    output.mkdir(parents=True, exist_ok=False)
    archives = []
    for product in ('argand', 'aspec'):
        stem = f'{product}-v{version}-{target}'
        with tempfile.TemporaryDirectory(prefix='argand-release-') as temp:
            stage = Path(temp) / stem
            stage.mkdir()
            binary = binary_dir.resolve() / (product + suffix)
            if not binary.is_file():
                raise ValueError(f'missing executable {binary}')
            dependencies = []
            if windows:
                dependencies = check_windows(binary)
                shutil.copy2(binary, stage / binary.name)
            elif product == 'argand':
                dependencies = bundle_linux(binary, stage)
            else:
                shutil.copy2(binary, stage / binary.name)
            for name in ('README.md', 'LICENSE', 'CHANGELOG.md'):
                shutil.copyfile(ROOT / name, stage / name)
            notices = stage / 'licenses' / 'rust'
            notices.mkdir(parents=True, exist_ok=True)
            for relative, source in notice_files:
                destination = notices / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)
            (notices / 'INDEX.json').write_text(json.dumps(notice_inventory, indent=2) + '\n')
            if product == 'aspec':
                shutil.copyfile(ROOT / 'crates/cli/assets/DejaVuSans.LICENSE',
                                stage / 'licenses' / 'DejaVuSans.LICENSE')
            identity = {'version': version, 'commit': commit, 'target': target,
                        'toolkit': 'source-toolkit', 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                        'native_dependencies': dependencies}
            (stage / 'BUILD-INFO.json').write_text(json.dumps(identity, indent=2) + '\n')
            if smoke:
                reported = run(str(stage / (product + suffix)), '--version')
                if reported != f'{product} {version}':
                    raise ValueError(f'unexpected packaged version: {reported}')
                if product == 'aspec':
                    image = Path(temp) / 'flac-smoke.png'
                    run(str(stage / binary.name), str(ROOT / '.github/scripts/fixtures/iq-2k.flac'),
                        '-o', str(image))
                    if not image.is_file() or image.stat().st_size == 0:
                        raise ValueError('packaged aspec did not render the stereo FLAC fixture')
            if windows:
                archive = output / (stem + '.zip')
                with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED) as zipped:
                    for path in sorted(stage.rglob('*')):
                        if path.is_file():
                            zipped.write(path, path.relative_to(stage.parent))
            else:
                archive = output / (stem + '.tar.gz')
                with tarfile.open(archive, 'x:gz') as tar:
                    tar.add(stage, arcname=stem)
            archives.append(archive)
    return archives


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True, help='new directory, never overwritten')
    parser.add_argument('--target', choices=TARGETS, required=True)
    args = parser.parse_args()
    with (ROOT / 'Cargo.toml').open('rb') as manifest:
        version = tomllib.load(manifest)['workspace']['package']['version']
    commit = run('git', '-C', str(ROOT), 'rev-parse', 'HEAD')
    for archive in package(args.binary_dir, args.output, args.target, version, commit):
        print(archive)


if __name__ == '__main__':
    main()

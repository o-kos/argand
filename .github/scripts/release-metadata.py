#!/usr/bin/env python3
"""Validate release metadata without invoking Cargo or selecting a toolchain."""
import argparse
from datetime import date
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def read(path, revision=None):
    if revision:
        return subprocess.check_output(['git', '-C', str(ROOT), 'show', f'{revision}:{path}'], text=True)
    return (ROOT / path).read_text()


def metadata(revision=None, require_release=False):
    manifest = tomllib.loads(read('Cargo.toml', revision))
    workspace = manifest['workspace']
    version = workspace['package']['version']
    if not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise ValueError('workspace version must be X.Y.Z')
    locked = tomllib.loads(read('Cargo.lock', revision))['package']
    for member in workspace['members']:
        package = tomllib.loads(read(f'{member}/Cargo.toml', revision))['package']
        if package.get('version') != {'workspace': True}:
            raise ValueError(f'{member} must inherit the workspace version')
        entries = [p for p in locked if p['name'] == package['name'] and 'source' not in p]
        if len(entries) != 1 or entries[0]['version'] != version:
            raise ValueError(f'lockfile version mismatch for {package["name"]}')
    changelog = read('CHANGELOG.md', revision)
    headings = list(re.finditer(r'^## \[([^\]]+)\](.*)$', changelog, re.M))
    releases = [h for h in headings if h[1] != 'Unreleased']
    if not releases or releases[0][1] != version:
        raise ValueError('workspace version must match the latest changelog release')
    heading = releases[0]
    if sum(h[1] == version for h in headings) != 1:
        raise ValueError('duplicate release changelog section')
    dated = re.fullmatch(r' - (\d{4}-\d{2}-\d{2})', heading[2])
    if not dated:
        raise ValueError('release changelog section must have YYYY-MM-DD date')
    date.fromisoformat(dated[1])
    end = next((h.start() for h in headings if h.start() > heading.start()), len(changelog))
    notes = changelog[heading.end():end].strip()
    if not notes:
        raise ValueError('release changelog section must contain notes')
    # Regular development can accumulate Unreleased entries. A release commit cannot.
    if require_release:
        unreleased = [h for h in headings if h[1] == 'Unreleased']
        if len(unreleased) != 1 or headings[0][1] != 'Unreleased':
            raise ValueError('release needs one leading Unreleased section')
        pending = changelog[unreleased[0].end():heading.start()]
        if any(line.strip() and not line.startswith('### ') for line in pending.splitlines()):
            raise ValueError('release commit still has Unreleased entries')
    return version, notes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision')
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--tag')
    args = parser.parse_args()
    version, notes = metadata(args.revision, args.release)
    if args.tag and args.tag != f'v{version}':
        raise ValueError('tag does not match workspace version')
    print(notes if args.tag else version)


if __name__ == '__main__':
    main()

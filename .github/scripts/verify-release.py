#!/usr/bin/env python3
"""Fail before release builds unless an immutable release tag has exact main CI."""
import importlib.util
import os
from pathlib import Path
import re

script = Path(__file__).with_name('initiate-release.py')
spec = importlib.util.spec_from_file_location('initiation', script)
initiation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(initiation)


def main():
    repository = os.environ['GITHUB_REPOSITORY']
    for actor in {os.environ['GITHUB_ACTOR'], os.environ['GITHUB_TRIGGERING_ACTOR']}:
        permission = initiation.api(f'repos/{repository}/collaborators/{actor}/permission')
        if permission['permission'] != 'admin':
            raise ValueError('release initiation and reruns require repository administrator authority')
    reference = os.environ['GITHUB_REF']
    if not re.fullmatch(r'refs/tags/v\d+\.\d+\.\d+', reference):
        raise ValueError('release dispatch must select an existing vX.Y.Z tag')
    sha = initiation.run('git', 'rev-parse', f'{reference}^{{commit}}')
    if sha != initiation.run('git', 'rev-parse', 'HEAD'):
        raise ValueError('checked-out commit differs from selected tag')
    initiation.run('git', 'fetch', '--no-tags', 'origin', 'refs/heads/main')
    initiation.run('git', 'merge-base', '--is-ancestor', sha, 'FETCH_HEAD')
    version, _ = initiation.METADATA.metadata(sha, require_release=True)
    if reference != f'refs/tags/v{version}':
        raise ValueError('tag/version mismatch')
    branch = initiation.api(f'repos/{repository}/branches/main')
    checks = branch['protection']['required_status_checks']['checks']
    if {'context': 'ci/full', 'app_id': 15368} not in checks:
        raise ValueError('main must require GitHub Actions ci/full')
    initiation.validate_ci(repository, sha, checks)
    releases = initiation.json.loads(initiation.run('gh', 'api', '--paginate', '--slurp',
                                                   f'repos/{repository}/releases?per_page=100'))
    if any(r['tag_name'] == f'v{version}' for page in releases for r in page):
        raise ValueError('release already exists, refusing publication retry')
    print(f'Validated {reference} at {sha}')


if __name__ == '__main__':
    main()

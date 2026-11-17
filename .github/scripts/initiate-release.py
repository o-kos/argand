#!/usr/bin/env python3
"""Administrator-only release initiation at one explicitly approved full SHA."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('release_metadata', Path(__file__).with_name('release-metadata.py'))
METADATA = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(METADATA)


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def api(path):
    return json.loads(run('gh', 'api', path))


def validate_ci(repository, sha, checks):
    """Accept only the latest canonical main push run and its own required checks."""
    runs = api(f'repos/{repository}/actions/workflows/ci.yml/runs?head_sha={sha}&event=push&per_page=100')['workflow_runs']
    runs = [r for r in runs if r['head_sha'] == sha and r['head_branch'] == 'main'
            and r['path'] == '.github/workflows/ci.yml' and r['event'] == 'push']
    if not runs or runs[0]['status'] != 'completed' or runs[0]['conclusion'] != 'success':
        raise ValueError('latest canonical main CI run for approved SHA is missing or unsuccessful')
    suite = runs[0]['check_suite_id']
    actual = api(f'repos/{repository}/check-suites/{suite}/check-runs?filter=latest&per_page=100')['check_runs']
    for required in checks:
        candidates = [c for c in actual if c['name'] == required['context'] and
                      c['app']['id'] == required['app_id'] and c['head_sha'] == sha]
        if len(candidates) != 1 or candidates[0]['conclusion'] != 'success':
            raise ValueError(f'required canonical CI check is missing or unsuccessful: {required["context"]}')
    return runs[0]['id']


def preflight(repository, sha):
    if not re.fullmatch(r'[0-9a-f]{40}', sha):
        raise ValueError('approval requires a full lowercase 40-character commit SHA')
    if api(f'repos/{repository}')['permissions'].get('admin') is not True:
        raise ValueError('the authenticated GitHub user must be a repository administrator')
    protection = api(f'repos/{repository}/branches/main/protection')
    required = protection['required_status_checks']
    if not protection['enforce_admins']['enabled'] or not required['strict']:
        raise ValueError('main protection must enforce admins and strict required CI')
    checks = required['checks']
    if {'context': 'ci/full', 'app_id': 15368} not in checks:
        raise ValueError('main must require GitHub Actions ci/full')
    rules = api(f'repos/{repository}/rulesets')
    tag_rules = [api(f'repos/{repository}/rulesets/{r["id"]}') for r in rules if r['target'] == 'tag' and r['enforcement'] == 'active']
    if not any(r['conditions']['ref_name'] == {'include': ['refs/tags/v*'], 'exclude': []}
               and {x['type'] for x in r['rules']} >= {'creation', 'update', 'deletion'}
               and r['bypass_actors'] == [{'actor_id': 5, 'actor_type': 'RepositoryRole', 'bypass_mode': 'always'}]
               for r in tag_rules):
        raise ValueError('administrator-only immutable release tag ruleset is required')
    # Fetch objects into FETCH_HEAD only. Never replace a local branch or tag.
    run('git', 'fetch', '--no-tags', f'https://github.com/{repository}.git', 'refs/heads/main')
    run('git', 'merge-base', '--is-ancestor', sha, 'FETCH_HEAD')
    if run('git', 'rev-parse', f'{sha}^{{commit}}') != sha:
        raise ValueError('approved SHA must identify a commit')
    workflow = METADATA.read('.github/workflows/release.yml', sha)
    if 'workflow_dispatch:' not in workflow or 'python3 .github/scripts/verify-release.py' not in workflow:
        raise ValueError('approved commit must contain the guarded release dispatch workflow')
    version, _ = METADATA.metadata(sha, require_release=True)
    ci_run = validate_ci(repository, sha, checks)
    return f'v{version}', ci_run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--commit', required=True, help='exact owner-approved merged main commit')
    parser.add_argument('--execute', action='store_true', help='create/push the tag after preflight')
    parser.add_argument('--retry', action='store_true', help='explicitly redispatch an existing matching tag')
    args = parser.parse_args()
    # Explicit user authentication is needed: GITHUB_TOKEN cannot start a tag-push workflow.
    if any(os.environ.get(name) for name in ('GITHUB_ACTIONS', 'GITHUB_TOKEN', 'GH_TOKEN')):
        raise ValueError('run locally using existing gh user/keyring and administrator git authentication')
    repository = run('gh', 'repo', 'view', '--json', 'nameWithOwner', '--jq', '.nameWithOwner')
    tag, ci_run = preflight(repository, args.commit)
    # Include drafts, and fail closed on API/auth errors rather than interpreting all 404s as absence.
    releases = json.loads(run('gh', 'api', '--paginate', '--slurp', f'repos/{repository}/releases?per_page=100'))
    if any(r['tag_name'] == tag for page in releases for r in page):
        raise ValueError('a release already exists, including drafts; never replace it automatically')
    refs = run('git', 'ls-remote', f'https://github.com/{repository}.git', f'refs/tags/{tag}', f'refs/tags/{tag}^{{}}')
    remote = dict(line.split()[::-1] for line in refs.splitlines())
    target = remote.get(f'refs/tags/{tag}^{{}}', remote.get(f'refs/tags/{tag}'))
    if target and target != args.commit:
        raise ValueError('existing remote tag points to a different commit')
    if bool(target) != args.retry:
        raise ValueError('existing matching tag requires --retry; --retry requires an existing matching tag')
    local = subprocess.run(['git', 'show-ref', '--verify', '--quiet', f'refs/tags/{tag}'], cwd=ROOT)
    if local.returncode not in (0, 1):
        raise ValueError('cannot inspect local release tag')
    if local.returncode == 0 and run('git', 'rev-parse', f'refs/tags/{tag}^{{commit}}') != args.commit:
        raise ValueError('existing local tag points to a different commit')
    if args.retry:
        runs = api(f'repos/{repository}/actions/workflows/release.yml/runs?head_sha={args.commit}&per_page=100')['workflow_runs']
        if any(r['head_branch'] == tag and r['status'] != 'completed' for r in runs):
            raise ValueError('release workflow for this tag is still active')
    # Push an annotated object directly, avoiding any local release tag mutation.
    print(json.dumps({'commit': args.commit, 'tag': tag, 'ci_run': ci_run,
                      'operation': 'retry' if args.retry else 'create', 'execute': args.execute}))
    if not args.execute:
        return
    if args.retry:
        # The workflow at the immutable approved tag handles the same exact source SHA.
        run('gh', 'workflow', 'run', 'release.yml', '--repo', repository, '--ref', tag)
        return
    create_tag(repository, args.commit, tag)


def create_tag(repository, sha, tag):
    identity = run('git', 'var', 'GIT_COMMITTER_IDENT')
    annotation = f'object {sha}\ntype commit\ntag {tag}\ntagger {identity}\n\nRelease {tag}\n'
    object_id = subprocess.check_output(['git', 'mktag'], input=annotation, text=True, cwd=ROOT).strip()
    # No force: a concurrent tag creator wins and this operation fails safely.
    run('git', 'push', f'git@github.com:{repository}.git', f'{object_id}:refs/tags/{tag}')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f'error: {error}', file=sys.stderr)
        sys.exit(1)

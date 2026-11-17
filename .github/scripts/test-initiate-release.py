"""Exercise release success/rejections without any production tag or publication."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('initiation', HERE / 'initiate-release.py')
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)
SHA = 'a' * 40
OTHER = 'b' * 40
CHECK = {'context': 'ci/full', 'app_id': 15368}


class MetadataTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'crate').mkdir()
        (self.root / 'crate/Cargo.toml').write_text('[package]\nname="app"\nversion.workspace=true\n')
        (self.root / 'Cargo.toml').write_text('[workspace]\nmembers=["crate"]\n[workspace.package]\nversion="1.2.3"\n')
        (self.root / 'Cargo.lock').write_text('[[package]]\nname="app"\nversion="1.2.3"\n')
        (self.root / 'CHANGELOG.md').write_text('## [Unreleased]\n\n## [1.2.3] - 2026-11-16\n\n- Release fixture\n')
        self.addCleanup(patch.stopall)
        patch.object(release.METADATA, 'ROOT', self.root).start()

    def test_consistent_metadata_and_exact_revision(self):
        self.assertEqual(release.METADATA.metadata(require_release=True), ('1.2.3', '- Release fixture'))
        for args in [('init', '-q'), ('add', '.'), ('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture')]:
            subprocess.run(['git', '-C', str(self.root), *args], check=True)
        sha = subprocess.check_output(['git', '-C', str(self.root), 'rev-parse', 'HEAD'], text=True).strip()
        (self.root / 'Cargo.toml').write_text('invalid moving working tree')
        self.assertEqual(release.METADATA.metadata(sha, True)[0], '1.2.3')

    def test_mismatch_missing_date_duplicate_and_pending_entries(self):
        cases = [('Cargo.lock', '[[package]]\nname="app"\nversion="1.2.2"\n'),
                 ('CHANGELOG.md', '## [Unreleased]\n\n## [1.2.3]\n- Note\n'),
                 ('CHANGELOG.md', '## [Unreleased]\n\n## [1.2.3] - 2026-02-30\n- Note\n'),
                 ('CHANGELOG.md', '## [Unreleased]\n\n## [1.2.3] - 2026-11-16\n- Note\n## [1.2.3] - 2026-11-15\n- Duplicate\n'),
                 ('CHANGELOG.md', '## [Unreleased]\n- Pending\n## [1.2.3] - 2026-11-16\n- Note\n')]
        for filename, contents in cases:
            with self.subTest(filename=filename, contents=contents):
                path = self.root / filename
                original = path.read_text()
                path.write_text(contents)
                with self.assertRaises(ValueError):
                    release.METADATA.metadata(require_release=True)
                path.write_text(original)
        (self.root / 'CHANGELOG.md').write_text('## [Unreleased]\n- Development\n## [1.2.3] - 2026-11-16\n- Note\n')
        self.assertEqual(release.METADATA.metadata()[0], '1.2.3')


class CanonicalCITests(unittest.TestCase):
    def test_only_latest_canonical_main_run_and_own_app_checks(self):
        run = {'id': 42, 'head_sha': SHA, 'head_branch': 'main', 'path': '.github/workflows/ci.yml',
               'event': 'push', 'status': 'completed', 'conclusion': 'success', 'check_suite_id': 10}
        check = {'name': 'ci/full', 'head_sha': SHA, 'app': {'id': 15368}, 'conclusion': 'success'}
        def validate(runs, checks):
            with patch.object(release, 'api', side_effect=[{'workflow_runs': runs}, {'check_runs': checks}]):
                return release.validate_ci('fixture/repo', SHA, [CHECK])
        self.assertEqual(validate([run], [check]), 42)
        for field, value in [('head_sha', OTHER), ('head_branch', 'other'), ('event', 'pull_request'),
                             ('path', '.github/workflows/spoof.yml'), ('status', 'in_progress'),
                             ('conclusion', 'failure'), ('conclusion', 'cancelled')]:
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                validate([dict(run, **{field: value})], [check])
        for changes in ({'head_sha': OTHER}, {'app': {'id': 1}}, {'name': 'ci/quick'}, {'conclusion': 'skipped'}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                validate([run], [dict(check, **changes)])
        with self.assertRaises(ValueError):
            validate([dict(run, status='in_progress'), run], [check])
        with self.assertRaises(ValueError):
            validate([run], [])


class InitiationTests(unittest.TestCase):
    def call(self, *, target=None, retry=False, published=False, execute=False):
        args = ['initiate-release.py', '--commit', SHA]
        if retry:
            args.append('--retry')
        if execute:
            args.append('--execute')
        calls = []
        def run(*command):
            calls.append(command)
            if command[:3] == ('gh', 'repo', 'view'):
                return 'fixture/repo'
            if command[:4] == ('gh', 'api', '--paginate', '--slurp'):
                return json.dumps([[{'tag_name': 'v1.2.3', 'draft': True}] if published else []])
            if command[:2] == ('git', 'ls-remote'):
                return f'{target} refs/tags/v1.2.3\n' if target else ''
            return ''
        with patch.object(release, 'preflight', return_value=('v1.2.3', 42)), \
             patch.object(release, 'run', side_effect=run), \
             patch.object(release, 'api', return_value={'workflow_runs': []}), \
             patch.object(release.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1)), \
             patch.dict(os.environ, {}, clear=True), patch('sys.argv', args), contextlib.redirect_stdout(io.StringIO()):
            release.main()
        return calls

    def test_readonly_preflight_creates_nothing(self):
        calls = self.call()
        self.assertFalse(any(c[:2] == ('git', 'push') or c[:3] == ('gh', 'workflow', 'run') for c in calls))

    def test_conflicts_existing_release_and_explicit_retry(self):
        for options in ({'target': OTHER}, {'target': SHA}, {'retry': True}, {'published': True},
                        {'target': SHA, 'retry': True, 'published': True}):
            with self.subTest(options=options), self.assertRaises(ValueError):
                self.call(**options)
        calls = self.call(target=SHA, retry=True, execute=True)
        self.assertIn(('gh', 'workflow', 'run', 'release.yml', '--repo', 'fixture/repo', '--ref', 'v1.2.3'), calls)

    def test_short_sha_and_github_token_fail_closed(self):
        with self.assertRaises(ValueError):
            release.preflight('fixture/repo', 'main')
        with patch.dict(os.environ, {'GITHUB_TOKEN': 'fixture'}), patch('sys.argv', ['script', '--commit', SHA]), self.assertRaises(ValueError):
            release.main()


class AuthorityTests(unittest.TestCase):
    def fixtures(self):
        return {
            'repos/fixture/repo': {'permissions': {'admin': True}},
            'repos/fixture/repo/branches/main/protection': {
                'required_status_checks': {'strict': True, 'checks': [CHECK]},
                'enforce_admins': {'enabled': True}},
            'repos/fixture/repo/rulesets': [{'id': 1, 'target': 'tag', 'enforcement': 'active'}],
            'repos/fixture/repo/rulesets/1': {
                'conditions': {'ref_name': {'include': ['refs/tags/v*'], 'exclude': []}},
                'rules': [{'type': name} for name in ('creation', 'update', 'deletion')],
                'bypass_actors': [{'actor_id': 5, 'actor_type': 'RepositoryRole', 'bypass_mode': 'always'}]}}

    def test_admin_ruleset_protection_and_merged_sha_fail_closed(self):
        for mutation in ('none', 'writer', 'not_strict', 'admin_exempt', 'no_ruleset', 'bypass_writer', 'missing_ci', 'not_merged'):
            data = self.fixtures()
            if mutation == 'writer':
                data['repos/fixture/repo']['permissions']['admin'] = False
            if mutation == 'not_strict':
                data['repos/fixture/repo/branches/main/protection']['required_status_checks']['strict'] = False
            if mutation == 'admin_exempt':
                data['repos/fixture/repo/branches/main/protection']['enforce_admins']['enabled'] = False
            if mutation == 'no_ruleset':
                data['repos/fixture/repo/rulesets'] = []
            if mutation == 'bypass_writer':
                data['repos/fixture/repo/rulesets/1']['bypass_actors'][0]['actor_id'] = 4
            if mutation == 'missing_ci':
                data['repos/fixture/repo/branches/main/protection']['required_status_checks']['checks'] = []
            def run(*command):
                if command[:3] == ('git', 'merge-base', '--is-ancestor') and mutation == 'not_merged':
                    raise subprocess.CalledProcessError(1, command)
                return SHA
            with self.subTest(mutation=mutation), patch.object(release, 'api', side_effect=lambda path: data[path]), \
                 patch.object(release, 'run', side_effect=run), \
                 patch.object(release.METADATA, 'read', return_value='workflow_dispatch: python3 .github/scripts/verify-release.py'), \
                 patch.object(release.METADATA, 'metadata', return_value=('1.2.3', '- Notes')), \
                 patch.object(release, 'validate_ci', return_value=42):
                if mutation == 'none':
                    self.assertEqual(release.preflight('fixture/repo', SHA), ('v1.2.3', 42))
                else:
                    with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                        release.preflight('fixture/repo', SHA)


class GitHandoffTests(unittest.TestCase):
    def test_real_annotated_tag_push_is_exact_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source, remote = root / 'source', root / 'remote.git'
            subprocess.run(['git', 'init', '-q', str(source)], check=True)
            subprocess.run(['git', 'init', '--bare', '-q', str(remote)], check=True)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(source), *args], text=True).strip()
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            (source / 'fixture').write_text('approved source')
            git('add', '.')
            git('commit', '-qm', 'approved')
            approved = git('rev-parse', 'HEAD')
            original_run = release.run
            def local_run(*command):
                if command[:2] == ('git', 'push'):
                    command = ('git', 'push', str(remote), command[3])
                return original_run(*command)
            with patch.object(release, 'ROOT', source), patch.object(release, 'run', side_effect=local_run):
                release.create_tag('fixture/repo', approved, 'v1.2.3')
                self.assertEqual(git('tag', '--list'), '')
                target = subprocess.check_output(['git', '--git-dir', str(remote), 'rev-parse', 'refs/tags/v1.2.3^{}'], text=True).strip()
                self.assertEqual(target, approved)
                self.assertEqual(subprocess.check_output(['git', '--git-dir', str(remote), 'cat-file', '-t', 'refs/tags/v1.2.3'], text=True).strip(), 'tag')
                (source / 'fixture').write_text('superseded source')
                git('commit', '-qam', 'different')
                with self.assertRaises(subprocess.CalledProcessError):
                    release.create_tag('fixture/repo', git('rev-parse', 'HEAD'), 'v1.2.3')
                self.assertEqual(subprocess.check_output(['git', '--git-dir', str(remote), 'rev-parse', 'refs/tags/v1.2.3^{}'], text=True).strip(), approved)


class WorkflowAuthorityTests(unittest.TestCase):
    def test_original_and_rerun_actors_both_require_admin(self):
        spec = importlib.util.spec_from_file_location('verification', HERE / 'verify-release.py')
        verification = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(verification)
        environment = {'GITHUB_REPOSITORY': 'fixture/repo', 'GITHUB_ACTOR': 'owner',
                       'GITHUB_TRIGGERING_ACTOR': 'writer', 'GITHUB_REF': 'refs/tags/v1.2.3'}
        with patch.dict(os.environ, environment, clear=True), \
             patch.object(verification.initiation, 'api', side_effect=lambda path: {'permission': 'admin' if '/owner/' in path else 'write'}), \
             self.assertRaises(ValueError):
            verification.main()


if __name__ == '__main__':
    unittest.main()

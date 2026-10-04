"""Imported attribution is retained; new contribution attribution still fails."""
import importlib.util
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('validation', Path(__file__).resolve().parents[2] / 'scripts/validate_fork_pr.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class ValidationTests(unittest.TestCase):
    def test_vendor_history_excluded_but_new_bad_commit_checked(self):
        original = Path.cwd()
        with tempfile.TemporaryDirectory() as tmp:
            os.chdir(tmp)
            try:
                def git(*args):
                    return subprocess.check_output(['git', *args], text=True).strip()
                git('init', '-q'); git('config', 'user.name', 'Fixture'); git('config', 'user.email', 'fixture@example.invalid')
                git('commit', '--allow-empty', '-qm', 'base'); base = git('rev-parse', 'HEAD')
                git('commit', '--allow-empty', '-qm', 'vendor\n\nCo-Authored-By: AI <fixture@example.invalid>')
                module.UPSTREAM_RELEASE = git('rev-parse', 'HEAD')
                git('commit', '--allow-empty', '-qm', 'fix: fork change'); good = git('rev-parse', 'HEAD')
                self.assertEqual(module.contribution_commits(base, good), [good])
                git('commit', '--allow-empty', '-qm', 'fix: bad change\n\nCo-Authored-By: AI <fixture@example.invalid>')
                bad = git('rev-parse', 'HEAD')
                self.assertIn(bad, module.contribution_commits(base, bad))
                self.assertTrue(module.ATTRIBUTION.search(git('show', '-s', '--format=%B', bad)))
                module.UPSTREAM_RELEASE = base
                self.assertEqual(len(module.contribution_commits(base, bad)), 3)
            finally:
                os.chdir(original)

if __name__ == '__main__':
    unittest.main()

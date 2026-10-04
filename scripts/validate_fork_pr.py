"""Validate fork contributions without rewriting imported upstream history."""
import os
import re
import subprocess

UPSTREAM_RELEASE = 'f4b24ece538efed9ab2c4e6db9409fbfc4cc3a67'
TITLE = re.compile(r'^(feat|fix|docs|chore|refactor|test|ci|perf|build|style|revert)(\(.+\))?: .+')
ATTRIBUTION = re.compile(
    r'Co-Authored-By:.*\b(Claude|GPT|Copilot|AI|Anthropic|OpenAI)\b'
    r'|Generated (by|with).*\b(AI|Claude|GPT|Copilot|Anthropic|OpenAI)\b', re.IGNORECASE,
)

def contribution_commits(base, head):
    """Exclude pinned vendor ancestry only when actually merged into HEAD."""
    revisions = [head, '^' + base]
    ancestor = subprocess.run(['git', 'merge-base', '--is-ancestor', UPSTREAM_RELEASE, head], capture_output=True)
    if ancestor.returncode == 0:
        revisions.append('^' + UPSTREAM_RELEASE)
    return subprocess.check_output(['git', 'rev-list', *revisions], text=True).splitlines()

def main():
    if not TITLE.match(os.environ['PR_TITLE']):
        raise SystemExit('PR title must use conventional commit format')
    commits = contribution_commits(os.environ['BASE_SHA'], os.environ['HEAD_SHA'])
    for sha in commits:
        message = subprocess.check_output(['git', 'show', '-s', '--format=%B', sha], text=True)
        if ATTRIBUTION.search(message):
            raise SystemExit('AI attribution in new fork contribution: ' + sha)
    print(f'Validated title and {len(commits)} fork contribution commits')

if __name__ == '__main__':
    main()

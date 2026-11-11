# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Plan immutable main-branch releases and publish only verified distributions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
import urllib.error
import urllib.parse
import urllib.request
import zipfile

from package_release import ROOT, release_names


def git(*args, check=True):
    result = subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, encoding='utf-8')
    if check and result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result


def current():
    version = tomllib.loads((ROOT/'app/Cargo.toml').read_text(encoding='utf-8'))['package']['version']
    release_names(version)
    sha = git('rev-parse', 'HEAD').stdout.strip()
    if sha != os.environ['GITHUB_SHA'] or os.environ['GITHUB_REF'] != 'refs/heads/main':
        raise ValueError('Releases require the exact main workflow commit')
    if git('status', '--porcelain').stdout.strip():
        raise ValueError('Release source must be a clean committed checkout')
    return version, sha


class GitHub:
    def __init__(self, repository):
        if not re.fullmatch(r'[\w.-]+/[\w.-]+', repository):
            raise ValueError('Invalid repository identity')
        self.base = 'https://api.github.com/repos/'+repository
        self.token = os.environ.get('GH_TOKEN', '')

    def call(self, path, data=None, method=None, missing=False, upload=False):
        url = path if upload else self.base+'/'+path
        if upload and urllib.parse.urlsplit(url).netloc != 'uploads.github.com':
            raise ValueError('Unexpected release upload host')
        headers = {'User-Agent': 'RetroHatch-release', 'Accept': 'application/vnd.github+json'}
        if self.token:
            headers['Authorization'] = 'Bearer '+self.token
        if upload:
            headers['Content-Type'] = 'application/octet-stream'
        elif data is not None:
            data = json.dumps(data).encode('utf-8')
            headers['Content-Type'] = 'application/json'
        request = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(request, timeout=180 if upload else 30) as response:
                raw = response.read()
                return json.loads(raw) if raw else None
        except urllib.error.HTTPError as error:
            if missing and error.code == 404:
                return None
            # Do not log credentials, request headers or response bodies.
            raise RuntimeError(f'GitHub {request.get_method()} failed: HTTP {error.code} for {path.split("?")[0]}') from None


def tag_commit(api, tag):
    ref = api.call('git/ref/tags/'+urllib.parse.quote(tag, safe=''), missing=True)
    if ref is None:
        return None
    obj = ref['object']
    for _ in range(8):
        if obj['type'] == 'commit':
            return obj['sha']
        if obj['type'] != 'tag':
            break
        obj = api.call('git/tags/'+obj['sha'])['object']
    raise ValueError('Tag does not resolve to a commit')


def decision(version, previous, event, release, tagged, sha):
    tag = 'v'+version
    release_names(version)
    if release and not release['draft']:
        return False, 'The version already has a published release'
    if event == 'push' and previous == version:
        return False, 'Application version is unchanged'
    if event not in ('push', 'workflow_dispatch'):
        raise ValueError('Unsupported release event')
    if tagged is not None and tagged != sha:
        raise ValueError('Existing version tag points to another commit; it will not be moved')
    if release and f'<!-- retrohatch-release:{sha} -->' not in (release.get('body') or ''):
        raise ValueError('Existing draft is not owned by this verified commit')
    return True, f'Validate and release {tag}'


def plan(api):
    version, sha = current()
    event = os.environ['GITHUB_EVENT_NAME']
    previous = None
    if event == 'push':
        payload = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text(encoding='utf-8'))
        before = payload['before']
        if before != '0'*40:
            if not re.fullmatch('[0-9a-f]{40}', before):
                raise ValueError('Invalid preceding commit')
            previous = tomllib.loads(git('show', before+':app/Cargo.toml').stdout)['package']['version']
    tag = 'v'+version
    release = api.call('releases/tags/'+urllib.parse.quote(tag, safe=''), missing=True)
    tagged = tag_commit(api, tag) if not release or release['draft'] else None
    enabled, reason = decision(version, previous, event, release, tagged, sha)
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8', newline='\n') as output:
        output.write(f'publish={str(enabled).lower()}\ntag={tag}\n')
    print(reason)


def checksum(path):
    with path.open('rb') as file:
        return hashlib.file_digest(file, 'sha256').hexdigest()


def verify_artifacts(folder, version, sha):
    windows, source = release_names(version)
    names = [windows, source, 'SHA256SUMS.txt']
    rows = (folder/'SHA256SUMS.txt').read_text(encoding='utf-8').splitlines()
    expected = {}
    for row in rows:
        digest, name = row.split('  ', 1)
        if not re.fullmatch('[0-9a-f]{64}', digest) or name in expected:
            raise ValueError('Invalid/duplicate artifact checksum')
        expected[name] = digest
    if set(expected) != {windows, source}:
        raise ValueError('Missing or unexpected distribution archive')
    for name, digest in expected.items():
        if checksum(folder/name) != digest:
            raise ValueError('Distribution checksum mismatch: '+name)
    required = {'LICENSE', 'COPYRIGHT.md', 'THIRD_PARTY_LICENSES.md', 'SOURCE.md', 'BUILD_INFO.json',
                'LICENSES/CARGO_PACKAGES.json', 'LICENSES/CARGO_NOTICES.txt',
                'app/LICENSES/BrickEmuPy-CC0-1.0.txt', 'app/LICENSES/TamaEmu-GPL-3.0.txt',
                'app/third_party/tg18-core/LICENSE'}
    for name in (windows, source):
        with zipfile.ZipFile(folder/name) as archive:
            entries = set(archive.namelist())
            if not required.issubset(entries) or archive.testzip():
                raise ValueError('Incomplete or corrupt source/license package')
            info = json.loads(archive.read('BUILD_INFO.json'))
            if info['revision'] != sha or info['version'] != version or info['local_draft']:
                raise ValueError('Artifacts do not match the verified release commit')
            if name == windows:
                if 'RetroHatch.exe' not in entries or hashlib.sha256(archive.read('RetroHatch.exe')).hexdigest() != info['executable_sha256']:
                    raise ValueError('Windows executable identity mismatch')
                if any(Path(p).suffix.lower() in {'.rom', '.bin', '.rhstate', '.rhl'} for p in entries):
                    raise ValueError('Firmware or user data found in Windows package')
            elif not {'app/Cargo.toml', 'app/Cargo.lock', 'app/build.rs', '.cargo/config.toml'}.issubset(entries) or not any(p.startswith('vendor/') for p in entries):
                raise ValueError('Corresponding offline source is incomplete')
    return names


def history_notes(api, sha, tag):
    baseline = None
    for release in api.call('releases?per_page=100'):
        if release['draft'] or release['tag_name'] == tag:
            continue
        ref = 'refs/tags/'+release['tag_name']
        if git('rev-parse', '--verify', ref, check=False).returncode == 0 and git('merge-base', '--is-ancestor', ref, sha, check=False).returncode == 0:
            baseline = ref
            break
    commits = git('log', '--reverse', '--format=%H%x00%s', f'{baseline}..{sha}' if baseline else sha).stdout.splitlines()
    groups = {'New features': [], 'Bug fixes': [], 'Important improvements': []}
    for line in commits:
        commit, subject = line.split('\0', 1)
        group = 'New features' if re.match(r'feat(?:\([^)]*\))?!?:', subject, re.I) else 'Bug fixes' if re.match(r'fix(?:\([^)]*\))?!?:', subject, re.I) else 'Important improvements'
        subject = subject.replace('<', '&lt;').replace('>', '&gt;')
        groups[group].append(f'- {subject} (`{commit[:8]}`)')
    notes = []
    for heading, entries in groups.items():
        if entries:
            notes.append('## '+heading+'\n\n'+'\n'.join(entries))
    notes.append('## Known limitations\n\n'+(ROOT/'.github/release-limitations.md').read_text(encoding='utf-8').strip())
    return '\n\n'.join(notes)


def publish(api, folder):
    version, sha = current()
    tag = 'v'+version
    if os.environ['RELEASE_TAG'] != tag:
        raise ValueError('Workflow/Cargo version mismatch')
    names = verify_artifacts(folder, version, sha)
    release = api.call('releases/tags/'+urllib.parse.quote(tag, safe=''), missing=True)
    if release and not release['draft']:
        print('Release already published; nothing overwritten')
        return
    tagged = tag_commit(api, tag)
    decision(version, None, 'workflow_dispatch', release, tagged, sha)
    marker = f'<!-- retrohatch-release:{sha} -->'
    body = (folder/'RELEASE_NOTES.md').read_text(encoding='utf-8')+'\n'+history_notes(api, sha, tag)+'\n\n'+marker
    if tagged is None:
        # No update/delete API is ever used on refs. A race fails safely.
        api.call('git/refs', {'ref':'refs/tags/'+tag, 'sha':sha}, method='POST')
    if tag_commit(api, tag) != sha:
        raise ValueError('Release tag does not identify the verified commit')
    if release is None:
        release = api.call('releases', {'tag_name':tag, 'target_commitish':sha,
            'name':'RetroHatch '+tag, 'body':body, 'draft':True,
            'prerelease':False}, method='POST')
    assets = {a['name']:a for a in release.get('assets', [])}
    for name in names:
        path = folder/name
        if name in assets:
            if assets[name].get('digest') != 'sha256:'+checksum(path):
                raise ValueError('Existing draft asset differs; it will not be overwritten: '+name)
            continue
        url = release['upload_url'].split('{',1)[0]+'?'+urllib.parse.urlencode({'name':name})
        uploaded = api.call(url, path.read_bytes(), method='POST', upload=True)
        if uploaded['state'] != 'uploaded' or uploaded['size'] != path.stat().st_size or (uploaded.get('digest') and uploaded['digest'] != 'sha256:'+checksum(path)):
            raise ValueError('Uploaded artifact failed verification: '+name)
    remote = api.call('releases/'+str(release['id']))
    uploaded_names = {asset['name'] for asset in remote['assets'] if asset['state'] == 'uploaded'}
    if not set(names).issubset(uploaded_names):
        raise ValueError('Remote release is missing verified assets')
    api.call('releases/'+str(release['id']), {'draft':False, 'prerelease':False}, method='PATCH')
    print('Published '+tag+' with verified Windows ZIP, corresponding source and checksums')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['plan','publish'])
    parser.add_argument('--dist', type=Path, default=ROOT/'dist')
    args = parser.parse_args()
    api = GitHub(os.environ['GITHUB_REPOSITORY'])
    if args.command == 'plan':
        plan(api)
    else:
        publish(api, args.dist)

# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Exercise release eligibility, immutable tags, preflight checks and real history notes."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import auto_release as release


class ReleaseTests(unittest.TestCase):
    def test_changed_version_releases_but_ordinary_commits_and_published_versions_skip(self):
        self.assertTrue(release.decision('0.2.1','0.2.0','push',None,None,'abc')[0])
        self.assertFalse(release.decision('0.2.1','0.2.1','push',None,None,'abc')[0])
        self.assertFalse(release.decision('0.2.1','0.2.0','push',{'draft':False},None,'abc')[0])

    def test_development_versions_are_prereleases(self):
        self.assertTrue(release.is_prerelease('0.2.1'))
        self.assertTrue(release.is_prerelease('1.0.0-beta.1'))
        self.assertFalse(release.is_prerelease('1.0.0'))
        self.assertFalse(release.is_prerelease('1.0.0+build-local'))

    def test_artifact_validation_precedes_any_network_publication(self):
        api=unittest.mock.Mock()
        with patch.object(release,'current',return_value=('0.2.1','verified')), \
             patch.dict(release.os.environ,{'RELEASE_TAG':'v0.2.1'}), \
             patch.object(release,'verify_artifacts',side_effect=ValueError('bad package')):
            with self.assertRaisesRegex(ValueError,'bad package'):
                release.publish(api,Path('unused'))
        api.call.assert_not_called()

    def test_immutable_tags_and_foreign_drafts_are_never_replaced(self):
        with self.assertRaisesRegex(ValueError,'will not be moved'):
            release.decision('0.2.1','0.2.0','push',None,'different','abc')
        with self.assertRaisesRegex(ValueError,'not owned'):
            release.decision('0.2.1','0.2.0','push',{'draft':True,'body':'foreign'},'abc','abc')
        own={'draft':True,'body':'<!-- retrohatch-release:abc -->'}
        self.assertTrue(release.decision('0.2.1',None,'workflow_dispatch',own,'abc','abc')[0])
        self.assertTrue(release.decision('0.2.1',None,'workflow_dispatch',None,'abc','abc')[0])

    def test_annotated_tags_resolve_without_modification(self):
        api=unittest.mock.Mock()
        api.call.side_effect=[{'object':{'type':'tag','sha':'annotation'}},{'object':{'type':'commit','sha':'verified'}}]
        self.assertEqual(release.tag_commit(api,'v0.2.1'),'verified')
        self.assertTrue(all('method' not in call.kwargs for call in api.call.call_args_list))

    def test_notes_categorize_actual_commit_subjects_and_preserve_limitations(self):
        api=unittest.mock.Mock();api.call.return_value=[]
        result=unittest.mock.Mock(stdout='a'*40+'\0feat: actual new feature\n'+'b'*40+'\0fix: actual correction\n'+'c'*40+'\0docs: actual guide\n')
        with patch.object(release,'git',return_value=result):
            notes=release.history_notes(api,'commit','v0.2.1')
        for expected in ('New features','Bug fixes','Important improvements','actual new feature','actual correction','Known limitations','no game audio'):
            self.assertIn(expected,notes)

    def test_permission_errors_never_masquerade_as_missing_releases(self):
        api=release.GitHub('owner/repo')
        error=release.urllib.error.HTTPError('https://api.github.com/test',403,'Denied',{},None)
        with patch.object(release.urllib.request,'urlopen',side_effect=error):
            with self.assertRaisesRegex(RuntimeError,'HTTP 403'):
                api.call('releases/tags/v0.2.1',missing=True)

    def test_publication_creates_tag_then_checked_draft_uploads_then_prerelease(self):
        class Fake:
            def __init__(self):
                self.tag=None;self.release=None;self.writes=[]
            def call(self,path,data=None,method=None,missing=False,upload=False):
                if method:self.writes.append((path,method))
                if path.startswith('git/ref/tags/'):
                    return {'object':{'type':'commit','sha':self.tag}} if self.tag else None
                if path=='git/refs':
                    if self.tag:raise AssertionError('Tag overwrite attempted')
                    self.tag=data['sha'];return {}
                if path.startswith('releases/tags/'):return self.release
                if path=='releases' and method=='POST':
                    self.release=dict(data,id=1,upload_url='https://uploads.github.com/asset{?name}',assets=[])
                    return self.release
                if upload:
                    name=release.urllib.parse.parse_qs(release.urllib.parse.urlsplit(path).query)['name'][0]
                    asset={'name':name,'state':'uploaded','size':len(data),'digest':'sha256:'+hashlib.sha256(data).hexdigest()}
                    self.release['assets'].append(asset);return asset
                if path=='releases/1':
                    if method=='PATCH':self.release.update(data)
                    return self.release
                raise AssertionError(path)
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);names=['Windows.zip','source.zip','SHA256SUMS.txt']
            for name in names:(root/name).write_bytes(b'synthetic verified artifact')
            (root/'RELEASE_NOTES.md').write_text('Installation',encoding='utf-8')
            api=Fake()
            with patch.object(release,'current',return_value=('0.2.1','verified')), \
                 patch.dict(release.os.environ,{'RELEASE_TAG':'v0.2.1'}), \
                 patch.object(release,'verify_artifacts',return_value=names), \
                 patch.object(release,'history_notes',return_value='Actual history'), \
                 patch('builtins.print'):
                release.publish(api,root)
                before=list(api.writes)
                release.publish(api,root)
            self.assertEqual(api.writes,before)  # Published versions are immutable/idempotent.
            self.assertFalse(api.release['draft']);self.assertTrue(api.release['prerelease'])
            self.assertEqual(api.tag,'verified');self.assertEqual(len(api.release['assets']),3)
            self.assertEqual(api.writes[0],('git/refs','POST'))
            self.assertEqual(api.writes[-1],('releases/1','PATCH'))

    def test_packages_must_match_commit_notices_checksums_and_offline_source(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);version='0.2.1';sha='verified';exe=b'synthetic executable'
            windows,source=release.release_names(version)
            required={'LICENSE','COPYRIGHT.md','THIRD_PARTY_LICENSES.md','SOURCE.md','LICENSES/CARGO_PACKAGES.json',
                      'LICENSES/CARGO_NOTICES.txt','app/LICENSES/BrickEmuPy-CC0-1.0.txt',
                      'app/LICENSES/TamaEmu-GPL-3.0.txt','app/third_party/tg18-core/LICENSE'}
            info=json.dumps({'revision':sha,'version':version,'local_draft':False,'executable_sha256':hashlib.sha256(exe).hexdigest()})
            for name in (windows,source):
                with zipfile.ZipFile(root/name,'w') as archive:
                    for file in required:archive.writestr(file,'synthetic notice')
                    archive.writestr('BUILD_INFO.json',info)
                    if name==windows:archive.writestr('RetroHatch.exe',exe)
                    else:
                        for file in ('app/Cargo.toml','app/Cargo.lock','app/build.rs','.cargo/config.toml','vendor/test/LICENSE'):
                            archive.writestr(file,'synthetic source')
            sums=''.join(release.checksum(root/name)+'  '+name+'\n' for name in (windows,source))
            (root/'SHA256SUMS.txt').write_text(sums,encoding='utf-8')
            self.assertEqual(len(release.verify_artifacts(root,version,sha)),3)
            with self.assertRaisesRegex(ValueError,'verified release commit'):
                release.verify_artifacts(root,version,'wrong')
            with (root/windows).open('ab') as file:file.write(b'tamper')
            with self.assertRaisesRegex(ValueError,'checksum mismatch'):
                release.verify_artifacts(root,version,sha)


if __name__=='__main__':unittest.main()

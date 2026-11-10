# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Focused privacy and archive checks for the release packager."""
from pathlib import Path
import os
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from package_release import release_names, source_allowed, validate_executable, write_zip


class PackagingTests(unittest.TestCase):
    def test_archive_names_follow_the_cargo_semantic_version(self):
        self.assertEqual(release_names("0.3.2"),
                         ("RetroHatch-v0.3.2-Windows.zip", "RetroHatch-v0.3.2-source.zip"))
        self.assertEqual(release_names("0.4.0-beta.1+local")[0],
                         "RetroHatch-v0.4.0-beta.1+local-Windows.zip")
        for version in ("0.3", "v0.3.2", "../0.3.2"):
            with self.assertRaises(ValueError):
                release_names(version)

    def test_required_source_and_notices_are_allowed(self):
        for name in ("LICENSE", "README.md", "app/build.rs", "docs/assets/retrohatchlogo.png", "app/Cargo.lock", "app/src/main.rs",
                     "app/assets/lcd/angel.alpha", "app/tests/data/e0c6200_reference.txt",
                     "LICENSES/CARGO_NOTICES.txt", ".github/workflows/windows-release.yml",
                     "app/third_party/tg18-core/LICENSE", "app/third_party/tg18-core/src/cpu.rs",
                     "app/tools/s1c33_reference.c", "app/src/core/s1c33/UPSTREAM.json",
                     "app/LICENSES/TamaEmu-GPL-3.0.txt"):
            with self.subTest(name=name):
                self.assertTrue(source_allowed(name))

    def test_private_files_and_path_traversal_are_rejected(self):
        for name in ("roms/game.bin", "resources/retrohatchicon.ico", "resources/retrohatchlogo.png", "docs/assets/private.png", "screenshots/full/capture.png", "references/license.txt", "docs/private.md",
                     "AGENTS.md", ".git/config", "saves/slot.rhstate", ".env",
                     "app/target/release/RetroHatch.exe", "app/tools/secret.key",
                     "app/assets/lcd/firmware.rom", "app/src/../../private.md",
                     "/app/src/main.rs", "C:/private.md", "app\\src\\main.rs"):
            with self.subTest(name=name):
                self.assertFalse(source_allowed(name))

    def test_archive_contains_only_staged_files_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage = root/"stage"
            stage.mkdir()
            (stage/"LICENSE").write_text("test notice", encoding="utf-8")
            (root/"private.rom").write_bytes(b"not firmware")
            archive = root/"package.zip"
            write_zip(archive, stage)
            with zipfile.ZipFile(archive) as package:
                self.assertEqual(package.namelist(), ["LICENSE"])
                self.assertEqual(package.read("LICENSE"), b"test notice")
                self.assertIsNone(package.testzip())
            with self.assertRaises(FileExistsError):
                write_zip(archive, stage)

    def test_vendored_files_with_pre_1980_timestamps_can_be_archived(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage = root/"stage"
            stage.mkdir()
            old_file = stage/"old-source.rs"
            old_file.write_text("// synthetic source", encoding="utf-8")
            os.utime(old_file, (0, 0))
            write_zip(root/"source.zip", stage)
            with zipfile.ZipFile(root/"source.zip") as package:
                self.assertEqual(package.getinfo("old-source.rs").date_time[0], 1980)
                self.assertEqual(package.read("old-source.rs"), b"// synthetic source")

    def test_system_libraries_are_accepted_without_bundling_dlls(self):
        with patch("package_release.pe_imports", return_value=["kernel32.dll", "opengl32.dll"]):
            self.assertEqual(validate_executable(b"image", []), ["kernel32.dll", "opengl32.dll"])

    def test_separate_msvc_runtime_dependency_is_rejected(self):
        with patch("package_release.pe_imports", return_value=["vcruntime140.dll"]):
            with self.assertRaisesRegex(ValueError, "Non-system DLLs"):
                validate_executable(b"image", [])

    def test_private_paths_are_rejected_in_both_encodings_and_slash_styles(self):
        private = r"C:\Users\private-name"
        with patch("package_release.pe_imports", return_value=["kernel32.dll"]):
            for encoding in ("utf-8", "utf-16-le"):
                for form in (private, private.replace("\\", "/")):
                    with self.subTest(encoding=encoding, form=form):
                        with self.assertRaisesRegex(ValueError, "private build-path"):
                            validate_executable(form.encode(encoding), [private])

    def test_non_executable_is_rejected(self):
        with self.assertRaises(ValueError):
            validate_executable(b"not a PE image", [])


if __name__ == "__main__":
    unittest.main()

# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Verify private path overrides and independent public build defaults."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import workspace_paths as paths


class WorkspaceTests(unittest.TestCase):
    def test_sibling_local_defaults_are_outside_source(self):
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(paths.local_dir(), paths.ROOT.parent / "local")
            self.assertEqual(paths.references_dir(), paths.ROOT.parent / "local/references")

    def test_custom_local_and_reference_roots(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            with patch.dict(os.environ, {"RETROHATCH_LOCAL_DIR": str(root)}, clear=True):
                self.assertEqual(paths.references_dir(), root / "references")
                with patch.dict(os.environ, {"RETROHATCH_REFERENCES_DIR": str(root / "upstream")}):
                    self.assertEqual(paths.references_dir(), root / "upstream")

    def test_public_checkout_uses_dist_without_creating_private_data(self):
        with tempfile.TemporaryDirectory() as temporary:
            local = Path(temporary) / "absent"
            with patch.dict(os.environ, {"RETROHATCH_LOCAL_DIR": str(local)}, clear=True):
                self.assertEqual(paths.release_dir(), paths.ROOT / "dist")
                self.assertFalse(local.exists())

    def test_local_distributions_stay_outside_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            with patch.dict(os.environ, {"RETROHATCH_LOCAL_DIR": temporary}, clear=True):
                self.assertEqual(paths.release_dir(), Path(temporary).resolve() / "releases")

    def test_cargo_resolves_build_configuration_and_target_override(self):
        with patch("workspace_paths.subprocess.check_output", return_value=json.dumps({"target_directory": "/custom/target"})) as call:
            self.assertEqual(paths.target_dir(), Path("/custom/target"))
            self.assertIn("--offline", call.call_args.args[0])
            self.assertEqual(call.call_args.kwargs["cwd"], paths.ROOT / "app")


if __name__ == "__main__":
    unittest.main()

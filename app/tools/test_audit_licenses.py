# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Verify complete license evidence across Windows checkouts and real edits."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import audit_licenses as audit


class InventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline, cls.count = audit.generate()
        cls.license = (audit.ROOT / "app/third_party/tg18-core/LICENSE").resolve()
        cls.read_bytes = staticmethod(Path.read_bytes)

    def generate_with_license_bytes(self, transform):
        original_read = self.read_bytes
        license_path = self.license

        def read(path):
            data = original_read(path)
            return transform(data) if path.resolve() == license_path else data

        with patch.object(Path, "read_bytes", read):
            return audit.generate()

    def test_windows_crlf_checkout_preserves_every_notice_and_hash(self):
        outputs, count = self.generate_with_license_bytes(
            lambda data: data.replace(b"\r\n", b"\n").replace(b"\n", b"\r\n"))
        self.assertEqual(count, self.count)
        self.assertEqual(outputs, self.baseline)
        full_license = self.read_bytes(self.license).decode("utf-8").replace("\r\n", "\n")
        self.assertIn(full_license, outputs[audit.ROOT / "LICENSES/CARGO_NOTICES.txt"])

    def test_actual_legal_wording_change_still_invalidates_evidence(self):
        marker = b"Permission is hereby granted"
        self.assertIn(marker, self.read_bytes(self.license))
        outputs, count = self.generate_with_license_bytes(
            lambda data: data.replace(marker, b"Changed permission is hereby granted", 1))
        self.assertEqual(count, self.count)
        key = audit.ROOT / "LICENSES/CARGO_PACKAGES.json"
        old = next(p for p in json.loads(self.baseline[key])["packages"] if p["name"] == "tg18-core")
        new = next(p for p in json.loads(outputs[key])["packages"] if p["name"] == "tg18-core")
        self.assertNotEqual(old["notice_sections"]["LICENSE"], new["notice_sections"]["LICENSE"])
        self.assertNotEqual(outputs[key], self.baseline[key])
        key = audit.ROOT / "LICENSES/CARGO_NOTICES.txt"
        self.assertNotEqual(outputs[key], self.baseline[key])


if __name__ == "__main__":
    unittest.main()

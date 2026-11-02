# SPDX-FileCopyrightText: 2026 Slavi Pantaleev
#
# SPDX-License-Identifier: Apache-2.0

import importlib.util
import pathlib
import unittest

spec = importlib.util.spec_from_file_location("platforms", pathlib.Path(__file__).with_name("check-platforms.py"))
platforms = importlib.util.module_from_spec(spec)
spec.loader.exec_module(platforms)


def index(*architectures):
    return {"manifests": [{"platform": {"os": "linux", "architecture": arch}} for arch in architectures]}


class SupportedPlatforms(unittest.TestCase):
    def test_accepts_both_architectures_and_ignores_attestations(self):
        platforms.validate(index("amd64", "arm64", "unknown"))

    def test_rejects_missing_duplicate_and_unexpected_platforms(self):
        for architectures in [("amd64",), ("arm64",), ("amd64", "amd64"), ("amd64", "arm64", "arm")]:
            with self.subTest(architectures=architectures), self.assertRaises(ValueError):
                platforms.validate(index(*architectures))

    def test_matches_tested_child_digests_and_accepts_arm64_v8(self):
        document = index("amd64", "arm64", "unknown")
        expected = {"amd64": "example.invalid/image@sha256:" + "a" * 64,
                    "arm64": "sha256:" + "b" * 64}
        document["manifests"][0]["digest"] = "sha256:" + "a" * 64
        document["manifests"][1]["digest"] = "sha256:" + "b" * 64
        document["manifests"][1]["platform"]["variant"] = "v8"
        platforms.validate(document, expected)

    def test_rejects_missing_or_swapped_child_digests(self):
        expected = {"amd64": "sha256:" + "a" * 64, "arm64": "sha256:" + "b" * 64}
        for digest in [None, "sha256:" + "b" * 64, "sha256:" + "c" * 64]:
            document = index("amd64", "arm64")
            document["manifests"][0]["digest"] = digest
            document["manifests"][1]["digest"] = expected["arm64"]
            with self.subTest(digest=digest), self.assertRaises(ValueError):
                platforms.validate(document, expected)

    def test_rejects_incomplete_and_invalid_tested_references(self):
        document = index("amd64", "arm64")
        for expected in [{"amd64": "sha256:" + "a" * 64},
                         {"amd64": "example.invalid/image:tag", "arm64": "sha256:" + "b" * 64}]:
            with self.subTest(expected=expected), self.assertRaises(ValueError):
                platforms.validate(document, expected)


if __name__ == "__main__":
    unittest.main()

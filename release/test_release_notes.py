# SPDX-FileCopyrightText: 2026 Slavi Pantaleev
#
# SPDX-License-Identifier: Apache-2.0

import importlib.util
import os
import pathlib
import re
import subprocess
import tempfile
import unittest
import unittest.mock

spec = importlib.util.spec_from_file_location("notes", pathlib.Path(__file__).with_name("release-notes.py"))
notes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notes)

URL = "https://github.com/o/r"


def commit(sha, subject, author="Someone", notes_=(), breaking=(), reverts=()):
    return {"sha": sha * 40, "author": author, "subject": subject, "notes": list(notes_), "breaking": list(breaking),
            "reverts": list(reverts)}


def render(commits, image_commits, previous="v2026.10.30.0"):
    return notes.render(commits, {sha * 40 for sha in image_commits}, url=URL, previous=previous, tag="v2026.11.02.0")


def entries(body):
    """Each entry's text and the short hashes of its commits."""
    return [(match[1], re.findall(r"\[([0-9a-z]{7})\]", match[2]))
            for match in re.finditer(r"^- (.*) \((\[[0-9a-z]{7}\].*)\)$", body, re.MULTILINE)]


class Render(unittest.TestCase):
    def test_lists_image_commits_by_section(self):
        body = render([commit("a", "Fix a crash"), commit("b", "CI: tweak"), commit("c", "Update rust", author="renovate[bot]")],
                      ["a", "c"])
        self.assertEqual(body, "## Changes\n\n"
                               f"- Fix a crash ([aaaaaaa]({URL}/commit/{'a' * 40}))\n\n"
                               "## Dependency updates\n\n"
                               f"- Update rust ([ccccccc]({URL}/commit/{'c' * 40}))\n\n"
                               f"**Full changes**: {URL}/compare/v2026.10.30.0...v2026.11.02.0\n")

    def test_trailers_reword_include_skip_and_flag_breaking(self):
        body = render([commit("a", "Refactor x", notes_=["Faster startup"]),
                       commit("b", "Document y", notes_=["Document hooks"]),
                       commit("c", "Rename key", breaking=["`store` is now `storage`; rename it before upgrading."]),
                       commit("d", "Tidy z", notes_=["skip"])],
                      ["a", "c", "d"])
        self.assertIn("## Breaking changes\n\n- Rename key ([ccccccc]", body)
        self.assertIn("\n  `store` is now `storage`; rename it before upgrading.\n", body)
        self.assertIn("- Faster startup ([aaaaaaa]", body)
        self.assertIn("- Document hooks ([bbbbbbb]", body)
        self.assertNotIn("Tidy", body)
        self.assertNotIn("Refactor", body)
        self.assertLess(body.index("Breaking changes"), body.index("## Changes"))

    def test_lists_each_bot_updated_dependency_once_at_its_newest_version(self):
        body = render([commit("a", "Update dependency libfoo to v1.2.0", author="example-renovate[bot]"),
                       commit("b", "Update Rust crate bar to 0.5.*", author="dependabot[bot]"),
                       commit("c", "Update dependency libfoo to v1.3.0", author="example-renovate[bot]"),
                       commit("d", "Update libfoo to v2.0.0"),
                       commit("e", "Update Rust crate bar to 0.5.*", author="dependabot[bot]")],
                      "abcde")
        self.assertEqual(entries(body), [("Update libfoo to v2.0.0", ["d" * 7]),
                                         ("Update dependency libfoo to v1.3.0", ["c" * 7]),
                                         ("Update Rust crate bar to 0.5.*", ["b" * 7, "e" * 7])])
        self.assertIn("## Changes\n\n- Update libfoo to v2.0.0", body)

    def test_merges_identical_entries_and_consecutive_version_bumps(self):
        body = render([commit("a", "Update submodules"), commit("b", "Upgrade Foo (v1.0-1 -> v1.0-2)"),
                       commit("c", "Lock file maintenance", author="renovate[bot]"), commit("d", "Update submodules"),
                       commit("e", "Upgrade Foo (v1.0-2 -> v1.0-3)"), commit("f", "Update Bar (v2 → v3)"),
                       commit("g", "Lock file maintenance", author="renovate[bot]"), commit("h", "Upgrade Foo (v1.0-3 -> v1.1-0)"),
                       commit("i", "Upgrade Foo (v7 -> v8)")],
                      "abcdefghi")
        self.assertEqual(entries(body), [("Update submodules", ["a" * 7, "d" * 7]),
                                         ("Upgrade Foo (v1.0-1 -> v1.1-0)", ["b" * 7, "e" * 7, "h" * 7]),
                                         ("Update Bar (v2 → v3)", ["f" * 7]),
                                         ("Upgrade Foo (v7 -> v8)", ["i" * 7]),
                                         ("Lock file maintenance", ["c" * 7, "g" * 7])])
        self.assertIn(f"- Update submodules ([aaaaaaa]({URL}/commit/{'a' * 40}), [ddddddd]({URL}/commit/{'d' * 40}))\n", body)

    def test_leaves_out_changes_reverted_in_the_same_release(self):
        body = render([commit("a", "Old change"), commit("b", "Add x"), commit("c", "Revert \"Add x\"", reverts=["b" * 7]),
                       commit("d", "Revert an older change", reverts=["0" * 40]),
                       commit("e", "Revert the old change", reverts=["a" * 12])],
                      "abcde")
        self.assertEqual([text for text, _ in entries(body)], ["Revert an older change"])

    def test_reverting_a_revert_restores_the_original_entry(self):
        body = render([commit("a", "Rename key", breaking=["Rename it."]), commit("b", "Revert key rename", reverts=["a" * 7]),
                       commit("c", "Reapply key rename", reverts=["b" * 7])],
                      "abc")
        self.assertEqual(entries(body), [("Rename key", ["a" * 7])])
        self.assertIn("\n  Rename it.\n", body)

    def test_keeps_reverts_with_their_own_trailers(self):
        body = render([commit("a", "Add x"), commit("b", "Revert x", notes_=["Fix startup"], reverts=["a" * 7])], "ab")
        self.assertEqual([text for text, _ in entries(body)], ["Add x", "Fix startup"])

    def test_counts_only_full_reverts(self):
        self.assertEqual(notes.reverts("Revert \"Add x\"", "Revert \"Add x\"\n\nThis reverts commit abcdef0123."), ["abcdef0123"])
        self.assertEqual(notes.reverts("Reapply \"Add x\"", "Reapply \"Add x\"\n\nThis reverts commit 0123abc."), ["0123abc"])
        self.assertEqual(notes.reverts("Revert x", "Revert x\n\nReverts abcdef0, partially."), [])
        self.assertEqual(notes.reverts("Document x", "Document x\n\nDo not revert commit abcdef0."), [])

    def test_keeps_commit_text_from_mentioning_people_or_vanishing(self):
        body = render([commit("a", "Update dependency @types/node to v26 (#12)", author="renovate[bot]"),
                       commit("b", "Update dependency @types/node to v27 (#13)", author="renovate[bot]"),
                       commit("c", "Document <name> for `@example` and mail@example.com")],
                      "abc")
        self.assertEqual([text for text, _ in entries(body)], [
            "Document &lt;name> for `@example` and mail@example.com", "Update dependency `@types/node` to v27 (#13)"])

    def test_puts_breaking_explanations_in_their_own_paragraph(self):
        body = render([commit("a", "Rename key", breaking=["Rename it."])], "a")
        self.assertIn(f"- Rename key ([aaaaaaa]({URL}/commit/{'a' * 40}))\n\n  Rename it.\n", body)

    def test_handles_empty_and_first_releases(self):
        self.assertEqual(render([commit("a", "CI: tweak")], [], previous="none"),
                         f"No notable changes.\n\n**Full changes**: {URL}/commits/v2026.11.02.0\n")


class Read(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        # Ignores the user's Git configuration (commit signing, hooks),
        # and the GIT_* variables of a Git hook running these tests (e.g. GIT_INDEX_FILE).
        env = {name: value for name, value in os.environ.items() if not name.startswith("GIT_")}
        env |= {"GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1", "GIT_DIR": f"{directory.name}/.git",
                "GIT_AUTHOR_NAME": "renovate[bot]", "GIT_AUTHOR_EMAIL": "r@example.com",
                "GIT_COMMITTER_NAME": "c", "GIT_COMMITTER_EMAIL": "c@example.com"}
        patch = unittest.mock.patch.dict(os.environ, env, clear=True)
        patch.start()
        self.addCleanup(patch.stop)
        self.git("init", "--quiet", "--initial-branch=main")

    def git(self, *args):
        return notes.git(*args).strip()

    def commit(self, message):
        self.git("commit", "--quiet", "--allow-empty", "--message", message)
        return self.git("rev-parse", "HEAD")

    def read(self, span="HEAD"):
        return [notes.read(sha) for sha in self.git("rev-list", "--no-merges", "--topo-order", "--reverse", span).split()]

    def test_reads_subjects_and_folded_trailers(self):
        self.commit("Plain \x1e subject\n\nBody without trailers.")
        self.commit("Rename key\n\nBody.\n\nRelease-note: Rename `store`\x1fskip\n"
                    "Breaking-change: Configs using `store` fail to load;\n  rename it to `storage`.\nRelease-note: to `storage`")
        self.assertEqual([(c["author"], c["subject"], c["notes"], c["breaking"]) for c in self.read()], [
            ("renovate[bot]", "Plain \x1e subject", [], []),
            ("renovate[bot]", "Rename key", ["Rename `store`\x1fskip", "to `storage`"],
             ["Configs using `store` fail to load; rename it to `storage`."]),
        ])

    def test_resolves_reverted_hashes_and_merged_branches(self):
        base = self.commit("Start")
        change = self.commit("Add x")
        self.commit(f"Revert x\n\nIt reverts commit {change[:9]}.")
        self.commit("Revert something unknown\n\nThis reverts commit 0123456789abcdef.")
        self.git("checkout", "--quiet", "-b", "update")
        update = self.commit("Update dependency libfoo to v2")
        self.git("checkout", "--quiet", "main")
        self.git("merge", "--quiet", "--no-ff", "--no-edit", "update")
        merge = self.git("rev-parse", "HEAD")
        self.commit(f"Revert \"Merge branch 'update'\"\n\nThis reverts commit {merge}, reversing\nchanges made to x.")
        self.assertEqual([c["reverts"] for c in self.read(f"{base}..HEAD")], [[], [change], [], [], [update]])


if __name__ == "__main__":
    unittest.main()

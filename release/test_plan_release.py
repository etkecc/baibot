# SPDX-FileCopyrightText: 2026 Slavi Pantaleev
#
# SPDX-License-Identifier: Apache-2.0

import datetime
import importlib.util
import pathlib
import unittest
import unittest.mock

spec = importlib.util.spec_from_file_location("plan", pathlib.Path(__file__).with_name("plan-release.py"))
plan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan)

TODAY = datetime.date(2026, 10, 22)
MINUTE = datetime.timedelta(minutes=1)


def comparison(*filenames, ahead_by=1, **file_fields):
    return {"ahead_by": ahead_by, "files": [{"filename": name, **file_fields} for name in filenames]}


class ImagePaths(unittest.TestCase):
    def test_reads_this_repositorys_dockerfile(self):
        """Fails when a Dockerfile change makes the planner choke, or names paths that do not exist."""
        root = pathlib.Path(__file__).resolve().parent.parent
        paths = plan.image_paths((root / "Dockerfile").read_text())
        self.assertIn("src", paths)
        for path in paths:
            with self.subTest(path=path):
                self.assertTrue((root / path).exists())

    def test_uses_build_context_sources_only(self):
        dockerfile = "\n".join(["FROM rust AS build", "COPY --chown=1:1 a b/ /app/", "COPY ./c/ ./c/  # comment",
                                "copy d /d", "COPY --from=build /x /y", "RUN cp a b \\", "  && true", "COPY . /app",
                                "COPY ././e//f/ /e"])
        self.assertEqual(plan.image_paths(dockerfile), [".", ".dockerignore", "Dockerfile", "a", "b", "c", "d", "e/f"])

    def test_refuses_forms_it_cannot_read(self):
        for dockerfile in ["COPY a \\\n  /app/", 'COPY ["a", "/app/"]', "COPY /etc/x /x", "COPY ../x /x",
                           "ADD https://example.com/x /x", "COPY /app/", "FROM rust\nRUN true"]:
            with self.subTest(dockerfile=dockerfile), self.assertRaises(ValueError):
                plan.image_paths(dockerfile)


class ChangesImage(unittest.TestCase):
    PATHS = ["Cargo.lock", "Dockerfile", "src"]

    def test_changes_under_image_paths(self):
        self.assertTrue(plan.changes_image(comparison("docs/x.md", "src/main.rs"), self.PATHS))
        self.assertTrue(plan.changes_image(comparison("Cargo.lock"), self.PATHS))
        self.assertFalse(plan.changes_image(comparison("docs/x.md", "srcx/main.rs", ".github/renovate.json"), self.PATHS))
        self.assertFalse(plan.changes_image(comparison("src/main.rs", ahead_by=0), self.PATHS))
        self.assertTrue(plan.changes_image(comparison("docs/x.md"), ["."]))

    def test_renames_and_truncated_lists_count(self):
        self.assertTrue(plan.changes_image(comparison("docs/x.rs", previous_filename="src/x.rs"), self.PATHS))
        self.assertTrue(plan.changes_image(comparison(*["docs/x.md"] * plan.COMPARE_FILES_LIMIT), self.PATHS))


def pr(**fields):
    return {"isDraft": False, "mergeable": "MERGEABLE", "reviewDecision": "", "statusCheckRollup": [], **fields}


class NextVersion(unittest.TestCase):
    def test_starts_each_day_at_zero_and_ignores_legacy_tags(self):
        self.assertEqual(plan.next_version(["1.0", "v2.0.0", "v3.1.0", "v2026.10.21.4"], TODAY), "2026.10.22.0")

    def test_counts_numerically_within_the_day(self):
        tags = [f"v2026.10.22.{n}" for n in range(11)]
        self.assertEqual(plan.next_version(tags, TODAY), "2026.10.22.11")

    def test_refuses_to_go_backward(self):
        with self.assertRaises(ValueError):
            plan.next_version(["v2026.10.23.0"], TODAY)


class LandingSoon(unittest.TestCase):
    def test_green_or_running_prs_may_land(self):
        self.assertTrue(plan.may_land_soon(pr()))
        self.assertTrue(plan.may_land_soon(pr(statusCheckRollup=[{"status": "IN_PROGRESS", "conclusion": ""},
                                                                   {"conclusion": "SUCCESS"}, {"state": "PENDING"}])))

    def test_stuck_prs_do_not_hold_releases(self):
        for stuck in [pr(isDraft=True), pr(mergeable="CONFLICTING"), pr(reviewDecision="CHANGES_REQUESTED"),
                      pr(statusCheckRollup=[{"conclusion": "SUCCESS"}, {"conclusion": "FAILURE"}]),
                      pr(statusCheckRollup=[{"state": "ERROR"}])]:
            with self.subTest(stuck=stuck):
                self.assertFalse(plan.may_land_soon(stuck))


class BranchMayLand(unittest.TestCase):
    def test_branches_ahead_of_main_may_land_unless_their_ci_failed(self):
        for ci in ["success", "in_progress", "queued", "missing"]:
            with self.subTest(ci=ci):
                self.assertTrue(plan.branch_may_land(True, ci))
        self.assertFalse(plan.branch_may_land(True, "failure"))
        self.assertFalse(plan.branch_may_land(True, "cancelled"))
        self.assertFalse(plan.branch_may_land(False, "success"))


class CiRuns(unittest.TestCase):
    def test_results_and_push_times(self):
        ci = {"conclusion": None, "status": "in_progress", "created_at": "2026-10-22T10:00:00Z"}
        self.assertEqual(plan.ci_result(ci), "in_progress")
        self.assertEqual(plan.ci_result({**ci, "conclusion": "success"}), "success")
        self.assertEqual(plan.ci_result(None), "missing")
        self.assertEqual(plan.pushed_at(ci), datetime.datetime(2026, 10, 22, 10, tzinfo=datetime.UTC))


class WaitQuietly(unittest.TestCase):
    def wait(self, heads, manual=lambda: False, minutes=3):
        clock = [0.0]
        heads = iter(heads)
        with unittest.mock.patch.object(plan.time, "monotonic", lambda: clock[0]), \
                unittest.mock.patch.object(plan.time, "sleep", lambda seconds: clock.__setitem__(0, clock[0] + seconds)), \
                unittest.mock.patch.object(plan, "main_head", lambda repository: next(heads)), \
                unittest.mock.patch.object(plan, "manual_run_waiting", lambda repository: manual()):
            return plan.wait_quietly("o/r", "abc", minutes * MINUTE), clock[0]

    def test_waits_the_whole_period_while_nothing_happens(self):
        self.assertEqual(self.wait(["abc"] * 3), (None, 180))

    def test_stops_when_main_moves_on_or_a_manual_run_waits(self):
        stopped, slept = self.wait(["abc", "def"])
        self.assertIn("moved on to def", stopped)
        self.assertEqual(slept, 120)
        stopped, slept = self.wait(["abc"] * 3, manual=lambda: True)
        self.assertIn("manual", stopped)
        self.assertEqual(slept, 60)


class Decide(unittest.TestCase):
    def decide(self, **overrides):
        return plan.decide(**{"forced": False, "changed": True, "ci": "success", "age": 40 * MINUTE, "landing": [], **overrides})[0]

    def test_releases_a_green_change_once_main_is_quiet(self):
        self.assertTrue(self.decide())
        self.assertTrue(self.decide(age=30 * MINUTE))
        self.assertFalse(self.decide(age=29 * MINUTE))
        self.assertFalse(self.decide(age=MINUTE))

    def test_only_the_quiet_period_asks_to_wait(self):
        wait = lambda **overrides: plan.decide(**{"forced": False, "changed": True, "ci": "success", "age": 10 * MINUTE,
                                                  "landing": [], **overrides})[2]
        self.assertEqual(wait(), 20 * MINUTE)
        self.assertEqual(wait(landing=["#36"]), 20 * MINUTE)
        for overrides in [{"age": 30 * MINUTE}, {"forced": True}, {"ci": "in_progress"}, {"changed": False}]:
            with self.subTest(**overrides):
                self.assertIsNone(wait(**overrides))

    def test_never_releases_unchanged_images(self):
        self.assertFalse(self.decide(changed=False))
        self.assertFalse(self.decide(changed=False, forced=True))

    def test_waits_for_green_ci_and_pending_renovate_prs(self):
        self.assertFalse(self.decide(ci="failure"))
        self.assertFalse(self.decide(ci="in_progress", age=200 * MINUTE))
        self.assertFalse(self.decide(landing=["#36"]))

    def test_cap_overrides_pending_prs(self):
        self.assertTrue(self.decide(landing=["#36"], age=90 * MINUTE))

    def test_manual_requests_skip_waiting_but_not_ci(self):
        self.assertTrue(self.decide(forced=True, age=MINUTE, landing=["#36"]))
        self.assertFalse(self.decide(forced=True, ci="failure"))
        self.assertFalse(self.decide(forced=True, ci="in_progress"))


if __name__ == "__main__":
    unittest.main()

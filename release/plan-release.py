#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Slavi Pantaleev
#
# SPDX-License-Identifier: Apache-2.0
"""Decide whether the main commit this workflow run is for should be released now.

Releases are CalVer tags (vYYYY.MM.DD.N, UTC date, N counting from 0 per day) cut without a human choosing a version.
To fold a burst of commits into 1 release, an unforced run waits until main's head was pushed at least QUIET_PERIOD ago
(sleeping in the run, since GitHub delays or drops the scheduled runs that would retry),
and while another Renovate update to the image may still land (an open PR, or a branch that Renovate merges itself
once its CI passes) until the head was pushed QUIET_CAP ago.
The image's paths come from the Dockerfile's COPY sources; Renovate updates elsewhere never hold a release back.
Writes `release`, `version`, `previous` (the previous release tag, or "none") and `reason` to $GITHUB_OUTPUT.
"""
import datetime
import json
import os
import posixpath
import re
import shlex
import subprocess
import sys
import time

# The workflow whose push runs must pass, on main and on Renovate's branches.
CI_WORKFLOW = "ci.yml"
QUIET_PERIOD = datetime.timedelta(minutes=30)
QUIET_CAP = datetime.timedelta(minutes=90)
CALVER = re.compile(r"v?([0-9]{4})\.([0-9]{2})\.([0-9]{2})\.([0-9]+)")
FAILED = {"FAILURE", "CANCELLED", "TIMED_OUT", "ACTION_REQUIRED", "STARTUP_FAILURE", "ERROR"}
# GitHub lists at most this many files in a comparison; a list this long may be cut short.
COMPARE_FILES_LIMIT = 300


def parse(version):
    match = CALVER.fullmatch(version)
    return tuple(map(int, match.groups())) if match else None


def next_version(tags, today):
    released = [parse(tag) for tag in tags if tag.startswith("v") and parse(tag)]
    date = (today.year, today.month, today.day)
    if any(version[:3] > date for version in released):
        raise ValueError("a release tag is dated after today; refusing to go backward")
    counters = [version[3] for version in released if version[:3] == date]
    return f"{today:%Y.%m.%d}.{max(counters) + 1 if counters else 0}"


def image_paths(dockerfile):
    """The paths that end up in the published image: the build-context sources of the Dockerfile's COPY/ADD
    instructions (not those copied from another stage or image), plus the Dockerfile and .dockerignore.
    Raises ValueError for COPY/ADD forms it cannot read reliably, rather than guessing."""
    paths = {"Dockerfile", ".dockerignore"}
    found = False
    for line in dockerfile.splitlines():
        if line.upper().split()[:1] not in (["COPY"], ["ADD"]):
            continue
        if line.rstrip().endswith("\\") or "[" in line:
            raise ValueError(f"unsupported COPY/ADD form (keep it on 1 line, without JSON syntax): {line}")
        words = shlex.split(line, comments=True)[1:]
        if any(word.startswith("--from=") for word in words):
            continue
        sources = [word for word in words if not word.startswith("--")][:-1]
        if not sources:
            raise ValueError(f"COPY/ADD without a source: {line}")
        for source in sources:
            if "://" in source or source.startswith("/") or ".." in source.split("/"):
                raise ValueError(f"COPY/ADD source outside the build context: {line}")
            paths.add(posixpath.normpath(source))
            found = True
    if not found:
        raise ValueError("the Dockerfile copies nothing from the build context")
    return sorted(paths)


def changes_image(comparison, paths):
    """Whether a Renovate update changes the image. `comparison` is GitHub's comparison of main with its head:
    a file list GitHub may have cut short counts as changing it, and a rename counts by its old name too."""
    if comparison["ahead_by"] == 0:
        return False
    files = comparison.get("files") or []
    if len(files) >= COMPARE_FILES_LIMIT:
        return True
    changed = [file[key] for file in files for key in ("filename", "previous_filename") if file.get(key)]
    return any(path == "." or name == path or name.startswith(path + "/") for name in changed for path in paths)


def may_land_soon(pr):
    """A Renovate PR that could still be merged (not draft, conflicted, rejected or red)."""
    if pr.get("isDraft") or pr.get("mergeable") == "CONFLICTING" or pr.get("reviewDecision") == "CHANGES_REQUESTED":
        return False
    return not any((check.get("conclusion") or check.get("state")) in FAILED
                   for check in pr.get("statusCheckRollup") or [])


def branch_may_land(changes, ci):
    """A Renovate branch changing the image, which Renovate merges itself once its CI passes (unless it failed)."""
    return changes and ci.upper() not in FAILED


def minutes(age):
    return f"{int(age.total_seconds() // 60)} min"


def decide(*, forced, changed, ci, age, landing):
    """Returns (release, reason, wait): `wait` is how long until the quiet period alone stops blocking, else None."""
    if not changed:
        return False, "nothing in the image changed since the last release", None
    if ci != "success":
        return False, f"CI for this commit is {ci}", None
    # A manual run skips only the waiting, never CI.
    if forced:
        return True, "manually requested", None
    if age >= QUIET_CAP:
        return True, f"main has been quiet for {minutes(age)} (cap reached)", None
    if age < QUIET_PERIOD:
        return (False, f"main changed {minutes(age)} ago; waiting until it has been quiet for {minutes(QUIET_PERIOD)}",
                QUIET_PERIOD - age)
    if landing:
        return False, "waiting for Renovate update(s) that may land soon: " + ", ".join(landing), None
    return True, f"CI passed, main has been quiet for {minutes(age)} and no Renovate update to the image is about to land", None


def run(*command):
    return subprocess.run(command, check=True, capture_output=True, text=True).stdout.strip()


def ci_run(repository, sha, branch=None):
    """The latest push run of CI_WORKFLOW for a commit (on `branch`, if given), or None."""
    query = f"head_sha={sha}&event=push&per_page=1" + (f"&branch={branch}" if branch else "")
    runs = json.loads(run("gh", "api", f"repos/{repository}/actions/workflows/{CI_WORKFLOW}/runs?{query}"))["workflow_runs"]
    return runs[0] if runs else None


def ci_result(ci):
    """A CI run's conclusion, its status while running, or "missing"."""
    return (ci["conclusion"] or ci["status"]) if ci else "missing"


def pushed_at(ci):
    """When the commit reached main: its push started the CI run (re-runs keep the run's creation time).
    A commit's own timestamp would make an old commit pushed just now look quiet already."""
    return datetime.datetime.fromisoformat(ci["created_at"].replace("Z", "+00:00"))


def main_head(repository):
    return json.loads(run("gh", "api", f"repos/{repository}/git/ref/heads/main"))["object"]["sha"]


def manual_run_waiting(repository):
    """Whether a manual Release run is queued behind this one (the workflow lets 1 run plan at a time)."""
    return any(json.loads(run("gh", "api", f"repos/{repository}/actions/workflows/release.yml/runs"
                                         f"?event=workflow_dispatch&branch=main&status={status}&per_page=1"))["total_count"] > 0
               for status in ("pending", "queued"))


def wait_quietly(repository, sha, wait):
    """Waits out the quiet period in this run: the cron that would otherwise retry is delayed or dropped by GitHub.
    Returns why it stopped early (main moved on, or a manual run should go first), or None after the full wait."""
    deadline = time.monotonic() + wait.total_seconds()
    while (left := deadline - time.monotonic()) > 0:
        time.sleep(min(60, left))
        if (head := main_head(repository)) != sha:
            return f"main has moved on to {head}; a later run decides for it"
        if manual_run_waiting(repository):
            return "a manual Release run is waiting; it decides instead"
    return None


def plan(repository, sha, forced):
    """Returns (release, reason, previous release, wait), `wait` as decide() returns it."""
    head = main_head(repository)
    if head != sha:
        return False, f"main has moved on to {head}; a later run decides for it", "none", None
    described = subprocess.run(["git", "describe", "--tags", "--abbrev=0", "--match", "v[0-9]*", sha],
                               capture_output=True, text=True)
    # Before the first release, everything in the image counts as changed.
    previous = described.stdout.strip() if described.returncode == 0 else "none"
    paths = image_paths(run("git", "show", f"{sha}:Dockerfile"))
    changed = previous == "none" or subprocess.run(["git", "diff", "--quiet", previous, sha, "--", *paths]).returncode != 0
    main_ci = ci_run(repository, sha, "main")
    ci = ci_result(main_ci)
    # Without a CI run nothing is released, so the age does not matter then.
    age = datetime.datetime.now(datetime.UTC) - pushed_at(main_ci) if main_ci else datetime.timedelta(0)
    prs = json.loads(run("gh", "pr", "list", "--repo", repository, "--base", "main", "--author", "app/renovate",
                         "--json", "number,headRefName,headRefOid,isDraft,mergeable,reviewDecision,statusCheckRollup"))

    def update_changes_image(tip):
        return changes_image(json.loads(run("gh", "api", f"repos/{repository}/compare/{sha}...{tip}")), paths)

    landing = [f"#{pr['number']}" for pr in sorted(prs, key=lambda pr: pr["number"])
               if may_land_soon(pr) and update_changes_image(pr["headRefOid"])]
    # Branches with a PR are judged by the PR above.
    branches = run("gh", "api", "--paginate", f"repos/{repository}/branches?per_page=100",
                   "--jq", '.[] | select(.name | startswith("renovate/")) | .name + " " + .commit.sha').splitlines()
    for name, tip in sorted(branch.split() for branch in branches):
        if name not in {pr["headRefName"] for pr in prs}:
            if branch_may_land(update_changes_image(tip), ci_result(ci_run(repository, tip))):
                landing.append(name)
    release, reason, wait = decide(forced=forced, changed=changed, ci=ci, age=age, landing=landing)
    return release, f"{reason} (previous release {previous})", previous, wait


def main():
    repository, sha, forced = os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_SHA"], os.environ.get("FORCE") == "true"
    if os.environ["GITHUB_REF"] != "refs/heads/main":
        sys.exit("Releases are only cut from main")
    release, reason, previous, wait = plan(repository, sha, forced)
    if wait:
        print(f"Waiting: {reason}", flush=True)
        stopped = wait_quietly(repository, sha, wait)
        if stopped:
            release, reason = False, stopped
        else:
            # Plan afresh: CI, Renovate updates and the image may have changed meanwhile.
            release, reason, previous, _ = plan(repository, sha, forced)
    version = next_version(run("git", "tag", "--list", "v*").splitlines(), datetime.datetime.now(datetime.UTC).date()) if release else ""
    print(f"{'Releasing ' + version if release else 'Not releasing'}: {reason}")
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"release={str(release).lower()}\nversion={version}\nprevious={previous}\nreason={reason}\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
        summary.write(f"{'Releasing `v' + version + '`' if release else 'Not releasing'}: {reason}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Slavi Pantaleev
#
# SPDX-License-Identifier: Apache-2.0
"""Decide whether the main commit this workflow run is for should be released now.

Releases are CalVer tags (vYYYY.MM.DD.N, UTC date, N counting from 0 per day) cut without a human choosing a version.
To fold a burst of Renovate merges into 1 release, an unforced run waits while another Renovate update may still land
(an open PR, or a branch that Renovate merges itself once its CI passes),
but never longer than QUIET_CAP after the last commit.
The image's paths come from the Dockerfile's COPY sources.
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

# The workflow whose push runs must pass, on main and on Renovate's branches.
CI_WORKFLOW = "ci.yml"
QUIET_CAP = datetime.timedelta(minutes=90)
CALVER = re.compile(r"v?([0-9]{4})\.([0-9]{2})\.([0-9]{2})\.([0-9]+)")
FAILED = {"FAILURE", "CANCELLED", "TIMED_OUT", "ACTION_REQUIRED", "STARTUP_FAILURE", "ERROR"}


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


def may_land_soon(pr):
    """A Renovate PR that could still be merged (not draft, conflicted, rejected or red)."""
    if pr.get("isDraft") or pr.get("mergeable") == "CONFLICTING" or pr.get("reviewDecision") == "CHANGES_REQUESTED":
        return False
    return not any((check.get("conclusion") or check.get("state")) in FAILED
                   for check in pr.get("statusCheckRollup") or [])


def branch_may_land(ahead, ci):
    """A Renovate branch with commits main lacks, which Renovate merges itself once its CI passes (unless it failed)."""
    return ahead and ci.upper() not in FAILED


def minutes(age):
    return f"{int(age.total_seconds() // 60)} min"


def decide(*, forced, changed, ci, age, landing):
    if not changed:
        return False, "nothing in the image changed since the last release"
    if ci != "success":
        return False, f"CI for this commit is {ci}"
    # A manual run skips only the waiting, never CI.
    if forced:
        return True, "manually requested"
    if age >= QUIET_CAP:
        return True, f"main has been quiet for {minutes(age)} (cap reached)"
    if landing:
        return False, "waiting for Renovate update(s) that may land soon: " + ", ".join(landing)
    return True, "CI passed and no Renovate PR is about to land"


def run(*command):
    return subprocess.run(command, check=True, capture_output=True, text=True).stdout.strip()


def ci_result(repository, sha):
    """The latest push run of CI_WORKFLOW for a commit: its conclusion, its status while running, or "missing"."""
    runs = json.loads(run("gh", "api", f"repos/{repository}/actions/workflows/{CI_WORKFLOW}/runs?head_sha={sha}&event=push&per_page=1"))["workflow_runs"]
    return (runs[0]["conclusion"] or runs[0]["status"]) if runs else "missing"


def main():
    repository, sha, forced = os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_SHA"], os.environ.get("FORCE") == "true"
    if os.environ["GITHUB_REF"] != "refs/heads/main":
        sys.exit("Releases are only cut from main")
    head = json.loads(run("gh", "api", f"repos/{repository}/git/ref/heads/main"))["object"]["sha"]
    previous = "none"
    if head != sha:
        release, reason = False, f"main has moved on to {head}; a later run decides for it"
    else:
        described = subprocess.run(["git", "describe", "--tags", "--abbrev=0", "--match", "v[0-9]*", sha],
                                   capture_output=True, text=True)
        # Before the first release, everything in the image counts as changed.
        previous = described.stdout.strip() if described.returncode == 0 else "none"
        paths = image_paths(run("git", "show", f"{sha}:Dockerfile"))
        changed = previous == "none" or subprocess.run(["git", "diff", "--quiet", previous, sha, "--", *paths]).returncode != 0
        committed = datetime.datetime.fromtimestamp(int(run("git", "log", "-1", "--format=%ct", sha)), datetime.UTC)
        age = datetime.datetime.now(datetime.UTC) - committed
        ci = ci_result(repository, sha)
        prs = json.loads(run("gh", "pr", "list", "--repo", repository, "--base", "main", "--author", "app/renovate",
                             "--json", "number,headRefName,isDraft,mergeable,reviewDecision,statusCheckRollup"))
        landing = [f"#{pr['number']}" for pr in sorted(prs, key=lambda pr: pr["number"]) if may_land_soon(pr)]
        # Branches with a PR are judged by the PR above.
        branches = run("gh", "api", "--paginate", f"repos/{repository}/branches?per_page=100",
                       "--jq", '.[] | select(.name | startswith("renovate/")) | .name + " " + .commit.sha').splitlines()
        for name, tip in sorted(branch.split() for branch in branches):
            if name not in {pr["headRefName"] for pr in prs}:
                ahead = json.loads(run("gh", "api", f"repos/{repository}/compare/{sha}...{tip}"))["ahead_by"] > 0
                if branch_may_land(ahead, ci_result(repository, tip)):
                    landing.append(name)
        release, reason = decide(forced=forced, changed=changed, ci=ci, age=age, landing=landing)
        reason += f" (previous release {previous})"
    version = next_version(run("git", "tag", "--list", "v*").splitlines(), datetime.datetime.now(datetime.UTC).date()) if release else ""
    print(f"{'Releasing ' + version if release else 'Not releasing'}: {reason}")
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"release={str(release).lower()}\nversion={version}\nprevious={previous}\nreason={reason}\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
        summary.write(f"{'Releasing `v' + version + '`' if release else 'Not releasing'}: {reason}\n")


if __name__ == "__main__":
    main()

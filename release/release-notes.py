#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Slavi Pantaleev
#
# SPDX-License-Identifier: Apache-2.0
"""Write the GitHub Release notes for a release from the commits since the previous one.

Best effort: each non-merge commit that touches the image becomes 1 entry, worded by its subject.
Optional commit trailers improve that:
`Release-note: <text>` rewords the entry (and includes commits outside the image, e.g. documentation),
`Release-note: skip` leaves the commit out,
and `Breaking-change: <what breaks and what to do>` lists it under "Breaking changes".
To keep long ranges readable, a commit reverted within the range is left out together with its revert,
repeated entries become 1 line linking all their commits:
identical texts and consecutive version bumps of 1 thing ("Upgrade <name> (<old> -> <new>)", shown from the first old to the last new version),
and of dependency bots' updates of 1 dependency ("Update <dependency> to <version>") only the newest is listed.
Usage: release-notes.py <previous tag or "none"> <commit> <new tag>; prints Markdown.
"""
import importlib.util
import os
import pathlib
import re
import subprocess
import sys

spec = importlib.util.spec_from_file_location("plan", pathlib.Path(__file__).with_name("plan-release.py"))
plan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan)

# Commits by dependency bots (e.g. renovate[bot], a self-hosted example-renovate[bot], dependabot[bot]).
# Author names are not verified: a faked one only moves an entry to "Dependency updates".
BOT = re.compile(r"renovate|dependabot", re.IGNORECASE)
# A squash merge appends the pull request: "Update <dependency> to <version> (#<number>)".
UPDATE = re.compile(r"Update (.+) to \S+(?: \(#\d+\))?")
BUMP = re.compile(r"(Upgrade|Update) (.+) \((\S+) (->|→) (\S+)\)")
# A revert's subject starts with "Revert" (or "Reapply" when `git revert` reverts a revert);
# its message names the (partial) hash it reverts, as `git revert`'s "This reverts commit <hash>." or reworded.
REVERT = re.compile(r"Revert\b|Reapply\b", re.IGNORECASE)
REVERTS = re.compile(r"\b[Rr]everts? (?:commit )?([0-9a-f]{7,40})\b")


def git(*args):
    # A commit message that is not UTF-8 must not stop the release.
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True, errors="replace").stdout


def trailer(sha, key):
    # Commit messages cannot contain NUL, so it separates values unambiguously.
    values = git("log", "-1", f"--format=%(trailers:key={key},valueonly,unfold,separator=%x00)", sha).split("\0")
    return [value.strip() for value in values if value.strip()]


def reverts(subject, message):
    """The hashes a full revert reverts; partial reverts revert nothing as far as the notes are concerned."""
    if not REVERT.match(subject) or "partial" in message.lower():
        return []
    return REVERTS.findall(message)


def resolve(hashes):
    """The full hashes of the non-merge commits that reverting these (partial) hashes undoes.

    Reverting a merged pull request names the merge commit, which stands for the commits it merged.
    """
    found = []
    for prefix in hashes:
        sha = subprocess.run(["git", "rev-parse", "--verify", "--quiet", f"{prefix}^{{commit}}"],
                             capture_output=True, text=True).stdout.strip()
        if not sha:
            continue
        merge = len(git("rev-list", "--parents", "-1", sha).split()) > 2
        found += git("rev-list", "--no-merges", f"{sha}^1..{sha}").split() if merge else [sha]
    return found


def read(sha):
    author, subject = git("log", "-1", "--format=%an%x00%s", sha).rstrip("\n").split("\0")
    return {"sha": sha, "author": author, "subject": subject,
            "notes": trailer(sha, "Release-note"), "breaking": trailer(sha, "Breaking-change"),
            "reverts": resolve(reverts(subject, git("log", "-1", "--format=%B", sha)))}


def markdown(text):
    """Keeps commit text from notifying people (`@scope/package`) or vanishing as HTML (`<placeholder>`)."""
    return re.sub(r"(?<![\w`])@[\w./-]*\w", lambda mention: f"`{mention[0]}`", text.replace("<", "&lt;"))


def line(entry, url):
    links = ", ".join(f"[{sha[:7]}]({url}/commit/{sha})" for sha in entry["shas"])
    # Each breaking change's explanation is its own paragraph under the entry.
    return "\n\n".join([f"- {markdown(entry['text'])} ({links})", *(f"  {markdown(breaking)}" for breaking in entry["breaking"])])


def reverted(commits):
    """Commits whose changes are not in the release: reverts within the range and what they revert."""
    gone, undoes = set(), {}
    for commit in commits:
        # A revert with trailers says something of its own: it stays listed, and so does what it reverts.
        if commit["notes"] or commit["breaking"]:
            continue
        targets = {other["sha"] for other in commits for prefix in commit["reverts"] if other["sha"].startswith(prefix)}
        if not targets:
            continue
        undoes[commit["sha"]] = targets
        gone.add(commit["sha"])
        for target in targets:
            if target in gone:
                # Reverting a revert brings back what that revert undid.
                gone -= undoes.get(target, set())
            else:
                gone.add(target)
    return gone


def render(commits, image_commits, *, url, previous, tag):
    sections = {"Breaking changes": {}, "Changes": {}, "Dependency updates": {}}
    # Version bumps continuing an entry's chain of versions, by (section, verb, name, newest version).
    chains = {}
    gone = reverted(commits)
    for commit in commits:
        if commit["sha"] in gone or any(note.lower() == "skip" for note in commit["notes"]):
            continue
        if not (commit["sha"] in image_commits or commit["notes"] or commit["breaking"]):
            continue
        sha, text = commit["sha"], " ".join(commit["notes"]) or commit["subject"]
        bot = BOT.search(commit["author"]) and not commit["notes"]
        section = "Breaking changes" if commit["breaking"] else "Dependency updates" if bot else "Changes"
        entries = sections[section]
        update, bump = UPDATE.fullmatch(text), BUMP.fullmatch(text)
        if commit["breaking"]:
            entries[sha] = {"text": text, "shas": [sha], "breaking": commit["breaking"]}
        elif bot and update:
            # A newer update supersedes the older ones.
            entry = entries.get(("update", update[1]))
            if entry and entry["text"] == text:
                entry["shas"].append(sha)
            else:
                entries[("update", update[1])] = {"text": text, "shas": [sha], "breaking": []}
        elif bump:
            verb, name, old, arrow, new = bump.groups()
            entry = chains.pop((section, verb, name, old), None)
            if entry is None:
                entry = entries[sha] = {"old": old, "shas": [], "breaking": []}
            entry["text"] = f"{verb} {name} ({entry['old']} {arrow} {new})"
            entry["shas"].append(sha)
            chains[(section, verb, name, new)] = entry
        else:
            entries.setdefault(("text", text), {"text": text, "shas": [], "breaking": []})["shas"].append(sha)
    body = [f"## {title}\n\n" + "\n".join(line(entry, url) for entry in entries.values())
            for title, entries in sections.items() if entries]
    if not body:
        body = ["No notable changes."]
    compare = f"{url}/compare/{previous}...{tag}" if previous != "none" else f"{url}/commits/{tag}"
    body.append(f"**Full changes**: {compare}")
    return "\n\n".join(body) + "\n"


def main():
    previous, sha, tag = sys.argv[1:]
    url = f"{os.environ.get('GITHUB_SERVER_URL', 'https://github.com')}/{os.environ['GITHUB_REPOSITORY']}"
    span = sha if previous == "none" else f"{previous}..{sha}"
    commits = [read(sha) for sha in git("rev-list", "--no-merges", "--topo-order", "--reverse", span).split()]
    paths = plan.image_paths(git("show", f"{sha}:Dockerfile"))
    image_commits = set(git("log", "--no-merges", "--format=%H", span, "--", *paths).split())
    sys.stdout.write(render(commits, image_commits, url=url, previous=previous, tag=tag))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Slavi Pantaleev
#
# SPDX-License-Identifier: Apache-2.0
"""Require both runtime platforms and the exact tested child manifest digests."""
import argparse
import json
import re
import sys


def validate(index, expected_digests=None):
    manifests = []
    for manifest in index.get("manifests", []):
        platform = manifest.get("platform", {})
        if platform.get("architecture") == "unknown":
            continue  # BuildKit provenance/SBOM attestations.
        variant = platform.get("variant", "")
        arch = platform.get("architecture")
        if arch == "arm64" and variant == "v8":
            variant = ""
        manifests.append(((platform.get("os"), arch, variant), manifest))
    platforms = [platform for platform, _ in manifests]
    expected = {("linux", "amd64", ""), ("linux", "arm64", "")}
    if len(platforms) != 2 or set(platforms) != expected:
        raise ValueError(f"expected exactly linux/amd64 and linux/arm64; got {platforms}")
    if expected_digests is not None:
        if set(expected_digests) != {"amd64", "arm64"}:
            raise ValueError("expected one tested digest for each supported architecture")
        for (_, arch, _), manifest in manifests:
            digest = expected_digests[arch].rsplit("@", 1)[-1]
            if not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
                raise ValueError(f"invalid tested digest for {arch}")
            if manifest.get("digest") != digest:
                raise ValueError(f"{arch} child digest does not match the tested image")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--amd64", help="tested amd64 digest or repository@digest")
    parser.add_argument("--arm64", help="tested arm64 digest or repository@digest")
    args = parser.parse_args()
    if bool(args.amd64) != bool(args.arm64):
        parser.error("provide both --amd64 and --arm64")
    digests = {"amd64": args.amd64, "arm64": args.arm64} if args.amd64 else None
    try:
        validate(json.load(sys.stdin), digests)
    except (ValueError, KeyError, TypeError) as error:
        sys.exit(str(error))
    print("Image index contains the required linux/amd64 and linux/arm64 images")

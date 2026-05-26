#!/usr/bin/env python3
"""HARDEN-03 D-22 — Compare cargo-geiger output against the allowlist.

Usage: check-geiger.py <allowlist.json> <geiger-output.json>
Exit 0 if all unsafe-using crates are on the allowlist; exit 1 otherwise.
"""

import json
import sys


def used_count(pkg: dict) -> int:
    u = pkg.get("unsafety", {}).get("used", {})

    def n(node):
        if isinstance(node, dict):
            return int(node.get("unsafe_", 0))
        return 0

    return (
        n(u.get("functions"))
        + n(u.get("exprs"))
        + n(u.get("item_impls"))
        + n(u.get("item_traits"))
        + n(u.get("methods"))
    )


def pkg_name(pkg: dict) -> str:
    # cargo-geiger 0.13 emits package.id.name; tolerate older schemas.
    pid = pkg.get("package", {}).get("id", {})
    if isinstance(pid, dict):
        return pid.get("name", "")
    if isinstance(pid, str):
        # "name version (source)" form
        return pid.split(" ", 1)[0]
    return ""


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: check-geiger.py allowlist.json geiger.json", file=sys.stderr)
        return 2
    with open(sys.argv[1], "r", encoding="utf-8") as f:
        allow = {e["name"] for e in json.load(f)["allowlist"]}
    with open(sys.argv[2], "r", encoding="utf-8") as f:
        geiger = json.load(f)

    offenders = []
    for pkg in geiger.get("packages", []):
        name = pkg_name(pkg)
        if not name:
            continue
        used = used_count(pkg)
        if used > 0 and name not in allow:
            offenders.append((name, used))

    if offenders:
        print("::error::Unsafe-using crates not on allowlist (HARDEN-03 D-22):")
        for n, c in sorted(set(offenders)):
            print(f"  - {n} (unsafe count: {c})")
        print(
            "Fix: add to cargo-geiger.json with a one-line reason, OR remove the dep."
        )
        return 1

    print(f"OK: all unsafe-using crates are on the allowlist ({len(allow)} entries).")
    return 0


if __name__ == "__main__":
    sys.exit(main())

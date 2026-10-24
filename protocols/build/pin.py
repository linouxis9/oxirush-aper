#!/usr/bin/env python3
"""Write the revision of oxirush-aper of build/revisions of a protocol crate wherever
its repository names it, or say where one names another.

    python3 ../oxirush-aper/protocols/build/pin.py           # write them
    python3 ../oxirush-aper/protocols/build/pin.py --check   # compare them

Run it from the directory of the crate. To move to another revision of oxirush-aper,
write the name of its commit in build/revisions, run this,
regenerate the bindings as CONTRIBUTING.md says, and commit the three together: with
`--check`, which CI runs, the script fails while a place names another revision, and the
Reproduce workflow while the generator at the revision of oxirush-aper writes other
bindings.

The workflows call those of oxirush-aper (`uses: linouxis9/oxirush-aper/...@COMMIT`):
they are the places that name its revision. The packages of `benches/`, `fuzz/` and
`oracle/` take oxirush-aper from the checkout beside the repository, whose tools they
build, and the crates come from crates.io.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path.cwd()
NAMES = ["oxirush-aper"]
PLACES = [
    ".github/workflows/ci.yml",
    ".github/workflows/reproduce.yml",
]
COMMIT = re.compile(r"[0-9a-f]{40}")


def recorded():
    """The revision of each crate that build/revisions records, in its order."""
    path = ROOT / "build/revisions"
    if not path.exists():
        raise SystemExit(f"{ROOT} has no build/revisions: run this from the directory of a crate")
    revisions = {}
    for line in path.read_text().splitlines():
        if line and not line.startswith("#"):
            name, revision = line.split()
            # The name of a commit, whole: a branch or a tag may move.
            if not COMMIT.fullmatch(revision):
                raise SystemExit(f"build/revisions: {name} {revision} is no name of a commit")
            revisions[name] = revision
    if list(revisions) != NAMES:
        raise SystemExit(f"build/revisions names {list(revisions)}, and not {NAMES}")
    return revisions


def written(lines, revisions, place):
    """`lines` with the revisions, and the number of the places that name one."""
    lines, taken = list(lines), 0
    for number, line in enumerate(lines):
        for name, revision in revisions.items():
            called = re.subn(rf"(linouxis9/{name}/\S+@)[0-9a-f]{{40}}\b", rf"\g<1>{revision}", line)
            if called[1]:
                lines[number], taken = called[0], taken + 1
                continue
            if f'"https://github.com/linouxis9/{name}"' not in line:
                continue
            taken += 1
            for at in (number, number + 1):
                if at < len(lines):
                    lines[at], count = re.subn(
                        r'(\brev ?= ?")[0-9a-f]{40}(")', rf"\g<1>{revision}\g<2>", lines[at]
                    )
                    if count:
                        break
            else:
                raise SystemExit(f"{place}:{number + 1}: {name} has no revision to write")
    return lines, taken


def main():
    check = sys.argv[1:] == ["--check"]
    if sys.argv[1:] not in ([], ["--check"]):
        raise SystemExit(__doc__)
    revisions, failed = recorded(), False
    for place in PLACES:
        path = ROOT / place
        text = path.read_text()
        lines, taken = written(text.split("\n"), revisions, place)
        if taken == 0:
            raise SystemExit(f"{place} names no revision of oxirush-aper")
        if not check:
            path.write_text("\n".join(lines))
            continue
        for number, (line, pinned) in enumerate(zip(text.split("\n"), lines)):
            if line != pinned:
                print(f"{place}:{number + 1} names another revision than build/revisions records")
                failed = True
    if failed:
        raise SystemExit("run ../oxirush-aper/protocols/build/pin.py")
    print("\n".join(f"{name} {revision}" for name, revision in revisions.items()))


main()

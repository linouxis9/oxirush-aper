"""The mutants of generator/ against its own tests and the bindings that it writes: a
mutant is caught when a test of generator/ fails, or when the bindings of oxirush-ngap, of
oxirush-s1ap or of TOY (protocols/toy/) that it writes, formatted, differ from those that
the generator writes unchanged, or when it fails to write them; missed when the tests pass
and the bindings are the same, and unviable when it does not build. The tests and TOY
check what the two protocols do not have. A mutant
that makes the tests or the generator run past 10 minutes, or take more than 8 GiB of
address space, fails, and is caught: without the limits it took the runner down. The
protocol crates are beside this repository, each with the ASN.1 of build/asn1.sha256
taken out of its specification. The missed ones are written to `generator-missed.txt`,
and the program exits with 1 when there is one.

    python3 oxirush-aper/.github/mutants-generator.py MUTANTS.json SHARD SHARDS

MUTANTS.json is what `cargo mutants --list --json --diff` writes in generator/; the
shard SHARD of SHARDS takes every SHARDS-th mutant from the SHARD-th. The `exclude_re` of
generator/.cargo/mutants.toml is applied here to every mutant too: cargo-mutants 27.1.0
lists a mutant that deletes a field of a struct expression whatever it says.
"""

import hashlib
import json
import os
import pathlib
import re
import signal
import subprocess
import sys
import tomllib

APER = pathlib.Path(__file__).resolve().parent.parent
PROTOCOLS = ["ngap", "s1ap", "toy"]
# What a run of the generator takes at most: thirty times what the unchanged one takes on
# a runner, and twice the memory that it needs.
SECONDS = 600
LIMIT = ["prlimit", "--as=8589934592"]


def ran(command, **options):
    """Whether `command` ends with success within SECONDS: one that runs longer is
    stopped, with the processes that it started."""
    process = subprocess.Popen(command, start_new_session=True, **options)
    try:
        return process.wait(timeout=SECONDS) == 0
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        return False


def written(protocol):
    """What the generator writes for a protocol crate and keeps: the bindings, and those
    of the oracle, which TOY does not keep."""
    bindings = [f"src/{protocol}.rs", "src/registry.rs", "src/inspect_registry.rs"]
    if protocol == "toy":
        return bindings
    return [*bindings, "oracle/src/bindings.rs", "oracle/src/types.rs"]


def checkout(protocol):
    """The directory of a protocol crate: beside this repository, or TOY in it."""
    if protocol == "toy":
        return APER / "protocols" / "toy"
    return APER.parent / f"oxirush-{protocol}"


def tested():
    """Whether the tests of generator/ pass, or "unviable" when they do not build."""
    manifest = ["--manifest-path", str(APER / "generator" / "Cargo.toml")]
    quiet = {"cwd": APER, "stdout": subprocess.DEVNULL, "stderr": subprocess.DEVNULL}
    if subprocess.run(["cargo", "test", "--no-run", *manifest], **quiet).returncode != 0:
        return "unviable"
    return ran([*LIMIT, "cargo", "test", *manifest], **quiet)


def generate(protocol):
    """The digest of the formatted bindings of `protocol`, None when the generator
    fails or writes what rustfmt cannot read, or "unviable" when it does not build. The
    crate is left as it was."""
    crate = checkout(protocol)
    env = dict(os.environ, CARGO=subprocess.check_output(["which", "cargo"], text=True).strip())
    manifest = ["--manifest-path", "build/Cargo.toml"]
    quiet = {"cwd": crate, "env": env, "stdout": subprocess.DEVNULL, "stderr": subprocess.DEVNULL}
    if subprocess.run(["cargo", "build", "--locked", *manifest], **quiet).returncode != 0:
        return "unviable"
    try:
        if not ran([*LIMIT, "cargo", "run", "--locked", *manifest], **quiet):
            return None
        files = written(protocol)
        rust = [file for file in files if file.endswith(".rs")]
        # Bindings that are no Rust are not those that the generator writes unchanged
        formatted = subprocess.run(["rustfmt", "--edition", "2024", *rust], **quiet)
        if formatted.returncode != 0:
            return None
        digest = hashlib.sha256()
        for file in files:
            digest.update((crate / file).read_bytes())
        return digest.hexdigest()
    finally:
        subprocess.run(["git", "checkout", "--quiet", "--", *written(protocol)], cwd=crate, check=True)


def judged(baseline):
    """What the mutant that is applied is: caught, missed or unviable."""
    own = tested()
    if own is not True:
        return "caught" if own is False else own
    for protocol in PROTOCOLS:
        digest = generate(protocol)
        if digest == "unviable":
            return "unviable"
        if digest != baseline[protocol]:
            return "caught"
    return "missed"


def main():
    mutants, shard, shards = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    if tested() is not True:
        sys.exit("the tests of the generator do not pass unchanged")
    baseline = {protocol: generate(protocol) for protocol in PROTOCOLS}
    if any(digest in (None, "unviable") for digest in baseline.values()):
        sys.exit(f"the generator does not write the bindings unchanged: {baseline}")
    counts = {"caught": 0, "missed": 0, "unviable": 0}
    missed = []
    with open(APER / "generator" / ".cargo" / "mutants.toml", "rb") as config:
        excluded = [re.compile(pattern) for pattern in tomllib.load(config).get("exclude_re", [])]
    listed = json.load(open(mutants))
    listed = [mutant for mutant in listed if not any(p.search(mutant["name"]) for p in excluded)]
    for mutant in listed[shard::shards]:
        diff = mutant["diff"].encode()
        file = APER / "generator" / mutant["file"]
        subprocess.run(["patch", "--quiet", str(file)], input=diff, check=True)
        try:
            outcome = judged(baseline)
        finally:
            subprocess.run(["patch", "--quiet", "--reverse", str(file)], input=diff, check=True)
        counts[outcome] += 1
        print(f"{outcome:8} {mutant['name']}", flush=True)
        if outcome == "missed":
            missed.append(mutant["name"])
    print(", ".join(f"{count} {outcome}" for outcome, count in counts.items()))
    with open("generator-missed.txt", "w") as out:
        out.writelines(name + "\n" for name in missed)
    sys.exit(1 if missed else 0)


main()

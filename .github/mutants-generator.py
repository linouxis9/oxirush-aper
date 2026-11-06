"""The mutants of generator/ against the bindings that it writes: a mutant is caught
when the bindings of oxirush-ngap or of oxirush-s1ap that it writes, formatted, or the
names of their fields in build/fields.tsv, differ from those that the generator writes
unchanged, or when it fails to write them; missed when they are the same, and unviable
when it does not build. A mutant that makes the generator run past 10 minutes, or take
more than 8 GiB of address space, fails to write them, and is caught: without the
limits it took the runner down. The protocol crates are
beside this repository, each with the ASN.1 of build/asn1.sha256 taken out of its
specification. The missed ones are written to `generator-missed.txt`, and the program
exits with 1 when there is one.

    python3 oxirush-aper/.github/mutants-generator.py MUTANTS.json SHARD SHARDS

MUTANTS.json is what `cargo mutants --list --json --diff` writes in generator/; the
shard SHARD of SHARDS takes every SHARDS-th mutant from the SHARD-th.
"""

import hashlib
import json
import os
import pathlib
import signal
import subprocess
import sys

APER = pathlib.Path(__file__).resolve().parent.parent
PROTOCOLS = ["ngap", "s1ap"]
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
    """What the generator writes for a protocol crate: the bindings, and the names that
    it gives the fields of the ASN.1."""
    return [
        f"src/{protocol}.rs",
        "src/registry.rs",
        "src/inspect_registry.rs",
        "oracle/src/bindings.rs",
        "oracle/src/types.rs",
        "build/fields.tsv",
    ]


def generate(protocol):
    """The digest of the formatted bindings of `protocol`, None when the generator
    fails, or "unviable" when it does not build. The crate is left as it was."""
    crate = APER.parent / f"oxirush-{protocol}"
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
        subprocess.run(["rustfmt", "--edition", "2024", *rust], cwd=crate, check=True)
        digest = hashlib.sha256()
        for file in files:
            digest.update((crate / file).read_bytes())
        return digest.hexdigest()
    finally:
        subprocess.run(["git", "checkout", "--quiet", "--", *written(protocol)], cwd=crate, check=True)


def main():
    mutants, shard, shards = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    baseline = {protocol: generate(protocol) for protocol in PROTOCOLS}
    if any(digest in (None, "unviable") for digest in baseline.values()):
        sys.exit(f"the generator does not write the bindings unchanged: {baseline}")
    counts = {"caught": 0, "missed": 0, "unviable": 0}
    missed = []
    for mutant in json.load(open(mutants))[shard::shards]:
        diff = mutant["diff"].encode()
        file = APER / "generator" / mutant["file"]
        subprocess.run(["patch", "--quiet", str(file)], input=diff, check=True)
        try:
            outcome = "missed"
            for protocol in PROTOCOLS:
                digest = generate(protocol)
                if digest == "unviable":
                    outcome = "unviable"
                    break
                if digest != baseline[protocol]:
                    outcome = "caught"
                    break
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

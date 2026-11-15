#!/usr/bin/env python3
# OxiRush
# Copyright 2025-2026 Valentin D'Emmanuele
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this
# file except in compliance with the License. You may obtain a copy of the License at
# http://www.apache.org/licenses/LICENSE-2.0. Unless required by applicable law or agreed
# to in writing, software distributed under the License is distributed on an "AS IS"
# BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under the
# License.

"""The comparison with rasn: the crate and the derive of rasn 0.29, each decoding
and encoding five values of NGAP with the same types (`src/bin/compare.rs`).

    python3 benches/compare.py [--core N] [--perf COMMAND] [--rounds N]

Run it from the root of the repository. It builds `compare` in benches/target, runs each
operation on one core (--core, 2 by default), with --perf for the cycles and the
instructions: `perf` by default, or `sudo -n perf` where perf_event_paranoid keeps them
from a user, in which case the program runs as the user again under perf. Each operation
of the two crates runs once a round, in turns, --rounds rounds (10 by default), so that a
busy moment of the machine falls on both: a figure is the median of the rounds, with the
lowest and the highest time, and the ratio of the times is given only where the two
spreads do not overlap. It prints the tables of benches/README.md.
"""

import argparse
import json
import os
import pathlib
import shutil
import statistics
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent


def once(program, codec, operation, value, iterations, core, perf):
    """One run of `compare`: its figures, and the cycles and instructions of perf."""
    argv = ["taskset", "-c", str(core), str(program), codec, operation, value,
            str(iterations)]
    if perf:
        if perf[0] == "sudo":
            argv = ["setpriv", f"--reuid={os.getuid()}", f"--regid={os.getgid()}",
                    "--clear-groups", *argv]
        argv = [*perf, "stat", "-x,", "-e", "cycles,instructions", "--", *argv]
    done = subprocess.run(argv, text=True, capture_output=True, check=True)
    counts = {}
    for line in done.stderr.splitlines():
        fields = line.split(",")
        if len(fields) > 2 and fields[2] in ("cycles", "instructions"):
            counts[fields[2]] = float(fields[0]) if fields[0][:1].isdigit() else None
    return json.loads(done.stdout.strip().splitlines()[-1]), counts


def iterations_of(program, codec, operation, value, core):
    """The calls of a run of about 0.2 s."""
    probe, _ = once(program, codec, operation, value, 1000, core, [])
    return max(1000, int(2e8 / max(probe["ns"], 1.0)))


def measure(program, codec, operation, value, iterations, core, perf):
    figures, _ = once(program, codec, operation, value, iterations, core, [])
    if perf:
        _, first = once(program, codec, operation, value, iterations, core, perf)
        _, second = once(program, codec, operation, value, 2 * iterations, core, perf)
        # The second run makes four times the iterations more calls.
        for name in ("cycles", "instructions"):
            if first.get(name) is not None and second.get(name) is not None:
                figures[name] = (second[name] - first[name]) / (4 * iterations)
    return figures


def summary(runs):
    """The median of each figure of the rounds, and the lowest and highest time."""
    figures = {}
    for name in runs[0]:
        values = [run[name] for run in runs if name in run]
        if len(set(values)) == 1:
            figures[name] = values[0]
        elif len(values) == len(runs) and all(isinstance(v, (int, float)) for v in values):
            figures[name] = statistics.median(values)
        else:
            figures[name] = runs[0][name]
    figures["low"], figures["high"] = min(r["ns"] for r in runs), max(r["ns"] for r in runs)
    return figures


def load():
    """The load average of the last minute."""
    return pathlib.Path("/proc/loadavg").read_text().split()[0]


def machine():
    """The model of the processor."""
    for line in pathlib.Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith(("model name", "Model")):
            return line.split(":", 1)[1].strip()
    return os.uname().machine


def main():
    arguments = argparse.ArgumentParser()
    arguments.add_argument("--core", type=int, default=2)
    arguments.add_argument("--perf", default="perf", help="the perf command, or ''")
    arguments.add_argument("--rounds", type=int, default=10)
    arguments = arguments.parse_args()
    perf = arguments.perf.split()
    if perf and shutil.which(perf[-1]) is None:
        perf = []
    target = os.environ.get("CARGO_TARGET_DIR", str(HERE / "target"))
    subprocess.run(["cargo", "build", "--release", "--manifest-path",
                    str(HERE / "Cargo.toml"), "--bin", "compare"], check=True,
                   env=dict(os.environ, CARGO_TARGET_DIR=target))
    program = pathlib.Path(target) / "release" / "compare"
    listed = subprocess.run([str(program), "list"], text=True, capture_output=True,
                            check=True).stdout
    values = [line.split("\t") for line in listed.splitlines()]
    before = load()
    calls = {(value, operation, codec): iterations_of(program, codec, operation, value, arguments.core)
             for value, _ in values for operation in ("decode", "encode") for codec in ("aper", "rasn")}
    runs = {key: [] for key in calls}
    for _ in range(arguments.rounds):
        for key, iterations in calls.items():
            value, operation, codec = key
            runs[key].append(measure(program, codec, operation, value, iterations, arguments.core, perf))
    results = {}
    for value, octets in values:
        results[value] = {"octets": len(octets) // 2}
        for operation in ("decode", "encode"):
            results[value][operation] = {codec: summary(runs[(value, operation, codec)])
                                         for codec in ("aper", "rasn")}
    print(json.dumps({"runs": {"/".join(key): runs[key] for key in runs}, "results": results}),
          file=sys.stderr)

    def cell(figures):
        text = f"{figures['ns']:.0f} ns ({figures['low']:.0f} to {figures['high']:.0f})"
        for name in ("cycles", "instructions"):
            if name in figures:
                text += f", {figures[name]:,.0f} {name}"
        count = figures["allocations"]
        text += f", {count} allocation{'' if count == 1 else 's'}"
        return text + (f" ({figures['bytes']:,} octets)" if count else "")

    for operation in ("decode", "encode"):
        print(f"{operation.capitalize()}, per call:\n")
        print("| Value (octets) | this crate | rasn 0.29 | rasn / this crate |")
        print("|---|---|---|---|")
        for value, figures in results.items():
            ours, theirs = figures[operation]["aper"], figures[operation]["rasn"]
            apart = theirs["low"] > ours["high"] or theirs["high"] < ours["low"]
            ratio = f"{theirs['ns'] / ours['ns']:.1f}x the time" if apart else "about the same time"
            if "instructions" in ours and "instructions" in theirs:
                ratio += f", {theirs['instructions'] / ours['instructions']:.1f}x the instructions"
            print(f"| {value} ({figures['octets']}) | {cell(ours)} | {cell(theirs)} | {ratio} |")
        print()
    print(f"On {machine()}, core {arguments.core}, {arguments.rounds} rounds, with a load average "
          f"of {before} before and {load()} after.")


main()

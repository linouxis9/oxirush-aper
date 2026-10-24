#!/bin/sh
# The crate as a member of a larger workspace, which is how a product that keeps a copy
# of it beside its own crates has it: cargo loads the workspace, clippy, which is
# `cargo check` with its lints, accepts every target of every member, and rustdoc
# documents them. With other crates, every target is built too: two members that give
# a target one name write one file, which cargo warns of where it builds them.
#
#     ../oxirush-aper/protocols/tests/workspace_member.sh DIRECTORY [CRATE...]
#
# Run it from the directory of the crate, oxirush-ngap or oxirush-s1ap. DIRECTORY, which
# the script makes, becomes a workspace whose members are this crate and each other
# CRATE, the directory of one: they are links, and nothing is written into them. A
# member named oxirush-aper or oxirush-utils is the one of the others. Without one it
# comes from crates.io.
set -eu
directory=$1
shift
here=$(pwd)
[ -f "$here/build/revisions" ] || { echo "run it from the directory of a protocol crate" >&2; exit 2; }
mkdir "$directory"
members=
patches=
for crate in "$here" "$@"; do
    crate=$(cd "$crate" && pwd)
    name=$(sed -n 's/^name = "\(.*\)"$/\1/p' "$crate/Cargo.toml" | head -n 1)
    ln -s "$crate" "$directory/$name"
    members="$members\"$name\", "
    if [ "$name" = oxirush-aper ] || [ "$name" = oxirush-utils ]; then
        patches="$patches
$name = { path = \"$name\" }"
    fi
done
cat > "$directory/Cargo.toml" <<MANIFEST
[workspace]
resolver = "3"
members = [$members]

[patch.crates-io]$patches
MANIFEST
cd "$directory"
cargo metadata --format-version 1 > /dev/null
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
if [ $# -gt 0 ]; then
    cargo build --workspace --all-targets 2> build.log || {
        cat build.log >&2
        exit 1
    }
    if grep -A 4 "output filename collision" build.log >&2; then
        exit 1
    fi
fi

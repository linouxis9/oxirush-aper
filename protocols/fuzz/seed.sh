#!/bin/sh
# Start the corpus of each target that reads octets from the vectors of the crate,
# from the PDUs that other implementations sent and from those that a fuzzer found a
# fault with: one file for each PDU of tests/fixtures and of the corpus of the comparison
# with Wireshark, tests/wireshark/corpus.tsv, that has no more than 4096 octets.
# The targets that build their input with `arbitrary` pick the PDUs of the fixtures
# themselves, and need none. Run it from the root of the repository of the crate,
# oxirush-ngap or oxirush-s1ap, or from protocols/toy:
#
#     ../oxirush-aper/protocols/fuzz/seed.sh
#     ../fuzz/seed.sh
set -eu
mkdir -p fuzz/corpus/decode fuzz/corpus/json fuzz/corpus/paths fuzz/corpus/exact
number=0
# The fixtures, and the corpus of the comparison with Wireshark where the crate has one
files=
for file in tests/fixtures/messages.tsv tests/fixtures/vectors.tsv tests/fixtures/later.tsv \
    tests/fixtures/captured.tsv tests/fixtures/fuzzed.tsv tests/wireshark/corpus.tsv; do
    if [ -f "$file" ]; then files="$files $file"; fi
done
# shellcheck disable=SC2086 # the names of the files, which have no space
grep -hv '^#' $files |
    while IFS="$(printf '\t')" read -r name octets rest; do
        [ "${#octets}" -le 8192 ] || continue
        number=$((number + 1))
        printf '%s' "$octets" | xxd -r -p > "fuzz/corpus/decode/$number"
        cp "fuzz/corpus/decode/$number" "fuzz/corpus/json/$number"
        cp "fuzz/corpus/decode/$number" "fuzz/corpus/exact/$number"
        # The same PDU under a procedure code that the specification does not have, 254,
        # whose message the crate reads as a container of IEs.
        { head -c 1 "fuzz/corpus/decode/$number"; printf '\376'; tail -c +3 "fuzz/corpus/decode/$number"; } \
            > "fuzz/corpus/json/$number-254"
        cp "fuzz/corpus/json/$number-254" "fuzz/corpus/decode/$number-254"
        cp "fuzz/corpus/json/$number-254" "fuzz/corpus/exact/$number-254"
        # Before its PDU, the target of the paths takes what is done with it: here
        # `select`, with the first path of the tree, whole, and nothing more.
        { printf '\000\000\377\000\000'; cat "fuzz/corpus/decode/$number"; } \
            > "fuzz/corpus/paths/$number"
    done


#!/bin/sh
# Start the corpus of each target that reads octets from the vectors of the crate,
# from the PDUs that other implementations sent and from those that a fuzzer found a
# fault with: one file for each of tests/fixtures that has no more than 4096 octets.
# The targets that build their input with `arbitrary` pick the PDUs of the fixtures
# themselves, and need none. Run it from the root of the repository of the crate,
# oxirush-ngap or oxirush-s1ap:
#
#     ../oxirush-aper/protocols/fuzz/seed.sh
set -eu
mkdir -p fuzz/corpus/decode fuzz/corpus/json fuzz/corpus/paths fuzz/corpus/exact
number=0
grep -hv '^#' tests/fixtures/messages.tsv tests/fixtures/vectors.tsv tests/fixtures/later.tsv \
    tests/fixtures/captured.tsv tests/fixtures/fuzzed.tsv |
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

# The dictionary of the targets that read octets, from the procedures of
# src/registry.rs: the header of each message, its direction, its procedure code and
# its criticality, and each IE of each message, its identifier and its criticality, as
# aligned PER writes them, each once.
awk '
    function criticality(name) { return name == "reject" ? "00" : name == "ignore" ? "40" : "80" }
    /^    [0-9]+ [A-Za-z]/ {
        code = sprintf("%02x", $1)
        procedure = criticality($3)
        line = $0
        while (match(line, /(InitiatingMessage|SuccessfulOutcome|UnsuccessfulOutcome) [^ ]+ "[^"]+"/)) {
            part = substr(line, RSTART, RLENGTH)
            split(part, words, " ")
            choice = words[1] == "InitiatingMessage" ? "00" : words[1] == "SuccessfulOutcome" ? "20" : "40"
            name = words[3]
            gsub(/"/, "", name)
            printf "%s=\"\\x%s\\x%s\\x%s\"\n", name, choice, code, procedure
            line = substr(line, RSTART + RLENGTH)
            if (match(line, /\[[^]]*\]/)) {
                n = split(substr(line, RSTART + 1, RLENGTH - 2), ies, ", ")
                for (i = 1; i <= n; i++) {
                    split(ies[i], ie, " ")
                    printf "%s_%s=\"\\x%02x\\x%02x\\x%s\"\n", name, ie[1], int(ie[1] / 256), ie[1] % 256, criticality(ie[2])
                }
            }
        }
    }
' src/registry.rs | awk -F = '!seen[$2]++' > fuzz/corpus/protocol.dict

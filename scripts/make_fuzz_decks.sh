#!/usr/bin/env bash
# Regenerate decks/fuzz/*-doubles.txt from decks/coverage/*-coverage.txt:
# every nonland card twice, basic lands doubled to match.
#
# The coverage decks are singleton, so in the nightly fuzz no board ever
# holds two same-named permanents, two same-named graveyard cards or two
# copies of one spell in hand — and the byte-identical-row defects (#612,
# #631, #668) were invisible to it. scripts/fuzz.sh runs these decks as a
# second, smaller campaign after the coverage one.
set -eu
cd "$(dirname "$0")/.."
mkdir -p decks/fuzz
for f in decks/coverage/*-coverage.txt; do
  b=$(basename "$f" -coverage.txt)
  out="decks/fuzz/$b-doubles.txt"
  {
    echo "# Fuzz deck $b-doubles: decks/coverage/$b-coverage.txt with every nonland"
    echo "# card twice. The coverage decks are singleton, so a board with two"
    echo "# same-named permanents, two same-named graveyard cards or two copies of"
    echo "# one spell in hand never arises in the nightly fuzz — the class of"
    echo "# byte-identical-row defects (#612, #631) was invisible to it. Generated"
    echo "# by scripts/make_fuzz_decks.sh; regenerate rather than edit."
    echo
    awk '!/^#/ && NF {
      n = $1; name = $0; sub(/^[0-9]+ /, "", name)
      if (name ~ /^(Plains|Island|Swamp|Mountain|Forest)$/) print n * 2, name
      else print 2, name
    }' "$f"
  } > "$out"
done
echo "wrote $(ls decks/fuzz/*-doubles.txt | wc -l) doubles decks"

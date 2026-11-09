#!/usr/bin/env bash
# Run the engine's proofs: Verus over mtg-kernel, then Kani over
# mtg-engine/src/proofs/.
#
# Usage: scripts/prove.sh [verus | kani [HARNESS...] | kani-pr | kani-shard K/N | exhaustive]
#   With no arguments, Verus and Kani. `verus` checks every contract in
#   mtg-kernel (seconds, once vstd is built). `kani` runs every Kani proof,
#   or only the named ones (a name may be the last part of the path, e.g.
#   counting_a_cost_counts_every_pip): every one is some 85 hours of solver
#   time on four cores, a day and a half here. `kani-pr` is the pull
#   request's tier, under half an hour: scripts/kani/pr.list and the
#   proofs of each proof module the change edits (scripts/kani-tiers.py
#   pr). `kani-shard K/N` is the Kth of the N jobs the scheduled run of
#   every harness is (.github/workflows/proofs-full.yml). Both fail on a
#   harness that does not prove unless scripts/kani/expected-open.txt names
#   it. `exhaustive` runs the bounded
#   exhaustive checks at a larger scope than `cargo test` gives them, in
#   release: the autotap planner against tapping by hand on every board of
#   up to three sources with up to two mana floating (about 30 million
#   boards, a few minutes), every Equipment and Aura the card grammar
#   reads on every creature it reads, against every other (about 100,000
#   combats), every creature it fights with blocked by every pair of kinds
#   of them (about 100,000 double blocks, six minutes), and every two kinds
#   of them attacking together (about 8,000 combats).
#
# A proof is a theorem over every input it quantifies over, not a sample:
# GREEN means no input breaks the property. A Verus proof holds for every
# value; a Kani proof for every value within the bounds its module doc
# states. When a Kani proof fails, `cargo kani -p mtg-engine --harness NAME
# -Z concrete-playback --concrete-playback=print` gives the failing input as
# a unit test. When a Verus proof fails, add `-- --expand-errors` to the
# `cargo verus` line below to see which part of which clause.
#
# The Kani proofs run one at a time on purpose: each takes up to a few GB,
# and four at once ran a 15 GB machine out of memory.
# docs/plans/formal-verification.md lists each proof, its bounds and how
# long it takes.
set -euo pipefail

cd "$(dirname "$0")/.."

# The releases CI uses. vstd in mtg-kernel/Cargo.toml must match Verus.
VERUS_RELEASE=0.2026.10.04.426d8b0
VERUS_TOOLCHAIN=1.98.1-x86_64-unknown-linux-gnu
KANI_VERSION=0.68.0

what="${1:-all}"
case "$what" in
    verus | kani | kani-pr | kani-shard | exhaustive) shift ;;
    all) ;;
    *) echo "usage: scripts/prove.sh [verus | kani [HARNESS...] | kani-pr | kani-shard K/N | exhaustive]" >&2; exit 2 ;;
esac

prove_verus() {
    if ! cargo verus --help >/dev/null 2>&1; then
        cat >&2 <<EOF
Verus is not installed. Install the release CI uses:

    curl -LO https://github.com/verus-lang/verus/releases/download/release/$VERUS_RELEASE/verus-$VERUS_RELEASE-x86-linux.zip
    unzip verus-$VERUS_RELEASE-x86-linux.zip
    export PATH="\$PWD/verus-x86-linux:\$PATH"
    rustup install $VERUS_TOOLCHAIN
EOF
        exit 2
    fi
    cargo verus verify -p mtg-kernel -- --triggers-mode silent
}

prove_kani() {
    if ! cargo kani --version >/dev/null 2>&1; then
        cat >&2 <<EOF
Kani is not installed. Install the version CI uses:

    cargo install --locked kani-verifier --version $KANI_VERSION
    cargo kani setup
EOF
        exit 2
    fi
    # Through scripts/kani-bounded.py, which runs each harness as `cargo
    # kani` does (-Z stubbing for the card proofs' stand-ins, a
    # field-sensitivity bound so CBMC follows a value through a heap
    # allocation, memcmp's own unwinding bound for card names) and adds a
    # recursion bound on the functions over the engine's self-referring
    # types, without which dropping one filter read from the heap explored
    # every variant it might be. Results, one line a harness, go to
    # logs/kani/results.txt; a harness already there is skipped.
    python3 scripts/kani-bounded.py "$@"
}

check_exhaustively() {
    AUTOTAP_EXHAUSTIVE_SOURCES=3 AUTOTAP_EXHAUSTIVE_POOL=2 \
        cargo test --release -p mtg-engine --test autotap_brute_force every_small_board
    CARD_SEMANTICS_EVERY_FIGHT=1 \
        cargo test --release -p mtg-engine --test card_semantics reaches_the_combats
    CARD_SEMANTICS_EVERY_FIGHT=1 \
        cargo test --release -p mtg-engine --test card_semantics every_double_block
    CARD_SEMANTICS_EVERY_FIGHT=1 \
        cargo test --release -p mtg-engine --test card_semantics every_two_attackers
}

case "$what" in
    verus) prove_verus ;;
    kani) prove_kani "$@" ;;
    kani-pr)
        list=$(mktemp)
        python3 scripts/kani-tiers.py pr --base "${KANI_PR_BASE:-origin/master}" --extra 300 > "$list"
        echo "$(wc -l < "$list") harnesses: scripts/kani/pr.list and the change's proof modules"
        prove_kani --list "$list" --expect-open scripts/kani/expected-open.txt --loops scripts/kani/loop-bounds.txt \
            --batch 24 --timeout 600 --solo-cap 12 --results logs/kani/pr.txt ;;
    kani-shard)
        prove_kani --shard "$1" --times scripts/kani/times.txt --expect-open scripts/kani/expected-open.txt \
            --loops scripts/kani/loop-bounds.txt --batch 8 --timeout 1800 --solo-cap 12 \
            --results "logs/kani/shard-${1%/*}.txt" ;;
    exhaustive) check_exhaustively ;;
    all) prove_verus; prove_kani ;;
esac

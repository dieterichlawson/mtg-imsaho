#!/usr/bin/env bash
# Run the engine's proofs: Verus over mtg-kernel, then Kani over
# mtg-engine/src/proofs/.
#
# Usage: scripts/prove.sh [verus | kani [HARNESS...] | kani-pr | kani-shard K/N | kani-new K/N |
#                          kani-shares RUN N | exhaustive]
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
#   it. `kani-new K/N` is the Kth of N shares of the harnesses written and
#   not yet run to a result (NOT_RUN in scripts/kani/times.txt), which the
#   other two leave out; it fails only if one of them is left with no
#   result. `kani-shares RUN N` is all N shares of RUN (kani-shard or
#   kani-new) at once, for one machine with the cores and memory: KANI_CAP
#   (12) is the gigabytes a solver may take and KANI_TIMEOUT its seconds.
#   `exhaustive` runs the bounded
#   exhaustive checks at a larger scope than `cargo test` gives them, in
#   release: the autotap planner against tapping by hand on every board of
#   up to three sources with up to two mana floating (about 30 million
#   boards, a few minutes), every Equipment and Aura the card grammar
#   reads on every creature it reads, against every other (about 100,000
#   combats), every creature it fights with blocked by every pair of kinds
#   of them (about 100,000 double blocks, six minutes), every two kinds
#   of them attacking together (about 8,000 combats), and every card the
#   grammar reads cast, every activated and loyalty ability it reads
#   activated, every flashback it reads cast and every upkeep and end
#   step trigger it reads played, with the real engine, on every board of
#   one piece (any permanent of the reference's vocabulary or creature
#   token on either side in each state of wear, or any card of the pool
#   in either graveyard: about 2,000 boards, 570,000 casts, 130,000
#   activations, 60,000 flashback casts and 70,000 turns begun or ended,
#   two minutes), and a combat declared every way on every board of two
#   pieces in every state of wear (2 million boards, 1.4 million combats,
#   a few minutes).
#   CARD_SEMANTICS_SMALL=2 is every board of two pieces for the rest too
#   (about 330,000 boards as they entered and 90 million casts, hours on
#   four cores), run by hand in shares (CARD_SEMANTICS_SHARD=K/N).
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
    verus | kani | kani-pr | kani-shard | kani-new | kani-shares | exhaustive) shift ;;
    all) ;;
    *) echo "usage: scripts/prove.sh [verus | kani [HARNESS...] | kani-pr | kani-shard K/N | kani-new K/N | kani-shares RUN N | exhaustive]" >&2; exit 2 ;;
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

# The list of loop bounds a run reads and adds to: the committed one, or
# with KANI_LOOPS_DIR a copy of the committed lists there (a share of
# `kani-shares`).
loops_file() {
    if [ -n "${KANI_LOOPS_DIR:-}" ]; then
        mkdir -p "$KANI_LOOPS_DIR"
        [ -e "$KANI_LOOPS_DIR/loop-bounds.txt" ] || cp scripts/kani/loop-bounds*.txt "$KANI_LOOPS_DIR"/
        echo "$KANI_LOOPS_DIR/loop-bounds.txt"
    else
        echo scripts/kani/loop-bounds.txt
    fi
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
    CARD_SEMANTICS_SMALL=1 \
        cargo test --release -p mtg-engine --test card_semantics every_small_board
    CARD_SEMANTICS_SMALL=2 CARD_SEMANTICS_SMALL_WORN=1 \
        cargo test --release -p mtg-engine --test card_semantics every_combat_ends_as_the_rules_say_on_every_small_board
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
        prove_kani --shard "$1" --times scripts/kani/times.txt --skip-not-run --expect-open scripts/kani/expected-open.txt \
            --loops "$(loops_file)" --batch 8 --timeout "${KANI_TIMEOUT:-1800}" --solo-cap "${KANI_CAP:-12}" \
            --results "logs/kani/shard-${1%/*}.txt" ;;
    kani-new)
        # Two to a build: Kani's compiler is stopped for memory building
        # four proofs of a whole game loop at once on 16 GB.
        prove_kani --shard "$1" --times scripts/kani/times.txt --only-not-run \
            --loops "$(loops_file)" --batch 2 --timeout "${KANI_TIMEOUT:-1500}" --solo-cap "${KANI_CAP:-12}" \
            --results "logs/kani/shard-${1%/*}.txt" ;;
    kani-shares)
        # Every one of N shares at once, on one machine with the cores and
        # memory for it: at most one build or one solver a share, so about
        # KANI_CAP gigabytes each. Each share builds in its own target
        # directory and learns loop bounds in its own copy of the lists, so
        # that none waits for another or writes over it.
        run=$1 n=$2
        case "$run" in kani-shard | kani-new) ;; *) echo "kani-shares: kani-shard or kani-new, not $run" >&2; exit 2 ;; esac
        mkdir -p logs/kani
        pids=()
        for k in $(seq 1 "$n"); do
            CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}/kani-share-$k" KANI_LOOPS_DIR="logs/kani/share-$k" \
                KANI_BOUNDED_CODEGEN_SLOTS="$n" bash scripts/prove.sh "$run" "$k/$n" > "logs/kani/share-$k.log" 2>&1 &
            pids+=($!)
        done
        status=0
        for pid in "${pids[@]}"; do wait "$pid" || status=1; done
        tail -n 3 logs/kani/share-*.log
        exit $status ;;
    exhaustive) check_exhaustively ;;
    all) prove_verus; prove_kani ;;
esac

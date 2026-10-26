#!/usr/bin/env bash
# Coverage-guided fuzzing of the whole engine: fuzz/fuzz_targets/game.rs,
# where every choice in a game comes from the fuzzer's input and every
# invariant the seeded campaign (scripts/fuzz.sh) checks is checked at every
# decision. libFuzzer keeps the inputs that reach new code and mutates
# those, so it goes where the random seats rarely do.
#
# Usage: scripts/fuzz-engine.sh [SECONDS]   fuzz for SECONDS (default 600)
#        scripts/fuzz-engine.sh repro FILE  replay one input, e.g. a crash
#
# A finding is a violated invariant, a stalled game, or a panic (overflow
# included), written to fuzz/artifacts/game/crash-* with the input that
# caused it; `repro` on that file shows it again. The corpus it learns
# lives in fuzz/corpus/game (gitignored); keep it between runs to keep the
# coverage. docs/plans/formal-verification.md (tier 2) says what the target
# is for.
set -euo pipefail

cd "$(dirname "$0")/.."

# cargo-fuzz needs a nightly; this is the one Kani installs (scripts/prove.sh).
TOOLCHAIN=nightly-2026-08-21
CARGO_FUZZ_VERSION=0.13.2

if ! cargo fuzz --version >/dev/null 2>&1 || ! rustup run "$TOOLCHAIN" rustc --version >/dev/null 2>&1; then
    cat >&2 <<HELP
cargo-fuzz or its nightly toolchain is not installed. Install them with:

    rustup toolchain install $TOOLCHAIN --profile minimal
    cargo install --locked cargo-fuzz --version $CARGO_FUZZ_VERSION
HELP
    exit 2
fi

fuzz=(cargo "+$TOOLCHAIN" fuzz run game --sanitizer none --debug-assertions)

if [ "${1:-}" = "repro" ]; then
    exec "${fuzz[@]}" "${2:?usage: scripts/fuzz-engine.sh repro FILE}"
fi

seconds="${1:-600}"
corpus=fuzz/corpus/game
mkdir -p "$corpus"
# A first run starts from random inputs: any byte string is a game.
if [ -z "$(ls -A "$corpus")" ]; then
    for i in $(seq 1 16); do
        head -c $((256 * i)) /dev/urandom > "$corpus/seed-$i"
    done
fi
exec "${fuzz[@]}" "$corpus" -- -max_len=8192 -max_total_time="$seconds" -print_final_stats=1

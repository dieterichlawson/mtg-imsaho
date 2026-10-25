#!/usr/bin/env bash
# Run the engine's proofs: Verus over mtg-kernel, then Kani over
# mtg-engine/src/proofs/.
#
# Usage: scripts/prove.sh [verus | kani [HARNESS...]]
#   With no arguments, both. `verus` checks every contract in mtg-kernel
#   (seconds, once vstd is built). `kani` runs every Kani proof, or only the
#   named ones (a name may be the last part of the path, e.g.
#   counting_a_cost_counts_every_pip).
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
    verus | kani) shift ;;
    all) ;;
    *) echo "usage: scripts/prove.sh [verus | kani [HARNESS...]]" >&2; exit 2 ;;
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
    local args=()
    for h in "$@"; do
        args+=(--harness "$h")
    done
    cargo kani -p mtg-engine --output-format terse "${args[@]}"
}

case "$what" in
    verus) prove_verus ;;
    kani) prove_kani "$@" ;;
    all) prove_verus; prove_kani ;;
esac

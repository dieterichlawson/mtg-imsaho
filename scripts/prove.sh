#!/usr/bin/env bash
# Run the engine's Kani proofs (mtg-engine/src/proofs/).
#
# Usage: scripts/prove.sh [HARNESS...]
#   With no arguments, every proof runs, one after another. With names, only
#   those (a name may be the last part of the path, e.g.
#   reserving_leaves_the_reserve_payable).
#
# A proof is a theorem over every input within its stated bounds, not a
# sample, so GREEN means no input in that space breaks the property and RED
# comes back with the input that does (run one harness with
# `cargo kani -p mtg-engine --harness NAME -Z concrete-playback
# --concrete-playback=print` to get it as a unit test).
#
# The proofs run one at a time on purpose: each takes up to a few GB, and
# four at once ran a 15 GB machine out of memory. docs/plans/
# formal-verification.md lists each proof, its bounds and how long it takes.
set -euo pipefail

cd "$(dirname "$0")/.."

if ! cargo kani --version >/dev/null 2>&1; then
    cat >&2 <<'EOF'
Kani is not installed. Install the version CI uses:

    cargo install --locked kani-verifier --version 0.68.0
    cargo kani setup
EOF
    exit 2
fi

args=()
for h in "$@"; do
    args+=(--harness "$h")
done

exec cargo kani -p mtg-engine --output-format terse "${args[@]}"

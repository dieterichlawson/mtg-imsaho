#!/usr/bin/env bash
# Audit-calibration harness: reintroduce real, previously fixed bugs into
# throwaway worktrees so blind audits can be scored against ground truth.
#
# Usage:
#   scripts/calibration/seed_bugs.sh candidates
#       List fix commits that touched exactly one card file — the cleanest
#       calibration specimens (the audit unit is a card).
#   scripts/calibration/seed_bugs.sh seed <commit> [<commit> ...]
#       For each commit: create a worktree at HEAD with that commit's
#       mtg-engine/src changes reverted (the bug is back; tests and audit
#       logs stay at HEAD). Verifies the worktree still compiles. Appends
#       a line per specimen to the manifest.
#   scripts/calibration/seed_bugs.sh clean
#       Remove all calibration worktrees and the manifest.
#
# The manifest (audits/calibration/manifest.tsv) maps specimen -> commit and
# is the answer key: auditors must not read it, and audits of a specimen run
# only inside its worktree. See docs/plans/audit-calibration.md for the
# protocol this feeds.
set -euo pipefail
cd "$(dirname "$0")/../.."

WORKTREE_BASE="${CALIB_WORKTREE_BASE:-/tmp/mtg-calibration}"
MANIFEST="audits/calibration/manifest.tsv"

case "${1:-}" in
candidates)
    # Fix commits whose non-test changes are confined to one card file.
    git log --no-merges --format='%H %s' -- mtg-engine/src/cards | \
    while read -r sha subject; do
        files=$(git show --name-only --format= "$sha" -- 'mtg-engine/src/**' | sort -u)
        count=$(printf '%s\n' "$files" | grep -c . || true)
        card_files=$(printf '%s\n' "$files" | grep -c 'src/cards/isd/' || true)
        if [ "$count" = 1 ] && [ "$card_files" = 1 ]; then
            printf '%s\t%s\t%s\n' "$sha" "$(printf '%s\n' "$files" | head -1)" "$subject"
        fi
    done
    ;;

seed)
    shift
    [ $# -ge 1 ] || { echo "seed needs at least one commit" >&2; exit 1; }
    mkdir -p "$WORKTREE_BASE" "$(dirname "$MANIFEST")"
    n=$( [ -f "$MANIFEST" ] && wc -l < "$MANIFEST" || echo 0 )
    for sha in "$@"; do
        n=$((n + 1))
        id=$(printf 'specimen-%02d' "$n")
        dir="$WORKTREE_BASE/$id"
        git worktree add --detach "$dir" HEAD > /dev/null
        if ! git show "$sha" -- 'mtg-engine/src/**' | git -C "$dir" apply -R --reject 2> "$dir/.revert-errors"; then
            echo "SKIP $sha: revert does not apply cleanly at HEAD (see $dir/.revert-errors)" >&2
            git worktree remove --force "$dir"
            n=$((n - 1))
            continue
        fi
        if ! (cd "$dir" && cargo check -p mtg-engine --quiet 2> "$dir/.check-errors"); then
            echo "SKIP $sha: worktree does not compile with the bug re-seeded (see $dir/.check-errors)" >&2
            git worktree remove --force "$dir"
            n=$((n - 1))
            continue
        fi
        files=$(git show --name-only --format= "$sha" -- 'mtg-engine/src/**' | sort -u | paste -sd, -)
        printf '%s\t%s\t%s\t%s\n' "$id" "$sha" "$dir" "$files" >> "$MANIFEST"
        echo "$id: $dir (bug from $sha re-seeded)"
    done
    echo "manifest: $MANIFEST"
    ;;

clean)
    if [ -f "$MANIFEST" ]; then
        cut -f3 "$MANIFEST" | while read -r dir; do
            git worktree remove --force "$dir" 2>/dev/null || true
        done
        rm -f "$MANIFEST"
    fi
    rm -rf "$WORKTREE_BASE"
    git worktree prune
    echo "calibration worktrees and manifest removed"
    ;;

*)
    echo "usage: $0 candidates | seed <commit>... | clean" >&2
    exit 1
    ;;
esac

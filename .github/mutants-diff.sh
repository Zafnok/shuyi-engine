#!/usr/bin/env bash
# The Rust diff the PR mutation gate tests (.github/workflows/mutants.yml):
# from the merge base with the base branch, or, while the base branch
# doesn't hold the carved history yet, from its tip `CARVED` (the code as
# it left the game's repository, where every line passed this gate).
set -euo pipefail
base="origin/$BASE_REF"
if git merge-base --is-ancestor "$CARVED" HEAD &&
    ! git merge-base --is-ancestor "$CARVED" "$base"; then
    echo "diff from the carved history's tip $CARVED" >&2
    git diff "$CARVED" HEAD -- '*.rs'
else
    git diff "$base...HEAD" -- '*.rs'
fi

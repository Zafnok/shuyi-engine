#!/usr/bin/env bash
# What one attempt of a CI run failed on, for `cargo xtask flaky` (ticket 2064):
# writes <out>/jobs.json, every job of the attempt with its conclusion, and for
# each failed job its log and, for a job tools/flaky.ron's `matrices` name,
# its matrix.json.
#
#   bash scripts/flaky-failures.sh <run id> <attempt> <out dir>
#
# Needs `gh` with `actions: read` on GITHUB_REPOSITORY. The logs and the matrix
# are data from the run: nothing here runs them.
set -euo pipefail

run=$1
attempt=$2
out=$3
repo=${GITHUB_REPOSITORY:?}
mkdir -p "$out"
# Each matrix job and its artifact, a tab between.
matrices=$(cargo xtask flaky matrices)

gh api "repos/$repo/actions/runs/$run/attempts/$attempt/jobs?per_page=100" --paginate \
  --jq '.jobs[] | {name, id, conclusion, completed_at}' | jq -s . > "$out/all.json"

records=()
while IFS= read -r job; do
  id=$(jq -r .id <<< "$job")
  name=$(jq -r .name <<< "$job")
  conclusion=$(jq -r '.conclusion // ""' <<< "$job")
  record=$(jq -c '{name, id, conclusion}' <<< "$job")
  if [ "$conclusion" = failure ]; then
    if gh api "repos/$repo/actions/jobs/$id/logs" > "$out/$id.log" 2> /dev/null; then
      record=$(jq -c --arg log "$out/$id.log" '. + {log: $log}' <<< "$record")
    fi
    # A matrix job's matrix: the newest of its artifacts when it finished (a
    # re-run uploads another of the same name).
    artifact_name=$(awk -F '\t' -v job="$name" '$1 == job { print $2 }' <<< "$matrices")
    if [ -n "$artifact_name" ]; then
      completed=$(jq -r .completed_at <<< "$job")
      artifact=$(gh api "repos/$repo/actions/runs/$run/artifacts?name=$artifact_name&per_page=100" \
        --jq "[.artifacts[] | select(.created_at <= \"$completed\")] | sort_by(.created_at) | last | .id // empty")
      if [ -n "$artifact" ] && gh api "repos/$repo/actions/artifacts/$artifact/zip" > "$out/$id.zip" 2> /dev/null \
        && unzip -p "$out/$id.zip" matrix.json > "$out/$id.matrix.json" 2> /dev/null; then
        record=$(jq -c --arg matrix "$out/$id.matrix.json" '. + {matrix: $matrix}' <<< "$record")
      fi
    fi
  fi
  records+=("$record")
done < <(jq -c '.[]' "$out/all.json")

printf '%s\n' "${records[@]}" | jq -s . > "$out/jobs.json"
jq -r '.[] | select(.conclusion == "failure") | "failed: \(.name)"' "$out/jobs.json"

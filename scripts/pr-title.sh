#!/usr/bin/env bash
# Prints the pull request's title as it is now (ticket 2051, ADR-0092). A
# re-run of a job keeps the event that started the run, and with it the
# title from before a retitle; GitHub's answer has the new one. Falls back
# to the event's title when GitHub can't be asked.
#
# Reads PR_NUMBER and PR_TITLE (the event's title), and GITHUB_REPOSITORY.
set -uo pipefail
if title=$(gh api "repos/$GITHUB_REPOSITORY/pulls/$PR_NUMBER" --jq .title 2>/dev/null) &&
  [ -n "$title" ]; then
  printf '%s\n' "$title"
else
  printf '%s\n' "$PR_TITLE"
fi

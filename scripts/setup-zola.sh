#!/usr/bin/env bash
# Installs the website's Zola in /usr/local/bin (ADR-0048): 0.21.0, the last
# release under MIT (later ones are EUPL-1.2, which ADR-0002 §8 denies),
# downloaded from its GitHub release and checked against the SHA-256 below.
# The one place the version is written (ticket 2840): CI's action
# (`.github/actions/setup-zola/action.yml`, which ci.yml and pages.yml use)
# runs it, and so does the book's dev container
# (`.devcontainer/book/devcontainer.json`), which builds the site to serve it.
# Linux on x86_64 only: a tool of the build, never shipped. A Zola already
# there at this version is kept.
set -euo pipefail

version=v0.21.0
# zola-v0.21.0-x86_64-unknown-linux-gnu.tar.gz, as the release lists it.
sha256=5c37a8f706567d6cad3f0dbc0eaebe3b9591cc301bd67089e5ddc0d0401732d6

if [ "$(zola --version 2> /dev/null)" = "zola ${version#v}" ]; then
  echo "setup-zola.sh: zola ${version#v} is there"
  exit 0
fi

archive="zola-${version}-x86_64-unknown-linux-gnu.tar.gz"
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
curl --proto '=https' --tlsv1.2 -fsSL -o "$dir/$archive" \
  "https://github.com/getzola/zola/releases/download/${version}/${archive}"
echo "${sha256}  $dir/$archive" | sha256sum --check --strict
tar -xzf "$dir/$archive" -C "$dir"
sudo=""
[ "$(id -u)" -eq 0 ] || sudo=sudo
$sudo install -m 0755 "$dir/zola" /usr/local/bin/zola
zola --version

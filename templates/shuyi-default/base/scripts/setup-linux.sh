#!/usr/bin/env bash
# Sets up a Linux machine, a dev container or a cloud agent's container to
# build this project: the C linker, pkg-config and the system libraries the
# engine's backend links and opens, then rustup and the Rust that
# rust-toolchain.toml pins, with the web target. `shuyi doctor` checks the
# same list. Safe to run again: what is there is left alone.
#
# Run it as `bash scripts/setup-linux.sh`, as root or as a user with sudo.

set -euo pipefail
cd "$(dirname "$0")/.."

# The packages, in doctor's order: the C linker, pkg-config, ALSA and
# libudev to build; X11, XInput, xkbcommon and GL to open a window.
APT="build-essential pkg-config libasound2-dev libudev-dev libx11-dev libxi-dev libxkbcommon-dev libgl1-mesa-dev"
DNF="gcc pkgconf-pkg-config alsa-lib-devel systemd-devel libX11-devel libXi-devel libxkbcommon-devel mesa-libGL-devel"
PACMAN="base-devel pkgconf alsa-lib systemd-libs libx11 libxi libxkbcommon libglvnd"

did=()

# Runs the package manager as root: directly, or through sudo.
as_root() {
  if [ "$(id -u)" = 0 ]; then
    "$@"
  elif command -v sudo > /dev/null; then
    sudo "$@"
  else
    echo "setup-linux: installing packages needs root: run this script as root, or install sudo" >&2
    exit 1
  fi
}

if command -v apt-get > /dev/null; then
  manager=apt
  has() { dpkg-query -W -f '${Status}' "$1" 2> /dev/null | grep -q 'install ok installed'; }
  add() {
    as_root apt-get update
    as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "$@"
  }
  packages=$APT
elif command -v dnf > /dev/null; then
  manager=dnf
  has() { rpm -q "$1" > /dev/null 2>&1; }
  add() { as_root dnf install -y "$@"; }
  packages=$DNF
elif command -v pacman > /dev/null; then
  manager=pacman
  has() { pacman -Q "$1" > /dev/null 2>&1; }
  # With the databases synced and the system upgraded with them, as Arch
  # installs (`-Sy` alone is a partial upgrade); a bare image has none.
  add() { as_root pacman -Syu --needed --noconfirm "$@"; }
  packages=$PACMAN
else
  echo "setup-linux: found none of apt-get, dnf and pacman. Install these with your package manager:" >&2
  echo "  apt:    $APT" >&2
  echo "  dnf:    $DNF" >&2
  echo "  pacman: $PACMAN" >&2
  exit 1
fi

# Installs those of its arguments that aren't installed.
install_missing() {
  local missing=()
  for package in "$@"; do
    has "$package" || missing+=("$package")
  done
  if [ ${#missing[@]} -gt 0 ]; then
    add "${missing[@]}"
    did+=("installed with $manager: ${missing[*]}")
  fi
}

# shellcheck disable=SC2086 # one word a package
install_missing $packages

export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup > /dev/null; then
  # Fedora's curl-minimal gives `curl` under another package's name.
  needed=(ca-certificates)
  command -v curl > /dev/null || needed+=(curl)
  install_missing "${needed[@]}"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y --profile minimal --default-toolchain none
  did+=("installed rustup in $HOME/.cargo")
fi

# The Rust rust-toolchain.toml pins, with its components: rustup 1.28 and
# later install it with `rustup toolchain install`, older ones on
# `rustup show`.
before=$(rustup toolchain list)
version=$(rustup --version 2> /dev/null)
version=${version#rustup }
version=${version%% *}
if [ "$(printf '%s\n' 1.28.0 "$version" | sort -V | head -n 1)" = 1.28.0 ]; then
  rustup toolchain install --no-self-update
else
  rustup show
fi
[ "$before" = "$(rustup toolchain list)" ] || did+=("installed the Rust rust-toolchain.toml pins")
if ! rustup target list --installed | grep -qx wasm32-unknown-unknown; then
  rustup target add wasm32-unknown-unknown
  did+=("added the web target, wasm32-unknown-unknown")
fi

if [ -n "${GITHUB_PATH:-}" ]; then
  echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"
fi

echo
if [ ${#did[@]} -eq 0 ]; then
  echo "setup-linux: everything was there already."
else
  echo "setup-linux: done."
  printf '  %s\n' "${did[@]}"
fi
echo "Next: open a new shell or run \`. \"\$HOME/.cargo/env\"\`, then \`cargo shuyi doctor\`."

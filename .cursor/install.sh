#!/usr/bin/env bash
# Install the Rust toolchain pinned in rust-toolchain.toml (stable, rustfmt, clippy).
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain none --profile minimal
fi

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  . "${HOME}/.cargo/env"
fi

# rustup 1.28 and later install a toolchain file only via this command.
rustup toolchain install
rustup component add rustfmt clippy

# Install-script PATH exports do not persist. Link ~/.cargo proxies onto the
# default PATH when rustup was installed there rather than system-wide.
cargo_bin="${CARGO_HOME:-${HOME}/.cargo}/bin"
if [[ -d "${cargo_bin}" ]]; then
  dest="/usr/local/bin"
  if [[ ! -w "${dest}" ]] && command -v sudo >/dev/null 2>&1; then
    sudo mkdir -p "${dest}"
  else
    mkdir -p "${dest}"
  fi
  shopt -s nullglob
  for bin in "${cargo_bin}"/*; do
    name="$(basename "${bin}")"
    if [[ -x "${bin}" && ! -e "${dest}/${name}" ]]; then
      if [[ -w "${dest}" ]]; then
        ln -sfn "${bin}" "${dest}/${name}"
      elif command -v sudo >/dev/null 2>&1; then
        sudo ln -sfn "${bin}" "${dest}/${name}"
      fi
    fi
  done
fi

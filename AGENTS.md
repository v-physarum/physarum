# Agent instructions

## Project goal

TODO: the owner has not defined the project purpose (library, CLI, or service). Do not invent a domain, product story, or example use case. Leave purpose-dependent work as a TODO until the owner fills this in.

## Repository layout

- `Cargo.toml`, `src/` — the `physarum` package. One crate today; it can become a Cargo workspace later.
- `docs/adr/` — architecture decision records. See `docs/adr/README.md`.
- `.github/` — CI workflow and issue and pull request templates.
- `rust-toolchain.toml`, `rustfmt.toml` — pinned stable toolchain and formatting.
- `.cursor/` — Cursor cloud-agent install and a pointer rule. The rules live in this file.
- `AGENTS.md` — this file.

## Code style and conventions

- Rust edition 2024. The toolchain is pinned in `rust-toolchain.toml` and includes `rustfmt` and `clippy`.
- Format with rustfmt. Settings are in `rustfmt.toml`.
- Clippy groups `all` and `pedantic` are warnings in `Cargo.toml`. CI denies warnings.
- `unsafe` is forbidden.
- Public items need rustdoc. The crate-level purpose stays a TODO until the owner writes it.
- Comments, docs, issues, and commit messages are in English.
- Keep diffs small. Do not add a domain, product story, or sample use case.

## Build, lint, and test

From the repository root, with the pinned toolchain:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Cargo only forwards `-D` to Clippy when it comes after `--`.

`cargo build` compiles the crate. There is no binary to run yet.

## How agents should work

- Open small pull requests against `main`. Do not merge unless the owner asks.
- Behavior changes need tests. `cargo test` must pass.
- Keep CI green. The commands above are what CI runs.
- Do not expand scope past the requested change.
- If a task needs a project purpose, stop and leave a TODO instead of guessing.

## Cursor Cloud specific instructions

`.cursor/install.sh` installs the toolchain from `rust-toolchain.toml`, including `rustfmt` and `clippy`. After that, the commands in "Build, lint, and test" are enough. No services or secrets are required.

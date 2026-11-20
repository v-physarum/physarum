# Agent instructions

## Project goal

TODO: the owner has not defined the project purpose (library, CLI, or service). Do not invent a domain, product story, or example use case. Leave purpose-dependent work as a TODO until the owner fills this in.

## Repository layout

- `Cargo.toml`, `src/` — the `physarum` package and the workspace root.
- `crates/util/ph_util_foreach/` — procedural macro `impl_foreach`.
- `tests/crates/ph_util_foreach/` — tests of that macro. See the test rules below.
- `benches/crates/ph_util_foreach/` — benchmarks of that macro. See the benchmark rules below.
- `docs/adr/` — architecture decision records. See `docs/adr/README.md`.
- `.github/` — CI workflow and issue and pull request templates.
- `rust-toolchain.toml`, `rustfmt.toml` — pinned stable toolchain and formatting.
- `.cursor/` — Cursor cloud-agent install and pointer rules. The rules live in this file.
- `AGENTS.md` — this file.

## Code style and conventions

- Rust edition 2024. The toolchain is pinned in `rust-toolchain.toml` and includes `rustfmt` and `clippy`.
- Format with rustfmt. Settings are in `rustfmt.toml`.
- Clippy groups `all` and `pedantic` are warnings in `Cargo.toml`. CI denies warnings.
- `unsafe` is forbidden.
- Public items need rustdoc. The crate-level purpose stays a TODO until the owner writes it.
- Comments, docs, issues, and commit messages are in English.
- Keep diffs small. Do not add a domain, product story, or sample use case.

## Macro design rules

Numbered rules for macros in this repository. This section states rule 1. Later rules use the next integer and do not renumber rule 1.

### 1. Pass arguments by name

Macro arguments are always passed by name, for example `name = value`.

Exception: if the macro (function) has exactly one argument, that argument is passed without a name.

## Test and benchmark rules

Numbered rules for tests and benchmarks in this repository. Numbering continues after rule 1. This section states rules 2 through 5. Later rules use the next integer and do not renumber these rules.

### 2. Repository root

Tests and benchmarks live at the repository root: `/tests` and `/benches`.

### 3. Per-crate layout

Per-crate layout: `/tests/crates/{crate_name}/` and `/benches/crates/{crate_name}/`.

### 4. Public API

Tests and benchmarks use only the public API of crates.

### 5. Benchmark measurements

Benchmarks measure: build time in debug and in release; incremental builds only in debug; runtime performance only in release.

## Build, lint, and test

From the repository root, with the pinned toolchain:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Cargo only forwards `-D` to Clippy when it comes after `--`.

`cargo build` compiles the crate. There is no binary to run yet.

Downstream compile time of `impl_foreach!` is a tracked metric. `cargo bench --bench ph_util_foreach_measure` records the measurements in rule 5 with hyperfine.

## How agents should work

- Open small pull requests against `main`. Do not merge unless the owner asks.
- Behavior changes need tests. `cargo test` must pass.
- Keep CI green. The format, clippy, and test commands above are what the check job runs. The user-compile-time job runs `cargo bench --bench ph_util_foreach_measure`.
- Do not expand scope past the requested change.
- If a task needs a project purpose, stop and leave a TODO instead of guessing.

## Cursor Cloud specific instructions

`.cursor/install.sh` installs the toolchain from `rust-toolchain.toml`, including `rustfmt` and `clippy`. After that, the commands in "Build, lint, and test" are enough. No services or secrets are required.

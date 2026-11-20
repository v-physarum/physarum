# Agent instructions

## Project goal

TODO: the owner has not defined the project purpose (library, CLI, or service). Do not invent a domain, product story, or example use case. Leave purpose-dependent work as a TODO until the owner fills this in.

## Repository layout

- `Cargo.toml`, `src/` — the `physarum` package and the workspace root.
- `crates/benches/ph_bench_build/` — hyperfine runner. A bench selects scenarios and runs them.
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
- JSON is read and written with `serde` and `serde_json`. Do not assemble JSON text by hand.
- Keep diffs small. Do not add a domain, product story, or sample use case.

## Macro design rules

Numbered rules for macros in this repository. This section states rule 1. Later rules use the next integer and do not renumber rule 1.

### 1. Pass arguments by name

Macro arguments are always passed by name, for example `name = value`.

Exception: if the macro (function) has exactly one argument, that argument is passed without a name.

Exception: if the last parameter is a block `{ ... }`, no name is required before that block. For example, `impl_foreach!(0..10, {})` instead of `impl_foreach!(0..10, body = {})`.

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

## Script and benchmark tool rules

Numbering continues after rule 5. This section states rules 6 and 7. Later rules use the next integer and do not renumber these rules.

### 6. Rust scripts

Scripts are written in Rust, not shell.

### 7. Hyperfine

Benchmarks use hyperfine.

## Pull request rules

Numbering continues after rule 7. This section states rules 8 and 9. Later rules use the next integer and do not renumber these rules.

### 8. Own pull request

AGENTS.md may be edited only in its own pull request. Do not mix AGENTS.md edits with other changes.

### 9. One crate

Each pull request works on its own crate and does not touch other crates.

## Build, lint, and test

From the repository root, with the pinned toolchain:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Cargo only forwards `-D` to Clippy when it comes after `--`.

`cargo build` compiles the crate. There is no binary to run yet.

`cargo bench --bench ph_util_foreach_runtime` runs the foreach runtime bench.

## How agents should work

- Open small pull requests against `main`. Do not merge unless the owner asks.
- Behavior changes need tests. `cargo test` must pass.
- Keep CI green. The format, clippy, and test commands above are what the check job runs. The runtime bench job runs `cargo bench --bench ph_util_foreach_runtime --offline`.
- Do not expand scope past the requested change.
- If a task needs a project purpose, stop and leave a TODO instead of guessing.

## Cursor Cloud specific instructions

`.cursor/install.sh` installs the toolchain from `rust-toolchain.toml`, including `rustfmt` and `clippy`. After that, the commands in "Build, lint, and test" are enough. No services or secrets are required.

# Agent coding rules

Rules an agent must follow when changing this workspace. Short form. Prefer these IDs in a review note.

## Allocation

### AG-01

A hot loop does not allocate.

#### Don't

- `Vec::new`, `Box::new`, `to_vec`, `clone` of a buffer, or `collect` into a new heap value inside a per-row or per-tick hot walk.

#### Do

- Keep storage outside the loop. `clear` and reuse.

### AG-02

A known size is reserved once.

#### Don't

- Grow `String` / `Vec` by repeated push when the final length is known or bounded.

#### Do

- `with_capacity` / `reserve` from the bound. A table-bound length prefers a fixed array when the type already has one.

### AG-03

Capacity is kept across clears.

#### Don't

- `shrink_to_fit`, replace the buffer, or drop a `mem::take` after warm-up on a reused path.

#### Do

- `clear` keeps the allocation.

## Domain

### AG-04

A number with meaning is a named constant or a typed wrapper.

#### Don't

- Bare literals that encode a rule (`8`, `12`, bit masks) scattered in bodies.

#### Do

- A `const` at the owner of the rule, or a newtype with methods.

### AG-05

A raw value does not travel as domain logic.

#### Don't

- Pass `u32` / `usize` through several functions that each reinterpret the same role (id, count, flag word).

#### Do

- A wrapper type for that role. Methods on the wrapper do the domain work. Crossing an id boundary uses a named constructor, not silent `From`.

### AG-06

Several yes/no choices of one thing are not a pile of `bool`s.

#### Don't

- Three or more `bool` fields or unexplained `bool` arguments for one concept.

#### Do

- One `enum` for a single choice. One bit set for several independent flags of one thing. A call that needs a side uses an enum argument.


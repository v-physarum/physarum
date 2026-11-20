# physarum

TODO: the project purpose is not defined. The owner should replace this paragraph with a short description.

## Crates

- `physarum` (repository root) — package skeleton. TODO: the owner fills in the purpose.
- [`ph_util_foreach`](crates/util/ph_util_foreach/README.md) — `impl_foreach` procedural macro.

## Build

Install the Rust toolchain pinned in [`rust-toolchain.toml`](rust-toolchain.toml) (stable, with `rustfmt` and `clippy`), then:

```bash
cargo build
```

## Test

```bash
cargo test
```

## Lint

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

Cargo only forwards `-D` to Clippy when it comes after `--`.

## Run

There is no binary or service entry point yet. TODO: document how to run the project after the owner decides whether it is a library, a CLI, or a service.

## License

Licensed under the [MIT License](LICENSE).

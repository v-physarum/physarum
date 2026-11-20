# Architecture

Workspace crates. The facade package does not link the probe compiler.

| Crate | Path | Role |
| --- | --- | --- |
| `physarum` | repository root | Facade package. |
| `ph_bench_build` | `crates/benches/ph_bench_build` | Hyperfine runner for downstream benches. |
| `ph_build_test` | `crates/tests/ph_build_test` | Debug probe driver. `Probe` returns compiler diagnostics. |

# Architecture

Workspace crates. The facade package does not link the test assert crates.

| Crate | Path | Role |
| --- | --- | --- |
| `physarum` | repository root | Facade package. |
| `ph_bench_build` | `crates/benches/ph_bench_build` | Hyperfine runner for downstream benches. |
| `ph_test_asm` | `crates/tests/ph_test_asm` | `AsmTester` and `AsmCompare` for tests. |

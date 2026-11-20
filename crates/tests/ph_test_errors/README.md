# ph_test_errors

Compile-fail asserts for debug probes. `ErrorTester` is the public API. Each probe is its own crate. The check builds it with `rustc` at `opt-level=0` and edition 2024, then matches an error code and a description without allocating. A mismatch panics with the codes and messages that arrived.

This crate does not emit LLVM IR or assembly.

```rust
use ph_test_errors::ErrorTester;

ErrorTester::source("fn main() { missing; }")
    .assert_code_description("E0425", "cannot find");

ErrorTester::file("fail.rs").assert_code_description("E0308", "mismatched types");
```

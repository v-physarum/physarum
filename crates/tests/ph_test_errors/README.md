# ph_test_errors

Compile-fail asserts for debug probes. `ErrorTester` is the public API. Each probe is its own crate. The first assert builds it with `rustc` at `opt-level=0` and edition 2024. Later asserts on the same chain walk the parsed diagnostics without allocating.

This crate does not emit LLVM IR or assembly.

```rust
use ph_test_errors::ErrorTester;

ErrorTester::source("fn main() { missing; }")
    .assert_code("E0425")
    .assert_description("cannot find");

ErrorTester::file("ok.rs").assert_pass();
```

`assert_description` matches a substring of an error message. `assert_code_description` matches one error's code and message. `assert_note` and `assert_help` match those child diagnostics. `assert_warning_code` matches a warning code.

# ph_test_errors

`ErrorTester` is the only public type.

- `file` reads a probe from a path.
- `source` uses the given text. `rustc` sees it as `probe.rs`.
- `assert_code_description` is success when an error has that code and its message contains the description. A mismatch panics with the codes and messages that arrived.

Each probe is its own crate, built with `rustc` at `opt-level=0` and edition 2024. The match does not allocate.

```rust
use ph_test_errors::ErrorTester;

ErrorTester::source("fn main() { missing; }")
    .assert_code_description("E0425", "cannot find");

ErrorTester::file("fail.rs").assert_code_description("E0308", "mismatched types");
```

# ph_test_errors

`ErrorTester` is the only public type.

- `file` reads a probe from a path.
- `source` uses the given text. `rustc` sees it as `probe.rs`.
- `extern_crate` passes one `--extern name=path`. Call it again for another crate.
- `assert_code_description` is success when an error has that code and its message contains the description. A diagnostic with no code does not match. A mismatch panics with the codes and messages that arrived.
- `assert_description` is success when an error has no code and its message contains the description. `compile_error!` is this case. An error that has a code does not match. A mismatch panics with the codes and messages that arrived.

Each probe is its own crate, built with `rustc` at `opt-level=0` and edition 2024. The match does not allocate.

```rust
use ph_test_errors::ErrorTester;

ErrorTester::source("fn main() { missing; }")
    .assert_code_description("E0425", "cannot find");

ErrorTester::file("fail.rs").assert_code_description("E0308", "mismatched types");

ErrorTester::source("compile_error!(\"stop\");").assert_description("stop");

ErrorTester::source("pub fn probe() { demo::missing(); }")
    .extern_crate("demo", "libdemo.rlib")
    .extern_crate("other", "libother.rlib")
    .assert_code_description("E0425", "cannot find function");
```

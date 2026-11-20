# ph_test_errors

`ErrorTester` is the only public type.

- `file` reads a probe from a path.
- `source` uses the given text. `rustc` sees it as `probe.rs`.
- `extern_crate` passes one `--extern name=path`. Call it again for another crate.
- `assert_code` is success when an error has that code. A mismatch panics with the codes and messages that arrived.
- `assert_description` is success when an error message contains the description. A diagnostic whose code is null matches. `compile_error!` is that case. A mismatch panics with the codes and messages that arrived.

Each probe is its own crate, built with `rustc` at `opt-level=0` and edition 2024. The match does not allocate.

```rust
use ph_test_errors::ErrorTester;

ErrorTester::source("fn main() { missing; }").assert_code("E0425");

ErrorTester::source("fn main() { missing; }").assert_description("cannot find");

ErrorTester::file("fail.rs").assert_code("E0308");

ErrorTester::source("compile_error!(\"stop\");").assert_description("stop");

ErrorTester::source("pub fn probe() { demo::missing(); }")
    .extern_crate("demo", "libdemo.rlib")
    .assert_code("E0425");
```

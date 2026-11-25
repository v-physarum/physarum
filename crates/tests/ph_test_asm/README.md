# ph_test_asm

Assembly assertions for tests. Probes compile as release in this crate. `assert_noalias` reads the LLVM IR signature and checks that every pointer parameter is `noalias`. There is no debug profile and no profile switch.

`AsmTester` checks one function or its hot loop. `AsmCompare` checks that two probes match after symbols are sorted and local labels are stripped. A failed compare panics with a unified diff.

```rust
use ph_test_asm::AsmTester;

AsmTester::new("probe.rs")
    .function("add")
    .hot_loop(|hot| {
        hot.assert_simd();
        hot.assert_no_bounds();
        hot.assert_no_branch();
        hot.assert_no_virtual_calls();
        hot.on_target("x86_64-unknown-linux-gnu")
            .assert_max_instructions(64);
    });
```

```rust
use ph_test_asm::AsmCompare;

AsmCompare::files("a.rs", "b.rs").assert_equivalent();
AsmCompare::files("a.rs", "b.rs").function("k").assert_equivalent();
AsmCompare::files("a.rs", "b.rs").function("k").hot_loop().assert_equivalent();
```

A probe can link a library crate. `extern_crate` passes `--extern name=path` and `-L dependency=dir`, where `dir` is the directory of `path`. The crates that library depends on must sit in that directory. `AsmCompare` has the same method and links the crate into both probes.

```rust
use ph_test_asm::AsmTester;

AsmTester::new("probe.rs")
    .extern_crate("demo", "target/release/libdemo.rlib")
    .function("on_tick")
    .on_function(|body| body.assert_no_virtual_calls());
```

The asserts read ELF and Mach-O assembly for `x86_64` and `aarch64`. A function is named by its Rust symbol. The Mach-O `_` prefix is not part of the name. The host target is fixed when this crate is built. `on_target` reads the format and instruction set from the triple.

`on_function` takes the same asserts as `hot_loop`. The hot loop may keep its latch branch. The function may not.

`Rules` collects those asserts and applies them in one call. `assert_max_instructions` is target-specific, so it stays on `on_target`.

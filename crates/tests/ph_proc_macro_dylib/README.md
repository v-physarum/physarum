# ph_proc_macro_dylib

`proc_macro_library` is the only public item.

It returns the newest built proc-macro library for a cargo package name. The file is `lib<name>.dll` on Windows, `lib<name>.dylib` on macOS, and `lib<name>.so` elsewhere. A hashed copy `lib<name>-<hash>.<ext>` under `deps` is the same library.

Search roots are `CARGO_TARGET_DIR`, ancestors of the running executable, and `target` next to this crate's manifest and its parents. From `crates/tests/<crate>` those parents include the workspace root. Inside a root the layout is the directory itself, `debug`, `deps`, `debug/deps`, and `build/<name>/*/out` with or without `debug`.

```rust
use ph_proc_macro_dylib::proc_macro_library;

let path = proc_macro_library("ph_util_foreach");
```

A name that is not a cargo package name panics. A missing file panics with the directories that were searched.

# ph_beans_derive

`#[derive(Module)]` implements `BeanInfo` for a struct.

```rust
use ph_beans_derive::Module;

#[derive(Module)]
struct Count(u8);

#[derive(Module)]
#[config(type = module)]
#[module(state = u8)]
#[provide(Id::T, Keys::MUT)]
#[inject(App::T)]
struct Logic;
```

A struct with fields is a data bean: `State` is `Self`. A unit struct is a module. `Type::T` is a shared claim and `Type::MUT` is an exclusive claim. Provide accepts at most four claims. Inject accepts at most three. `#[module(state = ...)]` sets the module state; without it the state is `()`.

## License

Licensed under the [GNU GPL v3 or later](../../LICENSE) (`GPL-3.0-or-later`).

# ph_beans_derive

`#[derive(Component)]` implements [`ph_beans`](../ph_beans/README.md) for a struct.

```rust
use ph_beans::Tick;
use ph_beans_derive::Component;

#[derive(Component, Default)]
struct Count(u32);

#[derive(Component)]
#[inject(Count)]
struct Child;

#[derive(Component)]
#[provider(Count, Child)]
struct Parent;
```

A struct with fields is a data bean: its state is itself, and `create` calls `Default`. A unit struct is a module. `#[provider]` owns beans. `#[inject]` receives them from the parent only.

The derive rejects a bean named twice, a bean listed in both attributes, a generic component, a provider list longer than four, and an inject list longer than three. A missing provider is reported by [`ph_beans::At`] when the parent is scheduled.

## License

Licensed under the [GNU GPL v3 or later](../../LICENSE) (`GPL-3.0-or-later`).

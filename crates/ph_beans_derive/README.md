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

A struct with `#[provider]` or `#[inject]` is a module and must be a unit struct. A struct with neither is a data bean: its state is itself, and `create` calls `Default`.

The derive rejects a bean named twice, a bean listed in both attributes, a provider list longer than four, and an inject list longer than three. A missing provider is reported by [`ph_beans::At`] when the parent is scheduled.

## License

Licensed under the [GNU GPL v3 or later](../../LICENSE) (`GPL-3.0-or-later`).

# ph_beans

A module owns the beans in `#[provider(..)]` and receives `#[inject(..)]` beans from its immediate parent. `tick` runs provided modules first, then the module's own `Tick`. Types and monomorphization choose every slot. There is no registry and no `dyn` dispatch.

The author writes `impl Tick` for a module and `Default` for a data bean. [`ph_beans_derive`](../ph_beans_derive/README.md) emits the state, the bean tuple, and the schedule.

`#[provider]` accepts at most four beans. `#[inject]` accepts at most three. The parent's fourth slot is the child module itself.

## License

Licensed under the [GNU GPL v3 or later](../../LICENSE) (`GPL-3.0-or-later`).

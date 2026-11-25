# ph_beans

A bean pipeline records resource claims (`.inject_resources`), init, maps/filters/emit, and `.build()`. [`Descriptor`](src/lib.rs) carries a single status for that chain. [`Setup::setup`](src/query/setup.rs) returns a tick only when the status is ready. Design: [`docs/sdd/design.md`](docs/sdd/design.md). Error codes: [`docs/errors.md`](docs/errors.md). Performance requirements: [`docs/performance.md`](docs/performance.md).

```rust
use ph_beans::{At, BeanInfo, Get, Ix, Marker, Mut, Query, Setup, Tick};

#[derive(Default)]
struct Id;
#[derive(Default)]
struct Keys;
struct App;

impl BeanInfo for Id {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u8;
}

impl BeanInfo for Keys {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u8;
}

impl BeanInfo for App {
    type Provides = (Get<Id>, Mut<Keys>);
    type Injects = ();
    type Beans = (Get<Id>, Mut<Keys>);
    type State = u8;
}

impl At<Id> for App {
    type Ix = Ix<0>;
}

impl At<Keys> for App {
    type Ix = Ix<1>;
}

// `*Id::T` lends the state of `Id`; `Id::T` would lend the bean itself.
fn boot(args: (&u8, &mut u8)) -> u8 {
    let (id, keys) = args;
    id.wrapping_add(*keys)
}

impl Setup for App {
    fn setup(self) -> impl Tick<Self> {
        App::query()
            .inject_resources(*Id::T & *Keys::MUT)
            .init_state(boot)
            .build()
    }
}

fn main() {
    let tick = App.setup();
    let mut instance = tick.init(()); // slots of Id and Keys, then boot once
    tick.emit((), &mut instance, ()); // the caller lends injects; App has none
}
```

## License

Licensed under the [GNU GPL v3 or later](../../LICENSE) (`GPL-3.0-or-later`).

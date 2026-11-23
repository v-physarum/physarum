# ph_beans

A bean pipeline records resource claims (`.inject_resources`), init, maps/filters/emit, and `.build()`. [`Descriptor`](src/lib.rs) carries a single status for that chain. [`Setup::setup`](src/query/setup.rs) returns a tick only when the status is ready. Design: [`docs/sdd/design.md`](docs/sdd/design.md).

```rust
use ph_beans::{At, BeanInfo, Get, Ix0, Ix1, Marker, Proxy, Query, Setup, Stop, Target, Tick};

struct Id(u8);
struct Keys(u8);
struct App;

impl BeanInfo for Id {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u8;
    type Proxy = Proxy<Id>;
    type Link = Get<Id>;
    type Chain = Stop;
}

impl BeanInfo for Keys {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u8;
    type Proxy = Proxy<Keys>;
    type Link = Get<Keys>;
    type Chain = Stop;
}

impl BeanInfo for App {
    type Provides = (Id, Keys);
    type Injects = ();
    type Beans = (Id, Keys);
    type State = u8;
    type Proxy = Proxy<App>;
    type Link = Get<App>;
    type Chain = Stop;
}

impl At<Id> for App {
    type Ix = Ix0;
}

impl At<Keys> for App {
    type Ix = Ix1;
}

impl Target for Id {}
impl Target for Keys {}
impl Target for App {}

fn boot(args: (&Id, &mut Keys)) -> u8 {
    let (id, keys) = args;
    id.0.wrapping_add(keys.0)
}

impl Setup for App {
    fn setup(self) -> impl Tick<Self> {
        App::query()
            .inject_resources(Id::T & Keys::MUT)
            .init_state(boot)
            .build()
    }
}
```

## License

Licensed under the [GNU GPL v3 or later](../../LICENSE) (`GPL-3.0-or-later`).

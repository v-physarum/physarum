# `ph_beans` API guide

This guide has two parts. Part 1 is the API that exists in the crate today,
written with manual impls; its full example (section 9) was compiled and run
against the library. Part 2 is the planned `ph_derive` form of the same
declarations; **none of Part 2 is implemented**. Section 13 lists what is
missing; section 14 is the planned filter-as-module shape.

Error codes are in [`errors.md`](errors.md), performance requirements in
[`performance.md`](performance.md), the design rules in
[`sdd/design.md`](sdd/design.md).

## Part 1 — the API today

### 1. Beans and state

A bean is a zero-sized type. Its data is its state, `BeanInfo::State`.

```rust
#[derive(Default)]
struct Id;

impl BeanInfo for Id {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u8;
}
```

A bean a module provides needs `Default`, and so does its state: the module
creates both when it makes an instance (`PipelineError<3>`).

A bean without state names `NoState`: `type State = NoState;`. Only the
library makes that value. `()` is not "no state": it is an ordinary state
type, with an init function like any other.

### 2. Modules: provides and injects

```rust
impl BeanInfo for App {
    type Provides = (Get<Id>, Mut<Keys>, Mut<Update>);
    type Injects = ();
    type Beans = (Get<Id>, Mut<Keys>, Mut<Update>); // Provides, then Injects
    type State = u8;
}
impl At<Id> for App { type Ix = Ix<0>; }
impl At<Keys> for App { type Ix = Ix<1>; }
impl At<Update> for App { type Ix = Ix<2>; }
```

- `Provides` lists the beans the module owns, each under its marker:
  `Get<Id>` or `Mut<Id>`. A bean with no marker is `PipelineError<5>`. It is
  always a list: one provide is `(Get<Id>,)`.
- The marker says who may change the bean. Under `Get<Id>` the bean is made
  by `init` and never changes: a system of the module that claims `Id::MUT`
  or `*Id::MUT` is `PipelineError<4>`, and a child injects it only as shared.
  `Mut<Id>` can be taken shared or exclusive, by the module's systems and by a
  child (`FromParent`).
- `Injects` lists what its caller lends: `Get<P>` (shared) or `Mut<P>`
  (exclusive).
- `Beans` is `Provides` followed by `Injects`. One `At<P>` impl per bean gives
  its position there.
- A module holds at most 4 beans, 3 when it has injects.

### 3. Events

An event is a bean with `EventInfo`. `Parents` names the events it nests under.

```rust
impl EventInfo for Update { type Parents = (); }
impl EventInfo for Draw { type Parents = Update; }
```

- `Update::ON` is the event. `Update::ON[Draw::ON]` is `Draw` under `Update`;
  the outer event must be in the inner event's `Parents` (`FilterError<1>`).
- Filters combine with `&`, `|` and `!`.

### 4. The registry of emit bits

Each event takes one bit of a registry. The registry is any type; the app
states the owner of each slot once.

```rust
struct Events;
impl SlotOf<Events> for Bit<0> { type Owner = Update; }
impl EmitBit for Update { type Reg = Events; type Slot = Bit<0>; }
impl EventReg for App { type Reg = Events; }
```

- A registry has 128 bits (`EmitMask::BITS`).
- Two owners of one slot are rejected by the compiler (`E0119`); a slot owned
  by another event is `E0271`; a slot with no owner is `FilterError<5>`.
- A module names its registry with `EventReg`. An event outside it, or a path
  over two registries, is `FilterError<4>`. A module with no events needs no
  `EventReg`.

### 5. Claims and systems

A system is a plain function with no captures. It takes the view of the claim
that is live at its `.map` call; with no claim it takes `()`.

| Claim | View |
| --- | --- |
| `Id::T` | `&Id` — the bean |
| `Id::MUT` | `&mut Id` |
| `*Id::T` | `&Id::State` — the state |
| `*Id::MUT` | `&mut Id::State` |

Claims join with `&`: `*Id::T & *Keys::MUT` gives `(&u8, &mut u8)`. Three
claims give a nested pair. A system never emits and never defers.

### 6. The pipeline

A pipeline starts with `Module::query()` and ends with `.build()`. Every step
changes the type, so a mistake is a build error with a code.

| Step | What it does |
| --- | --- |
| `.inject_resources(claim)` | sets the claim for the systems that follow |
| `.init_state(fn)` | stores the init function; its result is the module state |
| `.filter(path)` | the systems that follow run only for this event |
| `.map(fn)` | one system; at most 16 (`PipelineError<2>`) |
| `.emit(path)` | declares an event that can be deferred in this pipeline |
| `.emit_providers()` | declares the event of `Provides = (E,)` |
| `.build()` | seals the pipeline |

- `.init_state` is called once (`PipelineError<1>`). A default state is
  `.init_state(|_| Default::default())`.
- The init function returns the state of the module, nothing else
  (`PipelineError<9>`). A module with a state is not built without it
  (`NeedInit`).
- A module with `State = NoState` takes no `.init_state`
  (`PipelineError<9>`): it goes from its systems straight to `.build()`.
- After an exclusive claim, the next `.inject_resources` needs a wait filter
  (`FilterError<2>`).
- `Setup::setup` returns the pipeline as `impl Tick<Self>`.

### 7. Running

The pipeline holds only systems. `init` makes an instance; `emit` and `defer`
take it.

```rust
let tick = App.setup();
let mut app = tick.init(());                // provided slots, then init once
tick.emit(Update::ON, &mut app, ());        // direct: runs now
tick.defer(Update::ON[Draw::ON], &mut app); // deferred
```

- `init(injects)` creates the provided beans and states with `Default`, then
  runs the init function once over its claim.
- `emit(event, &mut instance, injects)` runs the systems whose filter matches
  the event, in pipeline order. The event is a build-time constant: a system
  that does not match is not in the generated code (PERF-1).
- `injects` is what the module injects, lent by the caller for this call: `()`
  with none, `&Slot<P>` for `Get<P>`, `&mut Slot<P>` for `Mut<P>`, a pair for
  two. A `Slot<P>` is a bean and its state: `Slot::new(Keys, 0)`.
- `tick.state(&instance)` is the module state: what the init function
  returned.
- One pipeline can run several instances.
- `Tick::DESC` (`ModuleDesc`) describes the pipeline at build time: what
  `.emit` declares, and whether an emit reads an event buffer.

### 8. Deferred events

`defer` does not run anything. The event waits in the buffer of its scope
event, the outermost event of its path, and runs the next time that event is
emitted, after the event's own systems.

- The scope event keeps the buffer in its state: `type State = EventBuf`, or a
  struct that holds an `EventBuf` and implements `DeferState`.
- The module holds the scope event as a bean under `Mut` (`Provides`), with
  its `At` impl.
  Otherwise the declaration is `FilterError<7>`.
- The pipeline declares the path with `.emit(path)`. A `defer` of an
  undeclared path is `FilterError<6>` (reported by `cargo build`, not by
  `cargo check`).
- Deferred events run in the order of the `defer` calls. An event deferred
  twice runs twice. One entry runs the systems of its path: the event and its
  parents.
- A pipeline with no `.emit` reads no buffer.

### 8a. Injecting a trait

A module can inject a bean as a trait instead of a type. Its `Injects` names
the trait; the pipeline and the systems do not change shape.

```rust
trait Painter { fn tint(&mut self, by: u8); }

// What the derive of a `dynamic` trait writes.
impl<'a> DynObject<'a> for dyn Painter {
    type Obj = dyn Painter + 'a;
    fn shared(&'a self) -> &'a Self::Obj { self }
    fn exclusive(&'a mut self) -> &'a mut Self::Obj { self }
}

impl BeanInfo for Ui {
    type Provides = ();
    type Injects = Dyn<Mut<Draw>, dyn Painter>; // #[inject(Draw::MUT as Painter)]
    type Beans = Dyn<Mut<Draw>, dyn Painter>;
    type State = NoState;
}
impl At<Draw> for Ui { type Ix = Ix<0>; }

fn tint(painter: &mut dyn Painter) { painter.tint(5); }

let tick = Ui::query()
    .inject_resources(Draw::MUT) // the ordinary claim
    .map(tint)
    .build();

let mut draw = Draw(1);
let mut ui = tick.init(&mut draw);     // `&mut Draw` becomes `&mut dyn Painter` here
tick.emit((), &mut ui, &mut draw);
```

- The operator knows nothing about `dyn`. The claim is `Draw::T` /
  `Draw::MUT` (or `*Draw::T`); the module's entry for `Draw` decides that the
  system takes the trait object.
- The cast happens at the boundary: the caller passes its concrete value where
  `init` and `emit` expect `&dyn Painter`. Any implementation of the trait
  can be lent.
- A bean injected with `Get` is lent shared: an exclusive claim on it is
  `PipelineError<4>`.
- The concrete type is known at the call, so the call into the trait is
  resolved at build time (asm test `a_dyn_inject_has_no_virtual_call`).
- A system is written the usual way: `fn tint(painter: &mut dyn Painter)`.
  `DynObject` is what makes that possible: it names the trait object for the
  borrow of one call. Its body is the same for every trait.
- `Beans` must list the module's entries exactly: the view of a claim is read
  from the entry at the bean's `At` index.

### 8b. Running a child

A bean a module provides can have its own pipeline. The parent names it with
`.forward`, and the library runs it: a system never emits.

```rust
// Desk provides Panel and the Keys that Panel injects.
impl BeanInfo for Desk {
    type Provides = (Mut<Panel>, Mut<Keys>);
    type Injects = ();
    type Beans = (Mut<Panel>, Mut<Keys>);
    type State = u8;
}

impl Setup for Panel { /* its own pipeline; Injects = Mut<Keys> */ }

Desk::query()
    .inject_resources(*Panel::T & *Keys::T)
    .init_state(desk_boot)
    .filter(Update::ON)
    .map(before)          // a system of Desk
    .forward(Panel::MUT)  // the pipeline of Panel, for the same event
    .map(after)
    .build()
```

- `.forward(P::MUT)` sits under the current filter like a system. When the
  filter matches the emitted event, the pipeline of `P` runs for that same
  event; `P`'s own filters decide which of its systems run. A child is not
  reached without this step.
- A deferred event starts at the parent. When it runs, each level forwards
  it to its child with a direct call; the child tests the same path against
  its own filters.
- `.forward` takes several children joined with `&`:
  `.forward(Panel::T & Dock::MUT)`.
- The marker of a `.forward` is the marker of the provide: `P::T` names a
  child provided under `Get`, `P::MUT` a child provided under `Mut`
  (`PipelineError<8>` otherwise). The call site then says what the child
  can do. A child under `Get` changes nothing of its own: every bean it
  provides is under `Get` too (`PipelineError<7>` otherwise), and its
  systems can change only what it injects. A child under `Mut` may change
  its provides. The state of a child is made by its init and no system
  changes it, under either marker. The order written is the order their
  pipelines run in, and the order of their init.
- `Tick::init` of the parent runs the init system of every forwarded child
  first, then the parent's own init function: the parent sees what its
  children made. A child named by several `.forward` steps is initialised
  once, at the place of the first step that names it; its pipeline runs at
  each step.
- The child's injects are beans of the parent: its provides or its own
  injects. They are lent to the child's init and to its systems.
- The parent keeps a node for each provide: the bean's slot and the nodes of
  the bean's own provides. The whole tree of a module is one value.
- `P` needs `Setup` and `Default`, and is provided under `Mut`. Anything else
  is `PipelineError<6>`.
- The child's systems are inlined into the parent's emit (PERF-1, probe
  `emit_into_child`).

### 9. Full example

```rust
use ph_beans::{
    At, BeanInfo, Bit, EmitBit, EventBuf, EventInfo, EventReg, Get, Ix, Marker, Mut, NoState,
    Query, Setup, Slot, SlotOf, Tick,
};

// Beans: zero-sized types. The data of a bean is its state.
#[derive(Default)]
struct Id;
#[derive(Default)]
struct Keys;

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

// Events. `Update` takes deferred events, so its state holds the buffer.
#[derive(Default)]
struct Update;
struct Draw;

impl BeanInfo for Update {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = EventBuf;
}

impl BeanInfo for Draw {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl EventInfo for Update {
    type Parents = ();
}

impl EventInfo for Draw {
    type Parents = Update;
}

// The registry of emit bits: one slot per event.
struct Events;

impl SlotOf<Events> for Bit<0> {
    type Owner = Update;
}

impl SlotOf<Events> for Bit<1> {
    type Owner = Draw;
}

impl EmitBit for Update {
    type Reg = Events;
    type Slot = Bit<0>;
}

impl EmitBit for Draw {
    type Reg = Events;
    type Slot = Bit<1>;
}

// The module.
struct App;

impl BeanInfo for App {
    type Provides = (Get<Id>, Mut<Keys>, Mut<Update>);
    type Injects = ();
    type Beans = (Get<Id>, Mut<Keys>, Mut<Update>); // Provides, then Injects
    type State = u8;
}

impl At<Id> for App {
    type Ix = Ix<0>;
}

impl At<Keys> for App {
    type Ix = Ix<1>;
}

impl At<Update> for App {
    type Ix = Ix<2>;
}

impl EventReg for App {
    type Reg = Events;
}

// Systems: plain functions over the view of the live claim.
fn boot(args: (&u8, &mut u8)) -> u8 {
    let (id, keys) = args;
    *keys = 1;
    id + *keys
}

fn on_update(args: (&u8, &mut u8)) {
    let (_, keys) = args;
    *keys += 1;
}

fn on_draw(args: (&u8, &mut u8)) {
    let (_, keys) = args;
    *keys += 10;
}

impl Setup for App {
    fn setup(self) -> impl Tick<Self> {
        App::query()
            .inject_resources(*Id::T & *Keys::MUT)
            .init_state(boot)
            .filter(Update::ON)
            .map(on_update)
            .filter(Update::ON[Draw::ON])
            .map(on_draw)
            .emit(Update::ON[Draw::ON])
            .build()
    }
}

// A module with an inject. Its caller lends the slot of `Keys`.
struct Logic;

impl BeanInfo for Logic {
    type Provides = ();
    type Injects = Mut<Keys>;
    type Beans = Mut<Keys>;
    type State = NoState;
}

impl At<Keys> for Logic {
    type Ix = Ix<0>;
}

fn grow(keys: &mut u8) {
    *keys += 10;
}

impl Setup for Logic {
    fn setup(self) -> impl Tick<Self> {
        Logic::query()
            .inject_resources(*Keys::MUT)
            .map(grow)
            .build()
    }
}

fn main() {
    let tick = App.setup();
    let mut app = tick.init(()); // slots of Id, Keys, Update; then `boot` once

    tick.emit(Update::ON, &mut app, ()); // on_update runs now
    tick.defer(Update::ON[Draw::ON], &mut app); // waits in Update's buffer
    tick.emit(Update::ON, &mut app, ()); // on_update, then on_draw

    let tick = Logic.setup();
    let mut keys = Slot::new(Keys, 0);
    let mut logic = tick.init(&mut keys);
    tick.emit((), &mut logic, &mut keys);
    assert_eq!(keys.state, 10);
}
```

### 10. Errors

Every failure is a build error that names a code; the tests match the code.
The table is [`errors.md`](errors.md). Codes in use: `FilterError<1>`, `<2>`,
`<4>`–`<7>`, `PipelineError<1>`–`<6>`, `ClaimError<T, 1>` (bean `T` named
twice in one claim list) and `ClaimError<T, 2>` (shared and exclusive).

## Part 2 — planned: `ph_derive`

**Not implemented.** The crate `ph_derive` does not exist. This part records
the decisions taken for it. The derive emits only meta traits; `Query`,
`Setup`, `Tick` and the pipeline stay hand-written.

### 11. One derive, one attribute

```rust
#[derive(Component)]
#[config(type = module, state = u8, events = (Update, Draw))]
#[provide(Id::T, Keys::MUT)]
struct App;

#[derive(Component)]
#[config(type = event, parents = Update, state = EventBuf)]
struct Draw;

#[derive(Component)]
#[config(type = filter)]
struct CustomFilter;

#[config(type = dynamic)]
trait Painter { fn paint(&self); }
```

| `type` | On | Emits |
| --- | --- | --- |
| `module` | zero-sized struct | `BeanInfo`, one `At` per bean; with `events`, the registry |
| `event` | zero-sized struct | `BeanInfo`, `EventInfo`, a hidden slot macro; a filter module |
| `filter` | zero-sized struct | `BeanInfo`; a filter module without an emit bit |
| `proxy` | zero-sized struct | `BeanInfo`, `ProxyInfo` |
| `dynamic` | trait | `DynObject` for the trait object |

| Key | Where | Meaning |
| --- | --- | --- |
| `state = T` | any bean | `BeanInfo::State`; `()` without the key |
| `parents = X` / `(X, Y)` | `event` | `EventInfo::Parents` |
| `events = (A, B, …)` | `module` | the module's events; the bit is the position in the list |
| `proxy = X::T` | a bean under a proxy | the link to the proxy |

### 12. Decisions

- `#[provide(Id::T, Keys::MUT)]`: the marker decides what a child may take.
  `Keys::MUT` can be injected as `T` or `MUT`, `Id::T` only as `T`.
- `#[inject(Keys::MUT)]` names the access the module asks of its parent.
- A provide is an instance. The bindings between a parent and the instances
  of its children are generated at the parent.
- A child sees the injects and the events of its parent.
- The bit registry is generated through hidden macros: the event's derive
  emits a macro, and the module's derive calls it with the registry and the
  position.
- A child takes the registry of its parent. `EventReg` stays in `ph_beans`:
  it is what lets `Tick::emit` reject an event of another registry. The
  derive of the parent writes `impl EventReg` for the module and for every
  child it provides, so a child declares nothing.
- `dynamic` is set on the trait. The inject is `#[inject(Draw::T as Painter)]`;
  the view is taken from the bean's state (`*Id::T`) when that state
  implements the trait.

Open: the type the parent's derive puts the parent-to-child binding in, and
the format of derive errors.

## 13. Not implemented

In `ph_beans`:

1. Limits of `.forward`. A child cannot inject its parent itself, nor a bean as a trait. The parent does not check that
   the child's `EventReg` names the same registry.
2. A child names its registry by hand (`impl EventReg`). The derive of its
   parent will write it (section 12).
3. `ph_derive` does not exist, so the `DynObject` impl of a `dynamic` trait
   is written by hand.
4. Proxy: no code.
5. A user state struct with `DeferState` has no test; `State = EventBuf` has.
6. `FilterError<6>` is a `const` assert: `cargo build` reports it,
   `cargo check` does not, and no test asserts it.

`ph_derive`: everything in Part 2, including section 14.

### 14. Filters as configurable modules

**Not implemented.** Part 1 filters are event paths and wait markers
(`Update::ON`, `Update::ON[Draw::ON]`, `Update::ON[Keys::MUT]`, combined with
`&` `|` `!`). This section is the planned shape: a **filter is a module**.
Event filters are the same thing; their derive type is `event`.

#### Main concept

1. **Highly configurable.** A filter is not a hard-coded predicate. It is a
   bean with `Setup`, the same pipeline language as any module: claims,
   filters, maps, build. What it matches, what it waits on, and what it
   touches are written in that pipeline, not in a separate filter API.

| Derive `type` | Filter module | Extra |
| --- | --- | --- |
| `event` | yes | emit bit, `EventInfo` / parents, usable as `Ev::ON[…]` |
| `filter` | yes | no emit bit; a predicate leaf in a path |

```rust
#[derive(Component)]
#[config(type = event, parents = Update, state = EventBuf)]
struct Draw;

impl Setup for Draw {
    fn setup(self) -> impl Tick<Self> {
        Draw::query()
            .inject_resources(Id::MUT)
            .filter(Update::ON)
            .build()
    }
}

#[derive(Component)]
#[config(type = filter)]
struct CustomFilter;

impl Setup for CustomFilter {
    fn setup(self) -> impl Tick<Self> {
        CustomFilter::query()
            .inject_resources(Id::MUT)
            .filter(Update::ON)
            .build()
    }
}
```

- `type = event` and `type = filter` are both filter modules. The difference
  is the role in the registry: an event owns a bit and nests under
  `parents`; a plain filter does not.
- `Setup` configures the filter. The pipeline is the configuration: which
  claims it needs, which event scope it sits under, which wait markers it
  clears. There is no second builder for filters.
- A filter with no state uses `NoState` like any other bean. A filter that
  keeps match state names `state = …` on the derive.

#### Use

An event filter is the path head / nest today (`Update::ON`,
`Update::ON[Draw::ON]`). A plain filter is a leaf under that path, or a
predicate on the current scope:

```rust
App::query()
    .inject_resources(*Id::T & *Keys::MUT)
    .init_state(boot)
    .filter(Update::ON[Draw::ON])           // event filter modules
    .map(on_draw)
    .filter(Update::ON[CustomFilter::T])    // plain filter leaf
    .map(on_match)
    .build()
```

- `Draw::ON` / `CustomFilter::T` / `CustomFilter::MUT` are the leaf markers.
  For a plain filter the marker says whether it may change its own state
  while it matches; for an event the nest is still `Ev::ON[…]`.
- Filters still combine with `&`, `|` and `!`:
  `.filter(Update::ON[CustomFilter::T & Alive::T])`,
  `.filter(CustomFilter::T | Other::T)`.

#### Why a module

| Today (Part 1) | Planned |
| --- | --- |
| Events are beans with `EventInfo` only | Events are filter modules (`type = event`) with `Setup` |
| Path leaves are wait markers | Path leaves may also be `type = filter` beans |
| Wait / scope policy is fixed by the AST node | Policy comes from the filter's `Setup` |
| One shared language for app modules only | The same language configures event and plain filters |

A filter that needs resources injects them. A filter that should run only
on `Draw` under `Update` says so with `.filter(Update::ON[Draw::ON])`
inside its own `setup`. A filter that clears a live `Mut` before the next
claim names that wait in its pipeline. The caller's `.filter(…)` then
names the filter module; it does not restate that configuration.

Open: how a filter's `Tick` participates in emit (inline vs gate), whether
a filter may `.forward` children, and the exact meta trait the derive
emits for an event vs a plain filter leaf.

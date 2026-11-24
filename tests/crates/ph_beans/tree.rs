//! Query slice plus module injects and `Tick::emit`.
//!
//! A second `init_state` and a missing `.build()` are compile-fail doctests.
//! `MUT` against an immutable provide is a compile-fail doctest on [`FromParent`].

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use ph_beans::{
    At, BeanInfo, EventInfo, Get, Ix0, Ix1, Marker, Module, Mut, OfInject, OfParent, OfProvide,
    OfProvide1, Query, Setup, Tick,
};

struct Id(u8);
struct Keys(u8);
struct App;
struct Update;

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
    type Provides = (Id, Keys);
    type Injects = ();
    type Beans = (Id, Keys);
    type State = u8;
}

impl BeanInfo for Update {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}

impl EventInfo for Update {
    type Parents = ();
}

impl At<Id> for App {
    type Ix = Ix0;
}

impl At<Keys> for App {
    type Ix = Ix1;
}

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

#[test]
fn setup_accepts_a_built_pipeline() {
    let _pipeline = App.setup();
}

struct Logic(u8);
struct Render;
struct UsesApp;
struct UsesRender;
struct PairHost;
struct TakesKeys;

impl BeanInfo for Logic {
    type Provides = Get<Render>;
    type Injects = Get<App>;
    type Beans = (Get<App>, Get<Render>);
    type State = u8;
}

impl BeanInfo for Render {
    type Provides = ();
    type Injects = Mut<Logic>;
    type Beans = Mut<Logic>;
    type State = u8;
}

impl BeanInfo for UsesApp {
    type Provides = ();
    type Injects = Get<App>;
    type Beans = Get<App>;
    type State = ();
}

impl BeanInfo for UsesRender {
    type Provides = ();
    type Injects = Get<Render>;
    type Beans = Get<Render>;
    type State = ();
}

impl BeanInfo for PairHost {
    type Provides = (Get<Id>, Mut<Keys>);
    type Injects = ();
    type Beans = (Get<Id>, Mut<Keys>);
    type State = ();
}

impl BeanInfo for TakesKeys {
    type Provides = ();
    type Injects = Mut<Keys>;
    type Beans = Mut<Keys>;
    type State = ();
}

impl Setup for Render {
    fn setup(self) -> impl Tick<Self> {
        Render::query().init_state_default().build()
    }
}

#[test]
fn default_state_is_ready_before_update() {
    let built = Render::query().init_state_default().build();
    assert_eq!(*built.state(), 0);
}

#[test]
fn a_module_updates_with_only_its_inject() {
    let mut tick = Render.setup();
    let mut logic = Logic(4);
    tick.emit((), &mut logic);
    assert_eq!(logic.0, 4);
}

#[test]
fn a_child_injects_the_parent_surface() {
    fn parent<C: Module<Logic, OfParent>>() {}
    fn from_inject<C: Module<Logic, OfInject>>() {}
    fn from_provide<C: Module<Logic, OfProvide>>() {}
    fn second<C: Module<PairHost, OfProvide1>>() {}
    parent::<Render>();
    from_inject::<UsesApp>();
    from_provide::<UsesRender>();
    second::<TakesKeys>();
}

#[test]
fn an_event_lists_its_parents() {
    fn parents<E: EventInfo>() {}
    parents::<Update>();
}

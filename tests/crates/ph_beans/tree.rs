//! Query slice plus module injects, `Tick::init` instances, and `Tick::emit`.
//!
//! A second `init_state` and a missing `.build()` are compile-fail doctests.
//! `MUT` against an immutable provide is a compile-fail doctest on [`FromParent`].

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use core::cell::Cell;

use ph_beans::{
    At, BeanInfo, EventInfo, Get, Ix, Marker, Module, Mut, OfInject, OfParent, OfProvide, Query,
    Setup, Slot, Tick,
};

/// Beans are zero-sized. `Id::T` lends the bean, `*Id::T` its `u8` state.
#[derive(Default)]
struct Id;
#[derive(Default)]
struct Keys;
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
    type Provides = (Get<Id>, Mut<Keys>);
    type Injects = ();
    type Beans = (Get<Id>, Mut<Keys>);
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
    type Ix = Ix<0>;
}

impl At<Keys> for App {
    type Ix = Ix<1>;
}

const BOOT_KEYS: u8 = 3;
const BOOT_STATE: u8 = 7;

fn boot(args: (&u8, &mut u8)) -> u8 {
    let (id, keys) = args;
    *keys = BOOT_KEYS;
    id + *keys + BOOT_STATE - BOOT_KEYS
}

impl Setup for App {
    fn setup(self) -> impl Tick<Self> {
        App::query()
            .inject_resources(*Id::T & *Keys::MUT)
            .init_state(boot)
            .build()
    }
}

#[test]
fn setup_accepts_a_built_pipeline() {
    let _pipeline = App.setup();
}

thread_local! {
    static SEEN_KEYS: Cell<u8> = const { Cell::new(0) };
}

fn press(args: (&u8, &mut u8)) {
    let (id, keys) = args;
    *keys += id + 1;
}

fn report(args: (&u8, &mut u8)) {
    let (_, keys) = args;
    SEEN_KEYS.with(|seen| seen.set(*keys));
}

#[test]
fn maps_borrow_the_provided_states_and_keep_their_changes() {
    let tick = App::query()
        .inject_resources(*Id::T & *Keys::MUT)
        .init_state(boot)
        .map(press)
        .map(report)
        .build();
    let mut instance = Tick::<App>::init(&tick, ());
    Tick::<App>::emit(&tick, (), &mut instance, ());
    assert_eq!(SEEN_KEYS.with(Cell::get), BOOT_KEYS + 1);
    Tick::<App>::emit(&tick, (), &mut instance, ());
    assert_eq!(SEEN_KEYS.with(Cell::get), BOOT_KEYS + 2);
}

#[test]
fn init_makes_the_instance_and_keeps_its_state() {
    let built = App::query()
        .inject_resources(*Id::T & *Keys::MUT)
        .init_state(boot)
        .build();
    let instance = Tick::<App>::init(&built, ());
    assert_eq!(*instance.state(), BOOT_STATE);
}

#[test]
fn one_pipeline_runs_separate_instances() {
    let tick = App::query()
        .inject_resources(*Id::T & *Keys::MUT)
        .init_state(boot)
        .map(press)
        .map(report)
        .build();
    let mut first = Tick::<App>::init(&tick, ());
    let mut second = Tick::<App>::init(&tick, ());
    Tick::<App>::emit(&tick, (), &mut first, ());
    Tick::<App>::emit(&tick, (), &mut first, ());
    Tick::<App>::emit(&tick, (), &mut second, ());
    assert_eq!(SEEN_KEYS.with(Cell::get), BOOT_KEYS + 1);
}

struct Logic;
struct Render;
struct UsesApp;
struct UsesRender;
struct PairHost;
struct TakesKeys;

impl BeanInfo for Logic {
    type Provides = (Get<Render>,);
    type Injects = Get<App>;
    type Beans = (Get<Render>, Get<App>);
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

impl At<Logic> for Render {
    type Ix = Ix<0>;
}

impl Setup for Render {
    fn setup(self) -> impl Tick<Self> {
        Render::query().init_state(|()| Default::default()).build()
    }
}

#[test]
fn default_state_is_ready_after_init() {
    let built = Render::query().init_state(|()| Default::default()).build();
    let mut logic = Slot::new(Logic, 4);
    let instance = Tick::<Render>::init(&built, &mut logic);
    assert_eq!(*instance.state(), 0);
}

#[test]
fn a_module_updates_with_only_its_inject() {
    let tick = Render.setup();
    let mut logic = Slot::new(Logic, 4);
    let mut instance = tick.init(&mut logic);
    tick.emit((), &mut instance, &mut logic);
    assert_eq!(logic.state, 4);
}

fn grow(logic: &mut u8) {
    *logic += 1;
}

#[test]
fn a_map_borrows_the_inject_of_that_emit() {
    let tick = Render::query()
        .inject_resources(*Logic::MUT)
        .init_state(|_| Default::default())
        .map(grow)
        .build();
    let mut logic = Slot::new(Logic, 4);
    let mut instance = Tick::<Render>::init(&tick, &mut logic);
    Tick::<Render>::emit(&tick, (), &mut instance, &mut logic);
    Tick::<Render>::emit(&tick, (), &mut instance, &mut logic);
    assert_eq!(logic.state, 6);
}

fn seed(logic: &mut u8) -> u8 {
    *logic += 1;
    *logic * 2
}

#[test]
fn init_reads_the_injects_passed_to_it() {
    let tick = Render::query()
        .inject_resources(*Logic::MUT)
        .init_state(seed)
        .build();
    let mut logic = Slot::new(Logic, 4);
    let instance = Tick::<Render>::init(&tick, &mut logic);
    assert_eq!(logic.state, 5);
    assert_eq!(*instance.state(), 10);
}

thread_local! {
    static SEEN_BEAN: Cell<bool> = const { Cell::new(false) };
}

fn sees_bean(args: (&Id, &u8)) {
    let (_bean, _state) = args;
    SEEN_BEAN.with(|seen| seen.set(true));
}

#[test]
fn a_bare_marker_lends_the_bean_and_a_star_lends_its_state() {
    let tick = App::query()
        .inject_resources(Id::T & *Keys::T)
        .init_state(|_| Default::default())
        .map(sees_bean)
        .build();
    let mut instance = Tick::<App>::init(&tick, ());
    Tick::<App>::emit(&tick, (), &mut instance, ());
    assert!(SEEN_BEAN.with(Cell::get));
}

impl At<Id> for PairHost {
    type Ix = Ix<0>;
}

impl At<Keys> for PairHost {
    type Ix = Ix<1>;
}

fn sum_into_keys(args: (&u8, &mut u8)) {
    let (id, keys) = args;
    *keys += id + 1;
    SEEN_KEYS.with(|seen| seen.set(*keys));
}

// `PairHost` lists its provides under markers: `(Id::T, Keys::MUT)`.
#[test]
fn a_module_owns_the_beans_it_provides_under_markers() {
    let tick = PairHost::query()
        .inject_resources(*Id::T & *Keys::MUT)
        .init_state(|_| ())
        .map(sum_into_keys)
        .build();
    let mut instance = Tick::<PairHost>::init(&tick, ());
    Tick::<PairHost>::emit(&tick, (), &mut instance, ());
    Tick::<PairHost>::emit(&tick, (), &mut instance, ());
    assert_eq!(SEEN_KEYS.with(Cell::get), 2);
}

/// Provides one bean, as a list of one.
struct Solo;
/// Injects the bean `Solo` provides.
struct UsesId;

impl BeanInfo for Solo {
    type Provides = (Get<Id>,);
    type Injects = ();
    type Beans = (Get<Id>,);
    type State = u8;
}

impl BeanInfo for UsesId {
    type Provides = ();
    type Injects = Get<Id>;
    type Beans = Get<Id>;
    type State = ();
}

impl At<Id> for Solo {
    type Ix = Ix<0>;
}

// One spelling, `(Get<Id>,)`, serves the store and the child check.
#[test]
fn a_list_of_one_provide_is_stored_and_seen_by_a_child() {
    fn from_provide<C: Module<Solo, OfProvide>>() {}
    from_provide::<UsesId>();
    let tick = Solo::query()
        .inject_resources(*Id::T)
        .init_state(|id: &u8| *id + BOOT_STATE)
        .build();
    let instance = Tick::<Solo>::init(&tick, ());
    assert_eq!(*instance.state(), BOOT_STATE);
}

/// Injects `Keys` shared.
struct ReadsKeys;

impl BeanInfo for ReadsKeys {
    type Provides = ();
    type Injects = Get<Keys>;
    type Beans = Get<Keys>;
    type State = ();
}

// `PairHost` provides `(Id::T, Keys::MUT)`.
#[test]
fn a_child_takes_an_exclusive_provide_either_way() {
    fn head<C: Module<PairHost, OfProvide<0>>>() {}
    fn tail<C: Module<PairHost, OfProvide<1>>>() {}
    head::<UsesId>();
    tail::<ReadsKeys>();
    tail::<TakesKeys>();
}

/// Provides three beans.
struct Trio;

impl BeanInfo for Trio {
    type Provides = (Get<Id>, Mut<Keys>, Get<Render>);
    type Injects = ();
    type Beans = (Get<Id>, Mut<Keys>, Get<Render>);
    type State = ();
}

// The child check reads any slot of a provide list of any length.
#[test]
fn a_child_takes_the_third_provide_of_three() {
    fn first<C: Module<Trio, OfProvide<0>>>() {}
    fn second<C: Module<Trio, OfProvide<1>>>() {}
    fn third<C: Module<Trio, OfProvide<2>>>() {}
    first::<UsesId>();
    second::<TakesKeys>();
    third::<UsesRender>();
}

// `impl Tick` hides the instance type. The state is read through the tick.
#[test]
fn a_hidden_tick_lends_the_module_state() {
    let tick = App.setup();
    let instance = tick.init(());
    assert_eq!(*tick.state(&instance), BOOT_STATE);
}

#[test]
fn a_child_injects_the_parent_surface() {
    fn parent<C: Module<Logic, OfParent>>() {}
    fn from_inject<C: Module<Logic, OfInject>>() {}
    fn from_provide<C: Module<Logic, OfProvide>>() {}
    fn second<C: Module<PairHost, OfProvide<1>>>() {}
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

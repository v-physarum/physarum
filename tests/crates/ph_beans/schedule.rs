//! Filters, maps, and emit selection.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use core::cell::Cell;

use ph_beans::{
    At, BeanInfo, Bit, Ctx, Dense, EmitBit, EmitIx, EventBuf, EventInfo, EventReg, Gate, Get, Info,
    Init, Ix, List, MapEntry, Marker, ModuleDesc, Mut, NoState, On, Pipe, Query, Setup, Slot,
    SlotOf, Sparse, Tick,
};

thread_local! {
    static HITS: Cell<u8> = const { Cell::new(0) };
    static STEP: Cell<u8> = const { Cell::new(0) };
}

/// Registry of the emit bits used here.
struct Events;

#[derive(Default)]
struct Sched;
#[derive(Default)]
struct Update;
struct Graphics;
struct Main;
struct Keys;

impl BeanInfo for Sched {
    type Provides = (Mut<Update>,);
    type Injects = ();
    type Beans = (Mut<Update>,);
    type State = NoState;
}

impl EventReg for Sched {
    type Reg = Events;
}

// `Update` takes deferred events: its state holds the buffer.
impl BeanInfo for Update {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = EventBuf;
}

impl EventInfo for Update {
    type Parents = ();
}

impl SlotOf<Events> for Bit<0> {
    type Owner = Update;
}

impl EmitBit for Update {
    type Reg = Events;
    type Slot = Bit<0>;
}

impl BeanInfo for Graphics {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl EventInfo for Graphics {
    type Parents = Update;
}

impl SlotOf<Events> for Bit<1> {
    type Owner = Graphics;
}

impl EmitBit for Graphics {
    type Reg = Events;
    type Slot = Bit<1>;
}

impl BeanInfo for Main {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl EventInfo for Main {
    type Parents = Update;
}

impl SlotOf<Events> for Bit<2> {
    type Owner = Main;
}

impl EmitBit for Main {
    type Reg = Events;
    type Slot = Bit<2>;
}

struct Sub;

impl BeanInfo for Sub {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl EventInfo for Sub {
    type Parents = Main;
}

impl SlotOf<Events> for Bit<3> {
    type Owner = Sub;
}

impl EmitBit for Sub {
    type Reg = Events;
    type Slot = Bit<3>;
}

impl BeanInfo for Keys {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl At<Keys> for Sched {
    type Ix = Ix<0>;
}

impl At<Update> for Sched {
    type Ix = Ix<0>;
}

fn graphics_under_update<A, S, P>(_pipeline: &Init<(Graphics, Update), Ctx<Sched, A, S, P>>) {}

fn bump_graphics(_: ()) {
    HITS.with(|hits| hits.set(hits.get() + 1));
}

fn bump_main(_: ()) {
    HITS.with(|hits| hits.set(hits.get() + 10));
}

fn reset_hits() {
    HITS.with(|hits| hits.set(0));
}

fn expect_next(expect: u8) {
    STEP.with(|step| {
        assert_eq!(step.get(), expect);
        step.set(expect + 1);
    });
}

fn reset_step() {
    STEP.with(|step| step.set(0));
}

fn step_value() -> u8 {
    STEP.with(Cell::get)
}

fn map_0(_: ()) {
    expect_next(0);
}
fn map_1(_: ()) {
    expect_next(1);
}
fn map_2(_: ()) {
    expect_next(2);
}
fn map_3(_: ()) {
    expect_next(3);
}
fn map_4(_: ()) {
    expect_next(4);
}
fn map_5(_: ()) {
    expect_next(5);
}
fn map_6(_: ()) {
    expect_next(6);
}
fn map_7(_: ()) {
    expect_next(7);
}
fn map_8(_: ()) {
    expect_next(8);
}
fn map_9(_: ()) {
    expect_next(9);
}
fn map_10(_: ()) {
    expect_next(10);
}
fn map_11(_: ()) {
    expect_next(11);
}
fn map_12(_: ()) {
    expect_next(12);
}
fn map_13(_: ()) {
    expect_next(13);
}
fn map_14(_: ()) {
    expect_next(14);
}
fn map_15(_: ()) {
    expect_next(15);
}

#[test]
fn a_nested_on_replaces_the_event_spine() {
    let pipeline = Sched::query().filter(Update::ON[Graphics::ON]);
    graphics_under_update(&pipeline);
}

#[test]
fn a_wait_lets_the_next_resource_replace_the_claim() {
    let _pipeline = Sched::query()
        .inject_resources(Keys::MUT)
        .filter(Update::ON[Main::ON[Keys::MUT]])
        .inject_resources(Keys::T);
}

#[test]
fn emitted_branch_runs_and_the_other_is_skipped() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON[Graphics::ON])
        .map(bump_graphics)
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .emit(Update::ON[Main::ON])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::emit(&tick, Update::ON[Main::ON], &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
    reset_hits();
    Tick::<Sched>::emit(&tick, Update::ON[Graphics::ON], &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 1);
}

#[test]
fn nine_maps_run_across_two_groups() {
    reset_step();
    let tick = Sched::query()
        .map(map_0)
        .map(map_1)
        .map(map_2)
        .map(map_3)
        .map(map_4)
        .map(map_5)
        .map(map_6)
        .map(map_7)
        .map(map_8)
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::emit(&tick, (), &mut instance, ());
    assert_eq!(step_value(), 9);
}

#[test]
fn sixteen_maps_fit_in_the_list() {
    reset_step();
    let tick = Sched::query()
        .map(map_0)
        .map(map_1)
        .map(map_2)
        .map(map_3)
        .map(map_4)
        .map(map_5)
        .map(map_6)
        .map(map_7)
        .map(map_8)
        .map(map_9)
        .map(map_10)
        .map(map_11)
        .map(map_12)
        .map(map_13)
        .map(map_14)
        .map(map_15)
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::emit(&tick, (), &mut instance, ());
    assert_eq!(step_value(), 16);
}

#[test]
fn a_deferred_event_runs_on_the_next_emit() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON[Graphics::ON])
        .map(bump_graphics)
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .emit(Update::ON[Main::ON])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::defer(&tick, Update::ON[Main::ON], &mut instance);
    assert_eq!(HITS.with(Cell::get), 0);
    Tick::<Sched>::emit(&tick, Graphics::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 0);
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
}

#[test]
fn a_deferred_event_runs_after_the_direct_maps_and_each_map_runs_once() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON)
        .map(bump_graphics)
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .emit(Update::ON[Main::ON])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::defer(&tick, Update::ON[Main::ON], &mut instance);
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 11);
    reset_hits();
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 1);
}

// The queue keeps the order of the `defer` calls. The pipeline lists `Main`
// first; `Graphics` was deferred first, so it runs first.
#[test]
fn deferred_events_run_in_defer_order() {
    reset_step();
    let tick = Sched::query()
        .filter(Update::ON[Main::ON])
        .map(map_1)
        .filter(Update::ON[Graphics::ON])
        .map(map_0)
        .emit(Update::ON[Main::ON])
        .emit(Update::ON[Graphics::ON])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::defer(&tick, Update::ON[Graphics::ON], &mut instance);
    Tick::<Sched>::defer(&tick, Update::ON[Main::ON], &mut instance);
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(step_value(), 2);
}

// A deferred child path runs inside its parents: `Update[Main[Sub]]` runs the
// maps of `Main` and of `Sub`. `Graphics` was not deferred and is skipped.
#[test]
fn a_deferred_path_runs_the_maps_of_its_parents() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .filter(Update::ON[Main::ON[Sub::ON]])
        .map(bump_main)
        .filter(Update::ON[Graphics::ON])
        .map(bump_graphics)
        .emit(Update::ON[Main::ON[Sub::ON]])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::defer(&tick, Update::ON[Main::ON[Sub::ON]], &mut instance);
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 20);
}

// An event deferred into `Update` waits while another root is emitted.
#[test]
fn a_deferred_event_waits_for_its_own_scope() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .filter(Hi::ON)
        .map(bump_graphics)
        .emit(Update::ON[Main::ON])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::defer(&tick, Update::ON[Main::ON], &mut instance);
    Tick::<Sched>::emit(&tick, Hi::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 1);
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 11);
}

/// The pipeline type is hidden: only `Tick<Sched>` is known here.
fn hidden_with_emit() -> impl Tick<Sched> {
    Sched::query()
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .emit(Update::ON[Main::ON])
        .build()
}

fn hidden_without_emit() -> impl Tick<Sched> {
    Sched::query()
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .build()
}

// `impl Tick` hides the pipeline and its instance type. Defer still works.
#[test]
fn a_hidden_tick_takes_a_deferred_event() {
    reset_hits();
    let tick = hidden_with_emit();
    let mut instance = tick.init(());
    tick.defer(Update::ON[Main::ON], &mut instance);
    tick.emit(Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
}

fn desc_of<T: Tick<Sched>>(_tick: &T) -> ModuleDesc {
    <T as Tick<Sched>>::DESC
}

// The descriptor is a constant of the tick, readable through `impl Tick`.
#[test]
fn a_hidden_tick_still_describes_its_pipeline() {
    let desc = desc_of(&hidden_with_emit());
    assert!(desc.declares::<On<Update, On<Main, ()>>>());
    assert!(!desc.declares::<On<Update, On<Graphics, ()>>>());
    assert!(desc.reads_queue::<On<Update, ()>>());
    // An emit of the declared path itself has nothing left to defer into it.
    assert!(!desc.reads_queue::<On<Update, On<Main, ()>>>());

    // The steps are part of the same constant.
    assert_eq!(desc.systems(), 1);
    assert!(desc.listens_to::<On<Update, On<Main, ()>>>());

    let desc = desc_of(&hidden_without_emit());
    assert!(!desc.declares::<On<Update, On<Main, ()>>>());
    assert!(!desc.reads_queue::<On<Update, ()>>());
}

impl Setup for Sched {
    fn setup(self) -> impl Tick<Self> {
        hidden_with_emit()
    }
}

/// Holds the descriptor of `Sched` and its own.
#[derive(Default)]
struct Seer;

impl BeanInfo for Seer {
    type Provides = (Get<Info<Sched>>, Get<Info<Seer>>);
    type Injects = ();
    type Beans = (Get<Info<Sched>>, Get<Info<Seer>>);
    type State = NoState;
}

impl At<Info<Sched>> for Seer {
    type Ix = Ix<0>;
}

impl At<Info<Seer>> for Seer {
    type Ix = Ix<1>;
}

thread_local! {
    // Whether the pipeline of `Sched`, then of `Seer`, declares `Update[Main]`.
    static DECLARED: Cell<[bool; 2]> = const { Cell::new([false, true]) };
}

fn sees(args: (&Info<Sched>, &Info<Seer>)) {
    let (sched, own) = args;
    let declared = [
        sched.desc().declares::<On<Update, On<Main, ()>>>(),
        own.desc().declares::<On<Update, On<Main, ()>>>(),
    ];
    DECLARED.with(|seen| seen.set(declared));
}

impl Setup for Seer {
    fn setup(self) -> impl Tick<Self> {
        Seer::query()
            .inject_resources(Info::<Sched>::T & Info::<Seer>::T)
            .map(sees)
            .build()
    }
}

// The descriptor of a bean is a bean: a system claims it with `Info::<M>::T`.
// A module takes the descriptor of another module and its own.
#[test]
fn a_system_reads_the_descriptor_of_a_bean_and_its_own() {
    let tick = Seer.setup();
    let mut instance = tick.init(());
    tick.emit((), &mut instance, ());
    assert_eq!(DECLARED.with(Cell::get), [true, false]);
}

/// Injects the descriptor of `Sched`: its caller lends it.
struct Reader;

impl BeanInfo for Reader {
    type Provides = ();
    type Injects = Get<Info<Sched>>;
    type Beans = Get<Info<Sched>>;
    type State = NoState;
}

impl At<Info<Sched>> for Reader {
    type Ix = Ix<0>;
}

fn reads(sched: &Info<Sched>) {
    let declared = sched.desc().declares::<On<Update, On<Main, ()>>>();
    DECLARED.with(|seen| seen.set([declared, false]));
}

// A descriptor is injected like any bean: the caller lends its slot.
#[test]
fn a_descriptor_is_injected_from_the_caller() {
    let tick = Reader::query()
        .inject_resources(Info::<Sched>::T)
        .map(reads)
        .build();
    let lent = Slot::<Info<Sched>>::default();
    let mut instance = Tick::<Reader>::init(&tick, &lent);
    Tick::<Reader>::emit(&tick, (), &mut instance, &lent);
    assert_eq!(DECLARED.with(Cell::get), [true, false]);
}

/// Provides `Leaf`, a bean with its own pipeline, and the event `Update`.
#[derive(Default)]
struct Host;
#[derive(Default)]
struct Leaf;

impl BeanInfo for Host {
    type Provides = (Mut<Leaf>, Mut<Update>);
    type Injects = ();
    type Beans = (Mut<Leaf>, Mut<Update>);
    type State = NoState;
}

impl BeanInfo for Leaf {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl At<Leaf> for Host {
    type Ix = Ix<0>;
}

impl At<Update> for Host {
    type Ix = Ix<1>;
}

impl EventReg for Host {
    type Reg = Events;
}

impl EventReg for Leaf {
    type Reg = Events;
}

impl Setup for Leaf {
    fn setup(self) -> impl Tick<Self> {
        Leaf::query()
            .filter(Update::ON[Main::ON])
            .map(bump_main)
            .build()
    }
}

impl Setup for Host {
    fn setup(self) -> impl Tick<Self> {
        Host::query()
            .filter(Update::ON)
            .forward(Leaf::MUT)
            .filter(Graphics::ON)
            .forward(Leaf::MUT)
            .build()
    }
}

// The built pipeline reports its steps: the descriptor holds what `.map`
// and `.forward` stored, and the events their filters name.
#[test]
fn a_descriptor_reports_the_steps_of_the_pipeline() {
    // The descriptor is a constant: it is asked at build time, and the bean
    // that carries it stores nothing.
    const { assert!(Info::<Sched>::DESC.systems() == 1) };
    const { assert!(Info::<Host>::DESC.children() == 1) };
    assert_eq!(size_of::<Info<Sched>>(), 0);

    let sched = Info::<Sched>::default().desc();
    assert_eq!(
        [sched.systems(), sched.forwards(), sched.children()],
        [1, 0, 0]
    );
    assert!(sched.listens_to::<On<Update, On<Main, ()>>>());
    assert!(!sched.listens_to::<On<Graphics, ()>>());

    // Two `.forward` steps name one child: it is made once.
    let host = Info::<Host>::default().desc();
    assert_eq!(
        [host.systems(), host.forwards(), host.children()],
        [0, 2, 1]
    );
    assert!(host.listens_to::<On<Update, ()>>());
    assert!(host.listens_to::<On<Graphics, ()>>());
    assert!(!host.listens_to::<On<Update, On<Main, ()>>>());
}

// `Host` sends what it gets under `Update` on to `Leaf`. `Leaf` listens to
// `Update[Main]`: its own filter decides, for the same event.
#[test]
fn a_forward_passes_the_emitted_event_to_the_child() {
    reset_hits();
    let tick = Host::query().filter(Update::ON).forward(Leaf::MUT).build();
    let mut instance = Tick::<Host>::init(&tick, ());
    Tick::<Host>::emit(&tick, Update::ON[Main::ON], &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
    // The child gets the event and has no system under it.
    Tick::<Host>::emit(&tick, Update::ON[Graphics::ON], &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
    // The forward is under `Update`: another root does not reach the child.
    Tick::<Host>::emit(&tick, Graphics::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
}

// A deferred event starts at the parent. When it runs, each level forwards
// it to its child with a direct call, and the child tests the same path.
#[test]
fn a_deferred_event_is_forwarded_to_the_child() {
    reset_hits();
    let tick = Host::query()
        .filter(Update::ON)
        .forward(Leaf::MUT)
        .emit(Update::ON[Main::ON])
        .build();
    let mut instance = Tick::<Host>::init(&tick, ());
    Tick::<Host>::defer(&tick, Update::ON[Main::ON], &mut instance);
    assert_eq!(HITS.with(Cell::get), 0);
    // The direct `Update` reaches `Leaf` and runs nothing there. The
    // deferred `Update[Main]` then goes the same way and runs `bump_main`.
    Tick::<Host>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
    // The buffer is empty again.
    Tick::<Host>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 10);
}

#[test]
fn deferring_one_event_twice_runs_it_twice() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .emit(Update::ON[Main::ON])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::defer(&tick, Update::ON[Main::ON], &mut instance);
    Tick::<Sched>::defer(&tick, Update::ON[Main::ON], &mut instance);
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 20);
}

fn touch_update(state: &mut EventBuf) {
    assert!(state.is_empty());
    HITS.with(|hits| hits.set(hits.get() + 1));
}

#[test]
fn star_mut_lends_the_event_state_from_its_slot() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON)
        .inject_resources(*Update::MUT)
        .map(touch_update)
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::emit(&tick, Update::ON, &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 1);
}

struct Hi;

impl BeanInfo for Hi {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl EventInfo for Hi {
    type Parents = Update;
}

impl SlotOf<Events> for Bit<100> {
    type Owner = Hi;
}

impl EmitBit for Hi {
    type Reg = Events;
    type Slot = Bit<100>;
}

#[test]
fn a_high_emit_bit_still_selects_the_branch() {
    reset_hits();
    let tick = Sched::query()
        .filter(Update::ON[Hi::ON])
        .map(bump_graphics)
        .emit(Update::ON[Hi::ON])
        .build();
    let mut instance = Tick::<Sched>::init(&tick, ());
    Tick::<Sched>::emit(&tick, Update::ON[Hi::ON], &mut instance, ());
    assert_eq!(HITS.with(Cell::get), 1);
}

// Bases (examples): cold Update gate; dense Graphics expands; sparse Main needs its bit.
#[test]
fn path_gate_cold_root_skips_dense_and_sparse() {
    reset_hits();
    let tree = Gate::<Update, Sparse, _>::new(List::new((
        Gate::<Graphics, Dense, _>::new(List::new((
            MapEntry::<(), fn(())>::new(bump_graphics),
            MapEntry::<(), fn(())>::new(bump_graphics),
        ))),
        Gate::<Main, Sparse, _>::new(MapEntry::<On<Main, ()>, fn(())>::new(bump_main)),
    )));
    tree.path_plan::<()>();
    assert_eq!(HITS.with(Cell::get), 0);
}

#[test]
fn path_gate_dense_graphics_runs_all_leaves() {
    reset_hits();
    let tree = Gate::<Update, Sparse, _>::new(List::new((
        Gate::<Graphics, Dense, _>::new(List::new((
            MapEntry::<(), fn(())>::new(bump_graphics),
            MapEntry::<(), fn(())>::new(bump_graphics),
        ))),
        Gate::<Main, Sparse, _>::new(MapEntry::<On<Main, ()>, fn(())>::new(bump_main)),
    )));
    tree.path_plan::<On<Update, On<Graphics, ()>>>();
    assert_eq!(HITS.with(Cell::get), 2);
}

#[test]
fn path_gate_sparse_main_needs_its_bit() {
    reset_hits();
    let tree = Gate::<Update, Sparse, _>::new(List::new((
        Gate::<Graphics, Dense, _>::new(List::new((
            MapEntry::<(), fn(())>::new(bump_graphics),
            MapEntry::<(), fn(())>::new(bump_graphics),
        ))),
        Gate::<Main, Sparse, _>::new(MapEntry::<On<Main, ()>, fn(())>::new(bump_main)),
    )));
    tree.path_plan::<On<Update, On<Main, ()>>>();
    assert_eq!(HITS.with(Cell::get), 10);
}

#[test]
fn emit_providers_names_the_provider_event() {
    #[allow(clippy::type_complexity)]
    fn holds_update<A, S, I, L>(
        _pipeline: &Init<
            (),
            Ctx<Sched, A, S, Pipe<I, ph_beans::MapList<L, ph_beans::On<Update, ()>, ()>>>,
        >,
    ) {
    }
    let pipeline = Sched::query().emit_providers();
    holds_update(&pipeline);
}

#[test]
fn events_of_one_registry_have_distinct_bits() {
    const BITS: [u32; 4] = [Update::BIT, Graphics::BIT, Main::BIT, Hi::BIT];
    assert_eq!(BITS, [0, 1, 2, 100]);
}

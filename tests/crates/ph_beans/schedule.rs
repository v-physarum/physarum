//! Filters, maps, and emit selection.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use core::cell::Cell;

use ph_beans::{
    At, BeanInfo, Ctx, Dense, Direct, DirectFn, EmitBit, EventInfo, ExactDrive, Gate, Init, Ix0,
    List, MapEntry, Marker, On, Pipe, Query, Sparse, Tick,
};

thread_local! {
    static HITS: Cell<u8> = const { Cell::new(0) };
    static STEP: Cell<u8> = const { Cell::new(0) };
}

fn idle(_: ()) {}

struct Sched;
struct Update;
struct Graphics;
struct Main;
struct Keys;

impl BeanInfo for Sched {
    type Provides = Update;
    type Injects = ();
    type Beans = Update;
    type State = ();
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

impl EmitBit for Update {
    const BIT: u32 = 0;
}

impl BeanInfo for Graphics {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}

impl EventInfo for Graphics {
    type Parents = Update;
}

impl EmitBit for Graphics {
    const BIT: u32 = 1;
}

impl BeanInfo for Main {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}

impl EventInfo for Main {
    type Parents = Update;
}

impl EmitBit for Main {
    const BIT: u32 = 2;
}

impl BeanInfo for Keys {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}

impl At<Keys> for Sched {
    type Ix = Ix0;
}

fn graphics_under_update<A, S, P>(_pipeline: &Init<(Graphics, Update), Ctx<Sched, A, S, P>>) {}

fn bump_graphics() {
    HITS.with(|hits| hits.set(hits.get() + 1));
}

fn bump_main() {
    HITS.with(|hits| hits.set(hits.get() + 10));
}

struct EmitGraphics;

impl DirectFn<Update> for EmitGraphics {
    fn run<L: ExactDrive>(&self, go: &Direct<Update, L>) {
        go.emit(Graphics::ON);
    }
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

fn map_0() {
    expect_next(0);
}
fn map_1() {
    expect_next(1);
}
fn map_2() {
    expect_next(2);
}
fn map_3() {
    expect_next(3);
}
fn map_4() {
    expect_next(4);
}
fn map_5() {
    expect_next(5);
}
fn map_6() {
    expect_next(6);
}
fn map_7() {
    expect_next(7);
}
fn map_8() {
    expect_next(8);
}
fn map_9() {
    expect_next(9);
}
fn map_10() {
    expect_next(10);
}
fn map_11() {
    expect_next(11);
}
fn map_12() {
    expect_next(12);
}
fn map_13() {
    expect_next(13);
}
fn map_14() {
    expect_next(14);
}
fn map_15() {
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
    let mut tick = Sched::query()
        .filter(Update::ON[Graphics::ON])
        .map(bump_graphics)
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .emit(Update::ON[Main::ON])
        .init_state(idle)
        .build();
    Tick::<Sched>::emit(&mut tick, Update::ON[Main::ON], ());
    assert_eq!(HITS.with(Cell::get), 10);
    reset_hits();
    Tick::<Sched>::emit(&mut tick, Update::ON[Graphics::ON], ());
    assert_eq!(HITS.with(Cell::get), 1);
}

#[test]
fn nine_maps_run_across_two_groups() {
    reset_step();
    let mut tick = Sched::query()
        .map(map_0)
        .map(map_1)
        .map(map_2)
        .map(map_3)
        .map(map_4)
        .map(map_5)
        .map(map_6)
        .map(map_7)
        .map(map_8)
        .init_state(idle)
        .build();
    Tick::<Sched>::emit(&mut tick, (), ());
    assert_eq!(step_value(), 9);
}

#[test]
fn sixteen_maps_fit_in_the_list() {
    reset_step();
    let mut tick = Sched::query()
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
        .init_state(idle)
        .build();
    Tick::<Sched>::emit(&mut tick, (), ());
    assert_eq!(step_value(), 16);
}

#[test]
fn a_deferred_event_runs_on_the_next_emit() {
    reset_hits();
    let mut tick = Sched::query()
        .filter(Update::ON[Graphics::ON])
        .map(bump_graphics)
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .init_state(idle)
        .build();
    Tick::<Sched>::defer(&mut tick, Update::ON[Main::ON]);
    assert_eq!(HITS.with(Cell::get), 0);
    Tick::<Sched>::emit(&mut tick, Graphics::ON, ());
    assert_eq!(HITS.with(Cell::get), 0);
    Tick::<Sched>::emit(&mut tick, Update::ON, ());
    assert_eq!(HITS.with(Cell::get), 10);
}

#[test]
fn deferring_one_event_twice_runs_it_once() {
    reset_hits();
    let mut tick = Sched::query()
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .init_state(idle)
        .build();
    Tick::<Sched>::defer(&mut tick, Update::ON[Main::ON]);
    Tick::<Sched>::defer(&mut tick, Update::ON[Main::ON]);
    Tick::<Sched>::emit(&mut tick, Update::ON, ());
    assert_eq!(HITS.with(Cell::get), 10);
}

#[test]
fn a_direct_child_runs_only_that_block() {
    reset_hits();
    let mut tick = Sched::query()
        .filter(Update::ON)
        .map_at(EmitGraphics)
        .filter(Update::ON[Graphics::ON])
        .map(bump_graphics)
        .filter(Update::ON[Main::ON])
        .map(bump_main)
        .init_state(idle)
        .build();
    Tick::<Sched>::emit(&mut tick, Update::ON, ());
    assert_eq!(HITS.with(Cell::get), 1);
}

#[test]
fn star_mut_claims_the_event_state_inside_its_scope() {
    let _pipeline = Sched::query()
        .filter(Update::ON)
        .inject_resources(*Update::MUT);
}

#[test]
fn a_high_emit_bit_still_selects_the_branch() {
    struct Hi;
    impl BeanInfo for Hi {
        type Provides = ();
        type Injects = ();
        type Beans = ();
        type State = ();
    }
    impl EventInfo for Hi {
        type Parents = Update;
    }
    impl EmitBit for Hi {
        const BIT: u32 = 100;
    }
    reset_hits();
    let mut tick = Sched::query()
        .filter(Update::ON[Hi::ON])
        .map(bump_graphics)
        .emit(Update::ON[Hi::ON])
        .init_state(idle)
        .build();
    Tick::<Sched>::emit(&mut tick, Update::ON[Hi::ON], ());
    assert_eq!(HITS.with(Cell::get), 1);
}

// Bases (examples): cold Update gate; dense Graphics expands; sparse Main needs its bit.
#[test]
fn path_gate_cold_root_skips_dense_and_sparse() {
    reset_hits();
    let tree = Gate::<Update, Sparse, _>::new(List::new((
        Gate::<Graphics, Dense, _>::new(List::new((
            MapEntry::<(), fn()>::new(bump_graphics),
            MapEntry::<(), fn()>::new(bump_graphics),
        ))),
        Gate::<Main, Sparse, _>::new(MapEntry::<On<Main, ()>, fn()>::new(bump_main)),
    )));
    tree.path_plan::<()>();
    assert_eq!(HITS.with(Cell::get), 0);
}

#[test]
fn path_gate_dense_graphics_runs_all_leaves() {
    reset_hits();
    let tree = Gate::<Update, Sparse, _>::new(List::new((
        Gate::<Graphics, Dense, _>::new(List::new((
            MapEntry::<(), fn()>::new(bump_graphics),
            MapEntry::<(), fn()>::new(bump_graphics),
        ))),
        Gate::<Main, Sparse, _>::new(MapEntry::<On<Main, ()>, fn()>::new(bump_main)),
    )));
    tree.path_plan::<On<Update, On<Graphics, ()>>>();
    assert_eq!(HITS.with(Cell::get), 2);
}

#[test]
fn path_gate_sparse_main_needs_its_bit() {
    reset_hits();
    let tree = Gate::<Update, Sparse, _>::new(List::new((
        Gate::<Graphics, Dense, _>::new(List::new((
            MapEntry::<(), fn()>::new(bump_graphics),
            MapEntry::<(), fn()>::new(bump_graphics),
        ))),
        Gate::<Main, Sparse, _>::new(MapEntry::<On<Main, ()>, fn()>::new(bump_main)),
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

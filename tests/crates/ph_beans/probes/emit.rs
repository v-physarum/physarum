//! PERF-1 probe: release assembly of `Tick::emit` (`docs/performance.md`).
//!
//! `First` and `Second` hold `u32` states. Each system writes its own
//! constant, so every store in the assembly names the system that made it.

#![allow(unused)]

use ph_beans::{At, BeanInfo, Bit, EmitBit, EventBuf, EventInfo, EventReg, Get, Info, Ix, Marker, Mut, Owned, Query, Node, Setup, Slot, SlotOf, Tick, NoState};

#[derive(Default)]
pub struct First;
#[derive(Default)]
pub struct Second;
pub struct App;
#[derive(Default)]
pub struct Update;
pub struct Main;
pub struct Events;

impl BeanInfo for First {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u32;
}

impl BeanInfo for Second {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = u32;
}

// `Update` takes deferred events: its state holds the buffer.
impl BeanInfo for Update {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = EventBuf;
}

impl BeanInfo for Main {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}

impl BeanInfo for App {
    type Provides = (Mut<First>, Mut<Second>, Mut<Update>);
    type Injects = ();
    type Beans = (Mut<First>, Mut<Second>, Mut<Update>);
    type State = NoState;
}

impl At<First> for App {
    type Ix = Ix<0>;
}

impl At<Second> for App {
    type Ix = Ix<1>;
}

impl At<Update> for App {
    type Ix = Ix<2>;
}

impl EventInfo for Update {
    type Parents = ();
}

impl EventInfo for Main {
    type Parents = Update;
}

impl SlotOf<Events> for Bit<0> {
    type Owner = Update;
}

impl SlotOf<Events> for Bit<1> {
    type Owner = Main;
}

impl EmitBit for Update {
    type Reg = Events;
    type Slot = Bit<0>;
}

impl EmitBit for Main {
    type Reg = Events;
    type Slot = Bit<1>;
}

impl EventReg for App {
    type Reg = Events;
}

/// One instance of `App`.
pub type Instance = Owned<NoState, (Node<First>, Node<Second>, Node<Update>)>;

fn write_first(states: (&mut u32, &mut u32)) {
    *states.0 = 11;
}

fn write_second(states: (&mut u32, &mut u32)) {
    *states.1 = 22;
}

fn write_main(states: (&mut u32, &mut u32)) {
    *states.1 = 44;
}

/// Every map matches `Update`, so no map can run from its defer buffer.
#[unsafe(no_mangle)]
pub fn emit_direct(instance: &mut Instance) {
    let tick = App::query()
        .inject_resources(*First::MUT & *Second::MUT)
        .filter(Update::ON)
        .map(write_first)
        .map(write_second)
        .build();
    Tick::<App>::emit(&tick, Update::ON, instance, ());
}

/// `write_main` does not match `Update`: it runs only when `Main` was
/// deferred. `.emit` declares that `Main` can be.
#[unsafe(no_mangle)]
pub fn emit_with_deferred(instance: &mut Instance) {
    let tick = App::query()
        .inject_resources(*First::MUT & *Second::MUT)
        .filter(Update::ON)
        .map(write_first)
        .filter(Update::ON[Main::ON])
        .map(write_main)
        .emit(Update::ON[Main::ON])
        .build();
    Tick::<App>::emit(&tick, Update::ON, instance, ());
}

/// The same maps with no `.emit`: nothing can be deferred, so the queue is
/// not read.
#[unsafe(no_mangle)]
pub fn emit_without_declared(instance: &mut Instance) {
    let tick = App::query()
        .inject_resources(*First::MUT & *Second::MUT)
        .filter(Update::ON)
        .map(write_first)
        .filter(Update::ON[Main::ON])
        .map(write_main)
        .build();
    Tick::<App>::emit(&tick, Update::ON, instance, ());
}

/// A child with its own pipeline: it writes into the `Second` it injects.
#[derive(Default)]
pub struct Inner;
/// Provides `Inner` and the `Second` that `Inner` injects.
pub struct Shell;

impl BeanInfo for Inner {
    type Provides = ();
    type Injects = Mut<Second>;
    type Beans = Mut<Second>;
    type State = NoState;
}

impl BeanInfo for Shell {
    type Provides = (Mut<Inner>, Mut<Second>);
    type Injects = ();
    type Beans = (Mut<Inner>, Mut<Second>);
    type State = NoState;
}

impl At<Second> for Inner {
    type Ix = Ix<0>;
}

impl At<Inner> for Shell {
    type Ix = Ix<0>;
}

impl At<Second> for Shell {
    type Ix = Ix<1>;
}

impl EventReg for Inner {
    type Reg = Events;
}

impl EventReg for Shell {
    type Reg = Events;
}

fn write_inner(second: &mut u32) {
    *second = 33;
}

impl Setup for Inner {
    fn setup(self) -> impl Tick<Self> {
        Inner::query()
            .inject_resources(*Second::MUT)
            .filter(Update::ON)
            .map(write_inner)
            .build()
    }
}

/// `Shell` sends `Update` on to the `Inner` it provides.
#[unsafe(no_mangle)]
pub fn emit_into_child(instance: &mut Owned<NoState, (Node<Inner>, Node<Second>)>) {
    let tick = Shell::query()
        .filter(Update::ON)
        .forward(Inner::MUT)
        .build();
    Tick::<Shell>::emit(&tick, Update::ON, instance, ());
}

/// Provides `Second` and injects the descriptor of `Inner`.
pub struct Watch;

impl BeanInfo for Watch {
    type Provides = (Mut<Second>,);
    type Injects = Get<Info<Inner>>;
    type Beans = (Mut<Second>, Get<Info<Inner>>);
    type State = NoState;
}

impl At<Second> for Watch {
    type Ix = Ix<0>;
}

impl At<Info<Inner>> for Watch {
    type Ix = Ix<1>;
}

impl EventReg for Watch {
    type Reg = Events;
}

/// What `write_steps` adds to the number of systems of `Inner`.
const STEPS_BASE: u32 = 54;

fn write_steps(args: (&Info<Inner>, &mut u32)) {
    let (inner, second) = args;
    *second = STEPS_BASE + inner.desc().systems() as u32;
}

/// The system reads the descriptor of `Inner`: one system, so it stores 55.
#[unsafe(no_mangle)]
pub fn emit_reads_info(instance: &mut Owned<NoState, (Node<Second>,)>, info: &Slot<Info<Inner>>) {
    let tick = Watch::query()
        .inject_resources(Info::<Inner>::T & *Second::MUT)
        .filter(Update::ON)
        .map(write_steps)
        .build();
    Tick::<Watch>::emit(&tick, Update::ON, instance, info);
}

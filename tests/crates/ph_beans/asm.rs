//! Release assembly of the tick schedule.
//!
//! `ph_test_asm` compiles a probe with `rustc` and has no `--extern`, so it
//! cannot link `ph_beans` or the derive. The application probe is the
//! monomorphized `Take` / `drive` lowering the derive emits. `tick` in that
//! probe matches the hand-written tree.
//!
//! `ph_build_test::time` is not in this repository, so there is no build-time
//! probe of the derive.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ph_test_asm::{AsmCompare, AsmTester};

static TICK: AtomicU64 = AtomicU64::new(0);

const HAND: &str = r"
pub struct A(pub u32);
pub struct ChildState {
    pub b: u32,
}
pub struct ParentState {
    pub a: A,
    pub child: ChildState,
}

#[unsafe(no_mangle)]
pub fn tick(parent: &mut ParentState) {
    parent.child.b = parent.child.b.wrapping_add(parent.a.0);
    parent.a.0 = parent.a.0.wrapping_add(parent.child.b);
}
";

const APP: &str = r"
pub struct Off;
pub struct Ix0;
pub struct Ix1;

pub trait Take<I> {
    type Out;
    type Rest;
    fn take(self) -> (Self::Out, Self::Rest);
}

impl<T0, T1> Take<Ix0> for (T0, T1) {
    type Out = T0;
    type Rest = (Off, T1);
    #[inline(always)]
    fn take(self) -> (Self::Out, Self::Rest) {
        let (a, b) = self;
        (a, (Off, b))
    }
}

impl<T0, T1> Take<Ix1> for (T0, T1) {
    type Out = T1;
    type Rest = (T0, Off);
    #[inline(always)]
    fn take(self) -> (Self::Out, Self::Rest) {
        let (a, b) = self;
        (b, (a, Off))
    }
}

pub struct A(pub u32);
pub struct ChildState {
    pub b: u32,
}
pub struct ParentState {
    pub a: A,
    pub child: ChildState,
}

pub trait Tick {
    type Beans<'a>;
    fn tick(beans: Self::Beans<'_>);
}

pub struct Child;

impl Tick for Child {
    type Beans<'a> = (&'a mut A, &'a mut u32);
    fn tick(beans: Self::Beans<'_>) {
        let (a, b) = beans;
        *b = b.wrapping_add(a.0);
    }
}

pub trait ChildDrive<Injects> {
    fn drive_slot(self, injects: Injects);
}

impl<'a, 'i> ChildDrive<(&'i mut A,)> for &'a mut ChildState {
    #[inline(always)]
    fn drive_slot(self, injects: (&'i mut A,)) {
        ChildState::tick(self, injects);
    }
}

impl ChildState {
    #[inline(always)]
    pub fn tick(&mut self, injects: (&mut A,)) {
        let (a,) = injects;
        <Child as Tick>::tick((a, &mut self.b));
    }
}

pub trait At<P> {
    type Ix;
}

pub struct Parent;

impl At<A> for Parent {
    type Ix = Ix0;
}

impl At<Child> for Parent {
    type Ix = Ix1;
}

impl Child {
    #[inline(always)]
    pub fn drive<M, R>(refs: R)
    where
        M: At<Child> + At<A>,
        R: Take<<M as At<Child>>::Ix>,
        <R as Take<<M as At<Child>>::Ix>>::Rest: Take<<M as At<A>>::Ix>,
        <R as Take<<M as At<Child>>::Ix>>::Out: ChildDrive<(
            <<R as Take<<M as At<Child>>::Ix>>::Rest as Take<<M as At<A>>::Ix>>::Out,
        )>,
    {
        let (slot, rest) = <R as Take<<M as At<Child>>::Ix>>::take(refs);
        let (inject, _rest) =
            <<R as Take<<M as At<Child>>::Ix>>::Rest as Take<<M as At<A>>::Ix>>::take(rest);
        ChildDrive::drive_slot(slot, (inject,));
    }
}

#[inline(always)]
fn data_drive<M, R>(_refs: R) {}

impl ParentState {
    #[inline(always)]
    pub fn tick(&mut self) {
        data_drive::<Parent, _>((&mut self.a, &mut self.child));
        Child::drive::<Parent, _>((&mut self.a, &mut self.child));
        self.a.0 = self.a.0.wrapping_add(self.child.b);
    }
}

#[unsafe(no_mangle)]
pub fn tick(parent: &mut ParentState) {
    parent.tick();
}
";

#[test]
fn tick_matches_the_hand_written_tree() {
    let hand = write_probe("hand.rs", HAND);
    let app = write_probe("app.rs", APP);
    AsmCompare::files(&hand, &app)
        .function("tick")
        .assert_equivalent();
}

#[test]
fn tick_has_no_bounds_panic_or_alloc() {
    let app = write_probe("app.rs", APP);
    AsmTester::new(&app).function("tick").on_function(|scope| {
        scope.assert_no_bounds();
        scope.assert_no_checks();
        scope.assert_no_alloc();
        scope.assert_no_virtual_calls();
        scope.assert_no_invoke();
        scope.assert_no_branch();
    });
}

fn write_probe(name: &str, source: &str) -> PathBuf {
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ph-beans-asm-{n}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}

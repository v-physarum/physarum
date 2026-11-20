//! Provide, inject, and tick across two and three levels.
//!
//! `ph_build_test::time` is not in this repository, so these runtime tests are
//! the check for two- and three-level trees.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use ph_beans::{Bean, Tick};
use ph_beans_derive::Component;

#[derive(Component, Default)]
struct Count(u32);

#[derive(Component)]
#[inject(Count)]
struct Adder;

impl Tick for Adder {
    type Beans<'a> = AdderBeans<'a>;

    fn tick((count,): Self::Beans<'_>) {
        count.0 = 5;
    }
}

#[derive(Component)]
#[provider(Count, Adder)]
struct Shared;

impl Tick for Shared {
    type Beans<'a> = SharedBeans<'a>;

    fn tick((count, _adder): Self::Beans<'_>) {
        assert_eq!(count.0, 5);
    }
}

#[test]
fn shared_inject_is_the_parent_slot() {
    let mut root = Shared::create();
    root.tick();
    assert_eq!(root.count.0, 5);
}

#[derive(Component)]
#[provider(Count)]
struct Leaf;

impl Tick for Leaf {
    type Beans<'a> = LeafBeans<'a>;

    fn tick((count,): Self::Beans<'_>) {
        count.0 += 1;
    }
}

#[derive(Component)]
#[provider(Leaf)]
struct Mid;

impl Tick for Mid {
    type Beans<'a> = MidBeans<'a>;

    fn tick((leaf,): Self::Beans<'_>) {
        assert_eq!(leaf.count.0, 1);
    }
}

#[derive(Component)]
#[provider(Mid)]
struct Root;

impl Tick for Root {
    type Beans<'a> = RootBeans<'a>;

    fn tick((mid,): Self::Beans<'_>) {
        assert_eq!(mid.leaf.count.0, 1);
    }
}

#[test]
fn three_levels_let_the_owner_see_the_child_write() {
    let mut root = Root::create();
    root.tick();
    assert_eq!(root.mid.leaf.count.0, 1);
}

#[derive(Component)]
#[provider(Count)]
struct Unit;

impl Tick for Unit {
    type Beans<'a> = UnitBeans<'a>;

    fn tick((count,): Self::Beans<'_>) {
        count.0 += 1;
    }
}

#[derive(Component)]
#[provider(Unit)]
struct Left;

impl Tick for Left {
    type Beans<'a> = LeftBeans<'a>;

    fn tick((unit,): Self::Beans<'_>) {
        assert_eq!(unit.count.0, 1);
    }
}

#[derive(Component)]
#[provider(Unit)]
struct Right;

impl Tick for Right {
    type Beans<'a> = RightBeans<'a>;

    fn tick((unit,): Self::Beans<'_>) {
        assert_eq!(unit.count.0, 1);
    }
}

#[derive(Component)]
#[provider(Left, Right)]
struct Pair;

impl Tick for Pair {
    type Beans<'a> = PairBeans<'a>;

    fn tick((left, right): Self::Beans<'_>) {
        assert_eq!(left.unit.count.0, 1);
        assert_eq!(right.unit.count.0, 1);
    }
}

#[test]
fn two_parents_keep_two_states() {
    let mut pair = Pair::create();
    pair.tick();
    assert_eq!(pair.left.unit.count.0, 1);
    assert_eq!(pair.right.unit.count.0, 1);
}

#[derive(Component, Default)]
struct Flag(u32);

#[derive(Component)]
#[provider(Count)]
struct Worker;

impl Tick for Worker {
    type Beans<'a> = WorkerBeans<'a>;

    fn tick((count,): Self::Beans<'_>) {
        count.0 += 1;
    }
}

#[derive(Component)]
#[inject(Worker)]
#[provider(Flag)]
struct Watcher;

impl Tick for Watcher {
    type Beans<'a> = WatcherBeans<'a>;

    fn tick((worker, flag): Self::Beans<'_>) {
        assert_eq!(worker.count.0, 1);
        flag.0 = 1;
    }
}

#[derive(Component)]
#[provider(Worker, Watcher)]
struct Owner;

impl Tick for Owner {
    type Beans<'a> = OwnerBeans<'a>;

    fn tick((worker, watcher): Self::Beans<'_>) {
        assert_eq!(worker.count.0, 1);
        assert_eq!(watcher.flag.0, 1);
    }
}

#[test]
fn an_injected_module_ticks_once() {
    let mut owner = Owner::create();
    owner.tick();
    assert_eq!(owner.worker.count.0, 1);
    assert_eq!(owner.watcher.flag.0, 1);
}

#[derive(Component)]
#[inject(Count)]
#[provider(Flag)]
struct After;

impl Tick for After {
    type Beans<'a> = AfterBeans<'a>;

    fn tick((injected, owned): Self::Beans<'_>) {
        injected.0 = 9;
        owned.0 = 3;
    }
}

#[derive(Component)]
#[provider(After, Count)]
struct AfterParent;

impl Tick for AfterParent {
    type Beans<'a> = AfterParentBeans<'a>;

    fn tick((after, injected): Self::Beans<'_>) {
        assert_eq!(after.flag.0, 3);
        assert_eq!(injected.0, 9);
    }
}

#[test]
fn an_inject_after_the_child_keeps_its_index() {
    let mut parent = AfterParent::create();
    parent.tick();
    assert_eq!(parent.after.flag.0, 3);
    assert_eq!(parent.count.0, 9);
}

#[derive(Component, Default)]
struct A(u32);

#[derive(Component, Default)]
struct B(u32);

#[derive(Component, Default)]
struct C(u32);

#[derive(Component)]
#[inject(A, B, C)]
struct WideChild;

impl Tick for WideChild {
    type Beans<'a> = WideChildBeans<'a>;

    fn tick((a, b, c): Self::Beans<'_>) {
        a.0 = 1;
        b.0 = 2;
        c.0 = 3;
    }
}

#[derive(Component)]
#[provider(A, B, C, WideChild)]
struct Wide;

impl Tick for Wide {
    type Beans<'a> = WideBeans<'a>;

    fn tick((a, b, c, _child): Self::Beans<'_>) {
        assert_eq!((a.0, b.0, c.0), (1, 2, 3));
    }
}

#[test]
fn four_slots_pass_three_injects() {
    let mut wide = Wide::create();
    wide.tick();
    assert_eq!((wide.a.0, wide.b.0, wide.c.0), (1, 2, 3));
}

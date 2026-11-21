//! Provide, inject, and tick across two and three levels.
//!
//! Each test records an ordered effect trace. `ph_build_test::time` is not in
//! this repository, so these runtime tests are the check for two- and
//! three-level trees.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use ph_beans::{Bean, Tick};
use ph_beans_derive::Component;

#[derive(Component, Default)]
struct Trace(Vec<&'static str>);

#[derive(Component)]
#[inject(Trace)]
struct Adder;

impl Tick for Adder {
    type Beans<'a> = AdderBeans<'a>;

    fn tick((trace,): Self::Beans<'_>) {
        trace.0.push("adder");
    }
}

#[derive(Component)]
#[provider(Trace, Adder)]
struct Shared;

impl Tick for Shared {
    type Beans<'a> = SharedBeans<'a>;

    fn tick((trace, _adder): Self::Beans<'_>) {
        trace.0.push("shared");
    }
}

#[test]
fn shared_inject_is_the_parent_slot() {
    let mut root = Shared::create();
    root.tick();
    assert_eq!(root.trace.0, ["adder", "shared"]);
}

#[derive(Component)]
#[provider(Trace)]
struct Leaf;

impl Tick for Leaf {
    type Beans<'a> = LeafBeans<'a>;

    fn tick((trace,): Self::Beans<'_>) {
        trace.0.push("leaf");
    }
}

#[derive(Component)]
#[provider(Leaf)]
struct Mid;

impl Tick for Mid {
    type Beans<'a> = MidBeans<'a>;

    fn tick((leaf,): Self::Beans<'_>) {
        leaf.trace.0.push("mid");
    }
}

#[derive(Component)]
#[provider(Mid)]
struct Root;

impl Tick for Root {
    type Beans<'a> = RootBeans<'a>;

    fn tick((mid,): Self::Beans<'_>) {
        mid.leaf.trace.0.push("root");
    }
}

#[test]
fn three_levels_let_the_owner_see_the_child_write() {
    let mut root = Root::create();
    root.tick();
    assert_eq!(root.mid.leaf.trace.0, ["leaf", "mid", "root"]);
}

#[derive(Component)]
#[provider(Trace)]
struct Unit;

impl Tick for Unit {
    type Beans<'a> = UnitBeans<'a>;

    fn tick((trace,): Self::Beans<'_>) {
        trace.0.push("unit");
    }
}

#[derive(Component)]
#[provider(Unit)]
struct Left;

impl Tick for Left {
    type Beans<'a> = LeftBeans<'a>;

    fn tick((unit,): Self::Beans<'_>) {
        unit.trace.0.push("left");
    }
}

#[derive(Component)]
#[provider(Unit)]
struct Right;

impl Tick for Right {
    type Beans<'a> = RightBeans<'a>;

    fn tick((unit,): Self::Beans<'_>) {
        unit.trace.0.push("right");
    }
}

#[derive(Component)]
#[provider(Left, Right)]
struct Pair;

impl Tick for Pair {
    type Beans<'a> = PairBeans<'a>;

    fn tick((left, right): Self::Beans<'_>) {
        left.unit.trace.0.push("pair");
        right.unit.trace.0.push("pair");
    }
}

#[test]
fn two_parents_keep_two_states() {
    let mut pair = Pair::create();
    pair.tick();
    assert_eq!(pair.left.unit.trace.0, ["unit", "left", "pair"]);
    assert_eq!(pair.right.unit.trace.0, ["unit", "right", "pair"]);
}

#[derive(Component)]
#[provider(Trace)]
struct Worker;

impl Tick for Worker {
    type Beans<'a> = WorkerBeans<'a>;

    fn tick((trace,): Self::Beans<'_>) {
        trace.0.push("worker");
    }
}

#[derive(Component)]
#[inject(Worker)]
struct Watcher;

impl Tick for Watcher {
    type Beans<'a> = WatcherBeans<'a>;

    fn tick((worker,): Self::Beans<'_>) {
        worker.trace.0.push("watcher");
    }
}

#[derive(Component)]
#[provider(Worker, Watcher)]
struct Owner;

impl Tick for Owner {
    type Beans<'a> = OwnerBeans<'a>;

    fn tick((worker, watcher): Self::Beans<'_>) {
        let _ = watcher;
        worker.trace.0.push("owner");
    }
}

#[test]
fn an_injected_module_ticks_once() {
    let mut owner = Owner::create();
    owner.tick();
    assert_eq!(owner.worker.trace.0, ["worker", "watcher", "owner"]);
}

#[derive(Component, Default)]
struct Flag(u32);

#[derive(Component, Default)]
struct Count(u32);

#[derive(Component)]
#[inject(Count)]
#[provider(Flag, Trace)]
struct After;

impl Tick for After {
    type Beans<'a> = AfterBeans<'a>;

    fn tick((count, flag, trace): Self::Beans<'_>) {
        count.0 = 9;
        flag.0 = 3;
        trace.0.push("after");
    }
}

#[derive(Component)]
#[provider(After, Count)]
struct AfterParent;

impl Tick for AfterParent {
    type Beans<'a> = AfterParentBeans<'a>;

    fn tick((after, count): Self::Beans<'_>) {
        let step = if after.flag.0 == 3 && count.0 == 9 {
            "parent"
        } else {
            "mismatch"
        };
        after.trace.0.push(step);
    }
}

#[test]
fn an_inject_after_the_child_keeps_its_index() {
    let mut parent = AfterParent::create();
    parent.tick();
    assert_eq!(parent.after.trace.0, ["after", "parent"]);
}

#[derive(Component, Default)]
struct A(u32);

#[derive(Component, Default)]
struct B(u32);

#[derive(Component, Default)]
struct C(u32);

#[derive(Component)]
#[inject(A, B, C)]
#[provider(Trace)]
struct WideChild;

impl Tick for WideChild {
    type Beans<'a> = WideChildBeans<'a>;

    fn tick((a, b, c, trace): Self::Beans<'_>) {
        a.0 = 1;
        b.0 = 2;
        c.0 = 3;
        trace.0.extend(["a", "b", "c"]);
    }
}

#[derive(Component)]
#[provider(A, B, C, WideChild)]
struct Wide;

impl Tick for Wide {
    type Beans<'a> = WideBeans<'a>;

    fn tick((a, b, c, child): Self::Beans<'_>) {
        let step = if (a.0, b.0, c.0) == (1, 2, 3) {
            "wide"
        } else {
            "mismatch"
        };
        child.trace.0.push(step);
    }
}

#[test]
fn four_slots_pass_three_injects() {
    let mut wide = Wide::create();
    wide.tick();
    assert_eq!(wide.wide_child.trace.0, ["a", "b", "c", "wide"]);
}

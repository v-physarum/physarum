//! Dyn inject: the module's `Injects` names the trait (`Draw::T as Painter`),
//! the operator takes the ordinary claim, and the system takes `&dyn Painter`.
//! The caller lends a concrete value; the call coerces it.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ph_beans::{At, BeanInfo, Dyn, DynObject, Get, Ix, Marker, Mut, Query, Tick};
use ph_test_asm::AsmTester;

mod library;

use library::{CRATE, library};

#[derive(Default)]
struct Draw(u8);
struct App;

impl BeanInfo for Draw {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = Self;
}

impl BeanInfo for App {
    type Provides = (Mut<Draw>,);
    type Injects = ();
    type Beans = (Mut<Draw>,);
    type State = u8;
}

impl At<Draw> for App {
    type Ix = Ix<0>;
}

trait Painter {
    fn paint(&self) -> u8;
    fn tint(&mut self, by: u8);
}

impl Painter for Draw {
    fn paint(&self) -> u8 {
        self.0
    }

    fn tint(&mut self, by: u8) {
        self.0 += by;
    }
}

/// Another implementation of the same trait.
struct Sketch(u8);

impl Painter for Sketch {
    fn paint(&self) -> u8 {
        self.0 * 2
    }

    fn tint(&mut self, by: u8) {
        self.0 += 2 * by;
    }
}

/// Injects `Draw::MUT as Painter`.
struct Ui;
/// Injects `Draw::T as Painter`.
struct Viewer;

impl BeanInfo for Ui {
    type Provides = ();
    type Injects = Dyn<Mut<Draw>, dyn Painter>;
    type Beans = Dyn<Mut<Draw>, dyn Painter>;
    type State = u8;
}

impl BeanInfo for Viewer {
    type Provides = ();
    type Injects = Dyn<Get<Draw>, dyn Painter>;
    type Beans = Dyn<Get<Draw>, dyn Painter>;
    type State = u8;
}

impl At<Draw> for Ui {
    type Ix = Ix<0>;
}

impl At<Draw> for Viewer {
    type Ix = Ix<0>;
}

const TINT: u8 = 5;

fn tint(painter: &mut dyn Painter) {
    painter.tint(TINT);
}

fn start(_: &mut dyn Painter) -> u8 {
    0
}

fn read(painter: &dyn Painter) -> u8 {
    painter.paint()
}

// The operator takes the ordinary claim `Draw::MUT`. The system takes
// `&mut dyn Painter`. The caller lends a concrete `Draw`.
#[test]
fn a_dyn_inject_lends_the_trait_object_for_the_ordinary_claim() {
    let tick = Ui::query()
        .inject_resources(Draw::MUT)
        .init_state(start)
        .map(tint)
        .build();
    let mut draw = Draw(1);
    let mut ui = Tick::<Ui>::init(&tick, &mut draw);
    Tick::<Ui>::emit(&tick, (), &mut ui, &mut draw);
    assert_eq!(draw.0, 1 + TINT);
}

// The module names the trait, not the type: any implementation can be lent.
#[test]
fn a_dyn_inject_takes_any_implementation_of_the_trait() {
    let tick = Ui::query()
        .inject_resources(Draw::MUT)
        .init_state(start)
        .map(tint)
        .build();
    let mut sketch = Sketch(1);
    let mut ui = Tick::<Ui>::init(&tick, &mut sketch);
    Tick::<Ui>::emit(&tick, (), &mut ui, &mut sketch);
    assert_eq!(sketch.0, 1 + 2 * TINT);
}

// `Draw::T` and `*Draw::T` both read the lent trait object.
#[test]
fn a_shared_dyn_inject_is_read_by_the_bean_and_the_state_claim() {
    let by_bean = Viewer::query()
        .inject_resources(Draw::T)
        .init_state(read)
        .build();
    let by_state = Viewer::query()
        .inject_resources(*Draw::T)
        .init_state(read)
        .build();
    let draw = Draw(9);
    assert_eq!(*Tick::<Viewer>::init(&by_bean, &draw).state(), 9);
    assert_eq!(*Tick::<Viewer>::init(&by_state, &draw).state(), 9);
}

// The concrete type is known where the caller lends it, so the call into
// the trait is resolved at build time.
#[test]
fn a_dyn_inject_has_no_virtual_call() {
    let probe = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/crates/ph_beans/probes/dyn_inject.rs");
    AsmTester::new(probe)
        .extern_crate(CRATE, library())
        .function("emit_with_dyn_inject")
        .on_function(|body| {
            body.assert_no_virtual_calls();
            body.assert_no_alloc();
        });
}

// What the derive of a `dynamic` trait writes: the object for a borrow.
impl<'a> DynObject<'a> for dyn Painter {
    type Obj = dyn Painter + 'a;

    fn shared(&'a self) -> &'a Self::Obj {
        self
    }

    fn exclusive(&'a mut self) -> &'a mut Self::Obj {
        self
    }
}

// A plain entry (`App` provides `Draw::MUT`) lends the bean itself: the claim
// `Draw::T` is `&Draw`, and the system may view it as a trait on its own.
fn boot(draw: &Draw) -> u8 {
    let view: &dyn Painter = draw;
    view.paint()
}

fn boot_mut(draw: &mut Draw) -> u8 {
    let view: &mut dyn Painter = draw;
    view.paint()
}

#[test]
fn a_shared_inject_uses_the_ordinary_bean_claim() {
    let _built = App::query()
        .inject_resources(Draw::T)
        .init_state(boot)
        .build();
}

#[test]
fn an_exclusive_inject_uses_the_ordinary_bean_claim() {
    let _built = App::query()
        .inject_resources(Draw::MUT)
        .init_state(boot_mut)
        .build();
}

#[test]
fn a_monomorphized_dyn_view_has_no_virtual_call() {
    let path = write_probe(
        "dyn_inject_probe.rs",
        r"
        trait Painter {
            fn paint(&self) -> u32;
        }
        struct Draw(u32);
        impl Painter for Draw {
            fn paint(&self) -> u32 {
                self.0
            }
        }

        #[unsafe(no_mangle)]
        pub fn on_tick(draws: &[Draw]) -> u32 {
            let mut total = 0u32;
            let mut index = 0usize;
            while index < draws.len() {
                let view: &dyn Painter = &draws[index];
                total = total.wrapping_add(view.paint());
                index += 1;
            }
            total
        }
        ",
    );
    AsmTester::new(path).function("on_tick").hot_loop(|hot| {
        hot.assert_no_virtual_calls();
    });
}

fn write_probe(name: &str, source: &str) -> PathBuf {
    static TICK: AtomicU64 = AtomicU64::new(0);
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ph-beans-dyn-{n}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}

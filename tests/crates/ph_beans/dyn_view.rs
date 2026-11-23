//! Dyn inject: concrete bean claim; cast is attribute / Injects encoding.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ph_beans::{At, BeanInfo, DynView, Get, Ix0, Marker, Pass, Proxied, Query, Target};
use ph_test_asm::AsmTester;

struct Draw(u8);
struct App;

impl BeanInfo for Draw {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}

impl BeanInfo for App {
    type Provides = (Draw,);
    type Injects = ();
    type Beans = (Draw,);
    type State = u8;
}

impl At<Draw> for App {
    type Ix = Ix0;
}

impl Target for Draw {}
impl Proxied for Draw {
    type Proxy = Pass<Draw>;
    type Link = Get<Pass<Draw>>;
}
impl Target for App {}
impl Proxied for App {
    type Proxy = Pass<App>;
    type Link = Get<Pass<App>>;
}

trait Painter {
    fn paint(&self) -> u8;
}

impl Painter for Draw {
    fn paint(&self) -> u8 {
        self.0
    }
}

impl<T: Painter> DynView<T> for dyn Painter {}

// Prototype without attr encoding: claim is still `Draw::T` / `Draw::MUT`
// (`ArgsView` = `&Draw`). Cast inject makes only the final arg `&dyn`
// (design: like `fn process(p: &dyn Platform)` with a concrete `Unix`).
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

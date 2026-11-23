//! Inject view `&dyn Trait` on the same `Get` / `Mut` leaf.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use ph_beans::{At, BeanInfo, DynView, Ix0, Marker, Query};

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

trait Painter {
    fn paint(&self) -> u8;
}

impl Painter for Draw {
    fn paint(&self) -> u8 {
        self.0
    }
}

impl<T: Painter> DynView<T> for dyn Painter {}

fn boot(view: &(dyn Painter + 'static)) -> u8 {
    view.paint()
}

fn boot_mut(view: &mut (dyn Painter + 'static)) -> u8 {
    view.paint()
}

#[test]
fn a_shared_inject_views_the_trait_object() {
    let _built = App::query()
        .inject_resources(Draw::T.as_dyn::<dyn Painter>())
        .init_state(boot)
        .build();
}

#[test]
fn an_exclusive_inject_views_the_trait_object() {
    let _built = App::query()
        .inject_resources(Draw::MUT.as_dyn::<dyn Painter>())
        .init_state(boot_mut)
        .build();
}

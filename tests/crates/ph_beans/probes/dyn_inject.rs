//! Probe: a module injects a bean as a trait object (`Draw::MUT as Painter`).
//!
//! The caller lends a concrete `Draw`. The call into the trait has to be
//! resolved at build time: no virtual call in the release assembly.

#![allow(unused)]

use ph_beans::{At, BeanInfo, Dyn, DynObject, Ix, Marker, Mut, Owned, Query, Tick};

pub trait Painter {
    fn tint(&mut self, by: u32);
}

pub struct Draw(pub u32);

impl Painter for Draw {
    fn tint(&mut self, by: u32) {
        self.0 = self.0.wrapping_add(by);
    }
}

impl BeanInfo for Draw {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = ();
}

impl<'a> DynObject<'a> for dyn Painter {
    type Obj = dyn Painter + 'a;

    fn shared(&'a self) -> &'a Self::Obj {
        self
    }

    fn exclusive(&'a mut self) -> &'a mut Self::Obj {
        self
    }
}

pub struct Ui;

impl BeanInfo for Ui {
    type Provides = ();
    type Injects = Dyn<Mut<Draw>, dyn Painter>;
    type Beans = Dyn<Mut<Draw>, dyn Painter>;
    type State = ();
}

impl At<Draw> for Ui {
    type Ix = Ix<0>;
}

fn start(_: &mut dyn Painter) {}

fn tint(painter: &mut dyn Painter) {
    painter.tint(7);
}

/// The system takes `&mut dyn Painter`; the caller lends a `Draw`.
#[unsafe(no_mangle)]
pub fn emit_with_dyn_inject(instance: &mut Owned<(), ()>, draw: &mut Draw) {
    let tick = Ui::query()
        .inject_resources(Draw::MUT)
        .init_state(start)
        .map(tint)
        .build();
    Tick::<Ui>::emit(&tick, (), instance, draw);
}

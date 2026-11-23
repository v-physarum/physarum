//! TODO: the owner fills in the project purpose.
#![allow(dead_code, unused_imports, unused_variables)]

use crate::{
    container::{
        accessors::{Get, Marker, Mut},
        injection::BeanInfo,
    },
    query::{
        bean::{AddQuery, InitState},
        context::SetResources,
    },
    scheduler::tick::{Setup, Tick},
    util::borrow::{At, Ix0, TakeMany},
};

mod container;
mod query;
mod scheduler;
mod util;

struct Repo;

impl Default for Repo {
    fn default() -> Self {
        Self
    }
}

impl BeanInfo for Repo {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = bool;
}

impl Setup for Repo {
    fn setup(self) -> impl Tick<Self> {
        todo!()
    }
}

impl At<Repo> for App {
    type Ix = Ix0;
}

struct App;

impl Default for App {
    fn default() -> Self {
        Self
    }
}

impl BeanInfo for App {
    type Provides = (Get<Repo>, Mut<Repo>);
    type Injects = ();
    type Beans = ();
    type State = u8;
}

fn init() {
    let a = App::T;
    let x = a.create_state();
    a.tick(x, x);
}

impl Setup for App {
    fn setup(self) -> impl Tick<Self> {
        App::query()
            .set_resources(App::MUT & Repo::MUT)
            .init_state(|args| 12)
    }
}

#[doc(hidden)]
pub trait Args<A, I> {
    fn init(self, state: I) -> A;
}

impl<A0, A1, M, T> Args<(A0, A1), T> for M
where
    M: BeanInfo,
    M: At<A0> + At<A1>,
    T: TakeMany<M, (A0, A1)>,
{
    fn init(self, state: T) -> (A0, A1) {
        state.take_many()
    }
}

pub trait Descriptor {
    type Args;
    type Bean;
}

pub trait Context {
    type Add<A, C>;

    fn add<A>(operator: A) -> Self::Add<A, impl Descriptor>;
}

struct QBlob {}

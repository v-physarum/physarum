use crate::container::accessors::{Get, Mut};
use crate::{
    container::injection::{Bean, BeanInfo, Provide},
    query::{bean::QBean, context::CtxInfo},
    scheduler::systems::System,
};

pub trait Setup: Sized {
    fn setup(self) -> impl Tick<Self>;
}

pub trait Tick<M> {
    type Beans<'a>;
    type State;

    fn tick(&self, state: Self::State, beans: Self::Beans<'_>);

    fn create_state(&self) -> Self::State;
}

impl<B> Tick<B> for Get<B>
where
    B: BeanInfo + Provide<B::Provides> + Default,
{
    type Beans<'a> = B::UState;
    type State = B::UState;

    fn tick(&self, state: Self::State, beans: Self::Beans<'_>) {
        todo!()
    }

    fn create_state(&self) -> Self::State {
        todo!()
    }
}

impl<I, C, B> Tick<B> for QBean<I, C>
where
    C: CtxInfo,
    B: Bean,
    I: System,
{
    type Beans<'a> = C::Resources;
    type State = B::State;

    fn create_state(&self) -> Self::State {
        todo!()
    }

    fn tick(&self, state: Self::State, beans: Self::Beans<'_>) {
        // let args = self.init.invoke(beans);
        todo!()
    }
}

impl<M: Bean> Tick<M> for ! {
    type Beans<'a> = M::State;
    type State = M::State;

    fn tick(&self, state: Self::State, beans: Self::Beans<'_>) {
        todo!()
    }

    fn create_state(&self) -> Self::State {
        todo!()
    }
}

use std::marker::PhantomData;

use crate::query::bean::QBean;

pub trait System {
    type Args;

    fn invoke(&self, args: Self::Args);
}

pub struct InitCb<I, O, CB> {
    cb: CB,
    _marker: PhantomData<fn(I, O) -> O>,
}

impl<F, C> System for QBean<F, C>
where
    F: System,
{
    type Args = F::Args;

    fn invoke(&self, args: Self::Args) {
        todo!()
    }
}

impl<I, O, CB> System for InitCb<I, O, CB>
where
    CB: Fn(I) -> O,
{
    type Args = I;

    fn invoke(&self, args: Self::Args) {
        todo!()
    }
}

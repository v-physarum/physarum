use crate::{
    container::{
        accessors::Access,
        injection::{Bean, BeanInfo},
    },
    query::context::{Ctx, CtxInfo, Resources, SetArgs, SetResources},
    scheduler::systems::InitCb,
};

pub struct QBean<I, C> {
    pub(crate) init: I,
    pub(crate) context: C,
}

impl<I, C> SetResources for QBean<I, C>
where
    C: SetResources,
{
    type Out<R: Resources> = QBean<I, C::Out<R>>;

    fn set_resources<R>(self, resources: R) -> Self::Out<R>
    where
        R: Resources,
    {
        QBean { init: self.init, context: self.context.set_resources(resources) }
    }
}

impl<I, C> SetArgs for QBean<I, C>
where
    C: SetArgs,
{
    type Set<T> = QBean<I, C::Set<T>>;

    fn set_args<T>(self, args: T) -> Self::Set<T> {
        QBean { init: self.init, context: self.context.set_args(args) }
    }
}

impl<I, C> QBean<I, C>
where
    C: CtxInfo,
{
    pub fn with_context<T>(self, context: T) -> QBean<I, T> {
        QBean { init: self.init, context }
    }
    pub fn new_with_context(init: I, context: C) -> Self {
        QBean { init, context }
    }
}

pub trait AddQuery {
    type Context;

    fn query() -> QBean<(), Self::Context>;
}

impl<B: Bean> AddQuery for B {
    type Context = Ctx<B, ()>;

    fn query() -> QBean<(), Self::Context> {
        // QBean::new_with_context((), Ctx::new(()))
        todo!()
    }
}

pub trait InitState {
    type Args<'a>;
    type State;
    type Out<'a, H>;

    fn init_state<'a, H>(self, cb: H) -> Self::Out<'a, H>
    where
        H: Fn(Self::Args<'_>) -> Self::State;
}

impl<C: CtxInfo> InitState for QBean<(), C>
where
    C::Resources: Access,
    C: CtxInfo,
{
    type Args<'a> = <C::Resources as Access>::Res<'a>;
    type State = <C::Bean as BeanInfo>::State;
    type Out<'a, H> = QBean<InitCb<Self::Args<'a>, Self::State, H>, C>;

    fn init_state<'a, H>(self, cb: H) -> Self::Out<'a, H>
    where
        H: Fn(Self::Args<'a>) -> Self::State,
    {
        todo!()
    }
}

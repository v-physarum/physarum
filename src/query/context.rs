use std::marker::PhantomData;

use crate::container::{
    accessors::{Access, Get, Res},
    injection::BeanInfo,
};

pub struct Ctx<B, A> {
    args: A,
    _marker: PhantomData<B>,
}

impl<A, B> SetResources for Ctx<B, A> {
    type Out<R: Resources> = Ctx<B, R::Out>;

    fn set_resources<R>(self, resources: R) -> Self::Out<R>
    where
        R: Resources,
    {
        Ctx::new(resources.get_resources())
    }
}

impl<A, B> Ctx<B, A> {
    pub fn new(args: A) -> Self {
        Ctx { args, _marker: PhantomData }
    }
}

pub trait SetArgs {
    type Set<T>;

    fn set_args<T>(self, args: T) -> Self::Set<T>;
}

impl<A, B> SetArgs for Ctx<B, A> {
    type Set<T> = Ctx<B, T>;

    fn set_args<T>(self, args: T) -> Self::Set<T> {
        Ctx::new(args)
    }
}

pub trait CtxInfo {
    type Bean: BeanInfo;
    type Resources;

    fn create(self) -> Self;
}

impl<A, B> CtxInfo for Ctx<B, A>
where
    B: BeanInfo,
{
    type Bean = B;
    type Resources = A;

    fn create(self) -> Self {
        self
    }
}

pub trait Resources {
    type Out;

    fn get_resources(self) -> Self::Out;
}

impl<R: Access> Resources for R {
    type Out = R;

    fn get_resources(self) -> Self::Out {
        todo!()
    }
}

pub trait SetResources {
    type Out<R: Resources>;

    fn set_resources<R>(self, resources: R) -> Self::Out<R>
    where
        R: Resources;
}

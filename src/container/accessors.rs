use std::marker::PhantomData;

use crate::container::injection::{Bean, BeanInfo, Provide};

pub struct Get<T>(PhantomData<T>);

impl<T> Get<T> {
    pub(crate) const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Bean for Get<T>
where
    T: BeanInfo + Provide<T::Provides> + Default,
{
    type State = T::UState;
}

pub struct Mut<T>(PhantomData<T>);

impl<T> Mut<T> {
    const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Bean for Mut<T>
where
    T: BeanInfo + Provide<T::Provides>,
{
    type State = T::UState;
}

pub trait Access {
    type Res<'a>;
}

impl<T: 'static> Access for Get<T> {
    type Res<'a> = &'a T;
}

impl<T: 'static> Access for Mut<T> {
    type Res<'a> = &'a mut T;
}

pub struct Res<R>(PhantomData<R>);

impl<R> Res<R> {
    pub(crate) const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<R0> Access for Res<(R0,)>
where
    R0: Access,
{
    type Res<'a> = (R0::Res<'a>,);
}

impl<R0, R1> Access for Res<(R0, R1)>
where
    R0: Access,
    R1: Access,
{
    type Res<'a> = (R0::Res<'a>, R1::Res<'a>);
}

pub trait Marker: Sized {
    const T: Get<Self> = Get::new();
    const MUT: Mut<Self> = Mut::new();
}

impl<T: BeanInfo> Marker for T {}

use core::marker::PhantomData;

pub struct New {}

pub struct Init<S0> {
    init_system: S0,
}

pub struct On<E, S> {
    systems: S,
    _markers: PhantomData<E>,
}

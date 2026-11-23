//A bean is a component of container
pub trait Bean {
    //The state of the bean (blob state + provider's blob state)
    type State;
}

pub trait Provide<B> {
    type UState;
}

impl<B: Bean> Provide<()> for B {
    type UState = B::State;
}

impl<B0: Bean, B: Bean> Provide<(B0,)> for B {
    type UState = (B::State, B0::State);
}

impl<B0: Bean, B1: Bean, B: Bean> Provide<(B0, B1)> for B {
    type UState = (B::State, B0::State, B1::State);
}

impl<B> Bean for B
where
    B: BeanInfo,
{
    type State = B::State;
}

/// Standard bean descriptor.
///
/// Derive macros emit this for every bean. It is the usual surface for provides,
/// injects, the combined bean list, and owned state.
pub trait BeanInfo {
    /// Beans this type provides.
    type Provides;
    /// Beans this type injects from its parent.
    type Injects;
    /// Provides and injects together.
    type Beans;
    /// Owned state of this bean only.
    type State;
}

/// Extra descriptor for event types (in addition to [`BeanInfo`]).
///
/// Derive with `#[config(type = event, parent = (…))]` emits this so the event
/// graph can validate parent edges. An event may list several parents.
pub trait EventInfo: BeanInfo {
    /// Parent events of this event.
    type Parents;
}

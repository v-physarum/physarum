//! Systems stored on [`Ctx`](super::Ctx).

/// No init system has been stored.
pub struct NoSystems;

/// The init function item `H`.
pub struct WithInit<H> {
    handler: H,
}

impl<H> WithInit<H> {
    pub(crate) fn new(handler: H) -> Self {
        let stored = Self { handler };
        // The function item stays installed until a later slice runs it.
        let _ = &stored.handler;
        stored
    }
}

/// State built by [`init_state_default`](crate::Init::init_state_default).
pub struct WithDefault<S> {
    pub(crate) state: S,
}

/// Systems live on the context, not on the operators.
pub trait ContextTick {}

/// Payload a [`FreshInit::Config`] stores in the init slot.
#[doc(hidden)]
pub trait Configuration {
    /// Value placed in [`WithInit`].
    type Out;
    /// Builds that value.
    fn install(self) -> Self::Out;
}

/// The init slot is still [`NoSystems`].
///
/// A second `init_state` fails this bound. Stable Rust rejects two inherent
/// `init_state` impls that differ only by a where-clause (`E0592`), so the
/// status is named here instead of a second impl whose body is unreachable.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `ErrInitAlreadySet` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "init_state stores one system"
)]
pub trait FreshInit: ContextTick {
    /// Tick after `init_state`.
    type WithInit<H>: ContextTick;
    /// Tick after `init_state_default`.
    type WithDefault<S>: ContextTick;
    /// Tick after configuration `C`.
    type Config<C: Configuration>: ContextTick;
    /// Stores the init function item.
    fn install_init<H>(self, handler: H) -> Self::WithInit<H>;
    /// Stores the value produced by `cfg`.
    fn install_config<C: Configuration>(self, cfg: C) -> Self::Config<C>;
    /// Stores default state.
    fn install_default<S>(self, state: S) -> Self::WithDefault<S>;
}

impl<L> FreshInit for Pipe<NoSystems, L>
where
    L: ContextTick,
{
    type WithInit<H> = Pipe<WithInit<H>, L>;
    type WithDefault<S> = Pipe<WithDefault<S>, L>;
    type Config<C: Configuration> = Pipe<WithInit<C::Out>, L>;

    fn install_init<H>(self, handler: H) -> Self::WithInit<H> {
        Pipe::new(WithInit::new(handler), self.list)
    }

    fn install_config<C: Configuration>(self, cfg: C) -> Self::Config<C> {
        self.install_init(cfg.install())
    }

    fn install_default<S>(self, state: S) -> Self::WithDefault<S> {
        Pipe::new(WithDefault { state }, self.list)
    }
}

/// Reads state stored by `init_state_default`.
#[doc(hidden)]
pub trait DefaultState {
    /// Stored state.
    type State;
    /// Borrows that state.
    fn state(&self) -> &Self::State;
}

impl<L, S> DefaultState for Pipe<WithDefault<S>, L> {
    type State = S;

    fn state(&self) -> &Self::State {
        &self.init.state
    }
}

impl ContextTick for NoSystems {}

impl<H> ContextTick for WithInit<H> {}

impl<S> ContextTick for WithDefault<S> {}

impl ContextTick for () {}

/// Init slot plus the map list.
pub struct Pipe<I, L> {
    pub(crate) init: I,
    pub(crate) list: L,
}

impl<I, L> Pipe<I, L> {
    pub(crate) fn new(init: I, list: L) -> Self {
        Self { init, list }
    }
}

impl<I, L> ContextTick for Pipe<I, L>
where
    I: ContextTick,
    L: ContextTick,
{
}

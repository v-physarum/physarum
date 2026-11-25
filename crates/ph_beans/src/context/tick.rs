//! Systems stored on [`Ctx`](super::Ctx), and the instance `init` makes.

/// No init system has been stored.
pub struct NoSystems;

/// One instance of a module: its state and the states of its provided beans.
///
/// Deferred events wait in the state of their scope event
/// ([`EventBuf`](crate::EventBuf)), which is one of those provided states.
///
/// The pipeline holds only systems. [`Tick::init`](crate::Tick::init) makes
/// this value, and every [`Tick::emit`](crate::Tick::emit) takes it.
/// Generics: `S` module state, `P` provided states.
pub struct Owned<S, P> {
    pub(crate) state: S,
    pub(crate) provides: P,
}

// Generics: `S` module state, `P` provided states.
impl<S, P> Owned<S, P> {
    pub(crate) const fn new(state: S, provides: P) -> Self {
        Self { state, provides }
    }

    /// State made by [`init_state`](crate::Init::init_state).
    #[must_use]
    pub const fn state(&self) -> &S {
        &self.state
    }
}

/// Systems live on the context, not on the operators.
pub trait ContextTick {}

/// The init slot is still [`NoSystems`].
///
/// A second `init_state` fails this bound. Stable Rust rejects two inherent
/// `init_state` impls that differ only by a where-clause (`E0592`), so the
/// status is named here instead of a second impl whose body is unreachable.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<1>` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "init_state runs once (PipelineError<1>)"
)]
pub trait FreshInit: ContextTick {
    /// Tick after the init step. Generics: `I` init system.
    type Ready<I: ContextTick>: ContextTick;

    /// Stores the init system.
    fn install<I: ContextTick>(self, init: I) -> Self::Ready<I>;
}

// Generics: `L` list.
impl<L> FreshInit for Pipe<NoSystems, L>
where
    L: ContextTick,
{
    type Ready<I: ContextTick> = Pipe<I, L>;

    fn install<I: ContextTick>(self, init: I) -> Self::Ready<I> {
        Pipe::new(init, self.list)
    }
}

impl ContextTick for NoSystems {}

impl ContextTick for () {}

/// Init slot plus the map list.
/// Generics: `I` init, `L` map list.
pub struct Pipe<I, L> {
    pub(crate) init: I,
    pub(crate) list: L,
}

// Generics: `I` init system, `L` list.
impl<I, L> Pipe<I, L> {
    pub(crate) fn new(init: I, list: L) -> Self {
        Self { init, list }
    }
}

// Generics: `I` init system, `L` list.
impl<I, L> ContextTick for Pipe<I, L>
where
    I: ContextTick,
    L: ContextTick,
{
}

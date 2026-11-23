//! `Init`: record resource claims and one init system.
//!
//! Seal + [`CtxConfig`](crate::context::CtxConfig) for init live here; unpacking
//! [`Ctx`](crate::context::Ctx) stays in the seal traits.

use core::marker::PhantomData;

use crate::bean::{BeanInfo, DriveInit, ForwardInit, Proxied, ProxyInfo};
use crate::context::Context;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::FreshInit;
use crate::descriptor::Descriptor;
use crate::descriptor::NeedBuild;
use crate::descriptor::OpOk;
use crate::descriptor::OpShape;
use crate::descriptor::live::AccSet;
use crate::query::apply::ApplyPatch;
use crate::query::args::{ArgsView, Claim};

/// Pipeline before `.build()`.
///
/// `E` is the event path. `()` means no event scope yet.
#[must_use = "call inject_resources, init_state, and build"]
pub struct Init<E, C> {
    context: C,
    _markers: PhantomData<E>,
}

impl<E, C> Init<E, C> {
    pub(crate) fn new(context: C) -> Self {
        Self { context, _markers: PhantomData }
    }

    pub(crate) fn into_context(self) -> C {
        self.context
    }
}

impl<E, C> Descriptor for Init<E, C>
where
    C: Descriptor,
{
    type Status = C::Status;
    type Accessors = C::Accessors;
}

/// Stores init on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
#[doc(hidden)]
pub trait SealInit<H>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Installs `handler` and moves status to [`NeedBuild`].
    fn seal_init(self, handler: H) -> Self::Out;
}

impl<B, A, S, T, H> SealInit<H> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: FreshInit,
    B: Proxied,
    B::Proxy: ForwardInit<B, H, <B::Proxy as ProxyInfo>::Chain>,
{
    type Out = Ctx<
        B,
        A,
        NeedBuild,
        T::WithInit<<B::Proxy as ForwardInit<B, H, <B::Proxy as ProxyInfo>::Chain>>::Out>,
    >;

    fn seal_init(self, handler: H) -> Self::Out {
        let handler =
            <B::Proxy as ForwardInit<B, H, <B::Proxy as ProxyInfo>::Chain>>::forward(handler);
        Ctx::new(FreshInit::install_init(self.tick, handler))
    }
}

/// Config: store init function `H` and move status to [`NeedBuild`].
#[doc(hidden)]
pub struct SetInit<H>(pub(crate) H);

impl<C, H> CtxConfig<C> for SetInit<H>
where
    C: Context + SealInit<H>,
{
    type Out = C::Out;
    type ScopeOf<E> = E;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_init(self.0)
    }
}

/// Stores [`Default`] bean state on a concrete [`Ctx`].
///
/// `D` is [`BeanInfo::State`] of the context bean — not a type param on the
/// config (unlike [`SetInit`], which carries a real handler value).
#[doc(hidden)]
pub trait SealDefault: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Installs [`Default::default`] for the bean state and moves status to [`NeedBuild`].
    fn seal_default(self) -> Self::Out;
}

impl<B, A, S, T> SealDefault for Ctx<B, A, S, T>
where
    A: AccSet,
    T: FreshInit,
    B: Proxied,
    B::Proxy: DriveInit<B>,
    B::State: Default,
{
    type Out = Ctx<B, A, NeedBuild, T::WithDefault<B::State>>;

    fn seal_default(self) -> Self::Out {
        let state = <B::Proxy as DriveInit<B>>::drive();
        Ctx::new(FreshInit::install_default(self.tick, state))
    }
}

/// Config: store [`Default`] state of the context bean and move status to [`NeedBuild`].
#[doc(hidden)]
pub struct SetDefault;

impl<C> CtxConfig<C> for SetDefault
where
    C: Context + SealDefault,
{
    type Out = C::Out;
    type ScopeOf<E> = E;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_default()
    }
}

/// Next pipeline after `.init_state`.
pub trait InitStateStep<H> {
    /// Pipeline with the init function stored.
    type Out;
    /// Stores the function item.
    fn init_state(self, handler: H) -> Self::Out;
}

impl<E, C, H> InitStateStep<H> for Init<E, C>
where
    Self: ApplyPatch<SetInit<H>>,
    C: Context,
    C::Bean: Proxied,
    <C::Bean as Proxied>::Proxy:
        ForwardInit<C::Bean, H, <<C::Bean as Proxied>::Proxy as ProxyInfo>::Chain>,
    C::Tick: FreshInit,
    C::Args: Claim<C::Bean> + ArgsView + OpShape<Status: OpOk>,
    H: for<'a> Fn(<C::Args as ArgsView>::View<'a>) -> <C::Bean as BeanInfo>::State,
{
    type Out = <Self as ApplyPatch<SetInit<H>>>::Out;

    fn init_state(self, handler: H) -> Self::Out {
        self.apply_patch(SetInit(handler))
    }
}

/// Next pipeline after `.init_state_default`.
pub trait InitStateDefaultStep {
    /// Pipeline with `Default` state stored.
    type Out;
    /// Stores that state.
    fn init_state_default(self) -> Self::Out;
}

impl<E, C> InitStateDefaultStep for Init<E, C>
where
    C: Context,
    C::Bean: Proxied,
    <C::Bean as Proxied>::Proxy: DriveInit<C::Bean>,
    C::Args: OpShape<Status: OpOk>,
    Self: ApplyPatch<SetDefault>,
{
    type Out = <Self as ApplyPatch<SetDefault>>::Out;

    fn init_state_default(self) -> Self::Out {
        self.apply_patch(SetDefault)
    }
}

impl<E, C> Init<E, C> {
    /// Stores the init system and moves status to [`NeedBuild`](crate::NeedBuild).
    ///
    /// Pass a **function item**. The system must not capture or mutate outer
    /// state — only resource args and the returned [`BeanInfo::State`]
    /// (rule 18). Under the hood handlers stay pure `fn` (build speed and a
    /// future SIMD store).
    ///
    /// A second call fails on this line with status [`ErrInitAlreadySet`](crate::ErrInitAlreadySet).
    ///
    /// ```compile_fail
    /// use ph_beans::{BeanInfo, DefaultProxy, Get, Proxied, Query, Target};
    ///
    /// struct App;
    ///
    /// impl BeanInfo for App {
    ///     type Provides = ();
    ///     type Injects = ();
    ///     type Beans = ();
    ///     type State = u8;
    /// }
    ///
    /// impl Target for App {}
    /// impl Proxied for App {
    ///     type Proxy = DefaultProxy<App>;
    ///     type Link = Get<DefaultProxy<App>>;
    /// }
    ///
    /// fn boot(_: ()) -> u8 {
    ///     1
    /// }
    ///
    /// let _twice = App::query().init_state(boot).init_state(boot);
    /// ```
    pub fn init_state<H>(self, handler: H) -> <Self as InitStateStep<H>>::Out
    where
        Self: InitStateStep<H>,
    {
        InitStateStep::init_state(self, handler)
    }

    /// Stores [`Default`] state and moves status to [`NeedBuild`](crate::NeedBuild).
    ///
    /// The state is created here, inside the builder, not by a later `init`.
    /// A second init fails on that call with [`ErrInitAlreadySet`](crate::ErrInitAlreadySet).
    ///
    /// ```compile_fail
    /// use ph_beans::{BeanInfo, DefaultProxy, Get, Proxied, Query, Target};
    ///
    /// struct App;
    ///
    /// impl BeanInfo for App {
    ///     type Provides = ();
    ///     type Injects = ();
    ///     type Beans = ();
    ///     type State = u8;
    /// }
    ///
    /// impl Target for App {}
    /// impl Proxied for App {
    ///     type Proxy = DefaultProxy<App>;
    ///     type Link = Get<DefaultProxy<App>>;
    /// }
    ///
    /// let _twice = App::query().init_state_default().init_state_default();
    /// ```
    pub fn init_state_default(self) -> <Self as InitStateDefaultStep>::Out
    where
        Self: InitStateDefaultStep,
    {
        InitStateDefaultStep::init_state_default(self)
    }
}

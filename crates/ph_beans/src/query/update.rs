//! Public runtime call: [`Tick::emit`](crate::Tick::emit).

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::ScopeHost;
use crate::context::list::{ConstEmit, DueMask};
use crate::filter::NestOk;
use crate::filter::eval::{Collect, EmitMask};
use crate::query::args::ClaimShape;
use crate::query::built::Built;
use crate::query::schedule::Drive;

/// Runs one module for an emitted event.
///
/// Generics: `M` module ([`BeanInfo`]).
pub trait Update<M: BeanInfo>: Sized
where
    M::Injects: ClaimShape,
{
    /// Runs maps whose filters match `N`. Generics: `N` emit tree ([`Collect`]).
    fn emit<N: Collect + ScopeHost>(
        &mut self,
        event: N,
        injects: <M::Injects as ClaimShape>::View<'_>,
    );
    /// Stores `N` on its scope. The next enter of that scope runs it.
    fn defer<N: Collect + NestOk + ScopeHost>(&mut self, event: N);
}

// Generics: `E` builder event scope, `C` context, `M` module.
impl<E, C, M> Update<M> for Built<E, C>
where
    C: Context,
    C::Tick: Drive,
    M: BeanInfo,
    M::Injects: ClaimShape,
{
    fn emit<N: Collect + ScopeHost>(
        &mut self,
        event: N,
        injects: <M::Injects as ClaimShape>::View<'_>,
    ) {
        let _ = (event, injects);
        let due = if N::SCOPED {
            self.take_scope(N::BIT)
        } else {
            EmitMask::EMPTY
        };
        if due == EmitMask::EMPTY {
            self.drive(ConstEmit::<N>(PhantomData));
        } else {
            self.drive(DueMask(N::MASK.or(due)));
        }
    }

    fn defer<N: Collect + NestOk + ScopeHost>(&mut self, event: N) {
        let _ = event;
        self.defer_into(N::BIT, N::MASK);
    }
}

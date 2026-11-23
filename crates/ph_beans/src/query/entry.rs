//! `query()` — start an [`Init`] pipeline (step trait, not on [`BeanInfo`](crate::BeanInfo)).

use crate::context::StartDesc;
use crate::query::Init;

/// Starts a pipeline whose status is [`NeedInit`](crate::NeedInit).
///
/// Separate from the meta trait [`BeanInfo`](crate::BeanInfo) (rule 14).
pub trait Query: Sized {
    /// [`Init`] over the bean's [`StartDesc::Ctx`](crate::context::StartDesc).
    type Out;
    /// Event path is `()` until `.filter` installs an `On` spine.
    fn query() -> Self::Out;
}

impl<B> Query for B
where
    B: StartDesc,
{
    type Out = Init<(), B::Ctx>;

    fn query() -> Self::Out {
        Init::new(B::start_ctx())
    }
}

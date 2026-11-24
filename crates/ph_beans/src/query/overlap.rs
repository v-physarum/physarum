//! A bean listed on the module and on a proxy.
//!
//! [`Beans`](crate::BeanInfo::Beans) is the module list. Each proxy's
//! [`Provides`](crate::BeanInfo::Provides) is that proxy's list. [`LevelDesc`]
//! is that chain as one leaf. `.inject_resources` reads [`LevelDesc::OK`].
//! The same id on both sides fails const-eval: `bean is already on the proxy`.
//!
//! Empty on either side skips the id check, so a provide that lives only on
//! the proxy still resolves. Two [`At`](crate::At) impls with no repeated list
//! entry stay ambiguous; this gate is the repeated entry.

use crate::bean::{BeanInfo, Parts, Stop};
use crate::descriptor::{LevelDesc, Valid};
use crate::query::args::Host;

/// Id of one bean type.
///
/// Required when the type is an item of a non-empty module list and a
/// non-empty proxy provide list. One side empty does not ask for it.
/// Equal [`ID`](BeanTag::ID) values are the same bean.
///
/// ```
/// use ph_beans::BeanTag;
/// struct Score;
/// impl BeanTag for Score {
///     const ID: u64 = 1;
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` needs `BeanTag` so it can be compared with a proxy provide",
    note = "equal `ID` values are the same bean"
)]
pub trait BeanTag {
    /// Id of this type. Equal ids are the same bean.
    const ID: u64;
}

const CLEAR: u8 = 1;

const fn reject_if_same<A: BeanTag, B: BeanTag>() -> u8 {
    assert!(A::ID != B::ID, "bean is already on the proxy");
    CLEAR
}

/// One provide compared with every item of a module list.
trait Against<Item> {
    const OK: u8;
}

impl<Item> Against<Item> for () {
    const OK: u8 = CLEAR;
}

impl<Item: BeanTag, A: BeanTag> Against<Item> for (A,) {
    const OK: u8 = reject_if_same::<A, Item>();
}

impl<Item: BeanTag, A: BeanTag, B: BeanTag> Against<Item> for (A, B)
where
    (B,): Against<Item>,
{
    const OK: u8 = {
        let _ = reject_if_same::<A, Item>();
        let _ = <(B,) as Against<Item>>::OK;
        CLEAR
    };
}

impl<Item: BeanTag, A: BeanTag, B: BeanTag, C: BeanTag> Against<Item> for (A, B, C)
where
    (B, C): Against<Item>,
{
    const OK: u8 = {
        let _ = reject_if_same::<A, Item>();
        let _ = <(B, C) as Against<Item>>::OK;
        CLEAR
    };
}

impl<Item: BeanTag, A: BeanTag, B: BeanTag, C: BeanTag, D: BeanTag> Against<Item> for (A, B, C, D)
where
    (B, C, D): Against<Item>,
{
    const OK: u8 = {
        let _ = reject_if_same::<A, Item>();
        let _ = <(B, C, D) as Against<Item>>::OK;
        CLEAR
    };
}

/// `Beans` against one `Provides` tuple.
trait Cross<Right> {
    const OK: u8;
}

impl<Left> Cross<()> for Left {
    const OK: u8 = CLEAR;
}

impl<Left: Against<A>, A> Cross<(A,)> for Left {
    const OK: u8 = <Left as Against<A>>::OK;
}

impl<Left, A, B> Cross<(A, B)> for Left
where
    Left: Against<A> + Cross<(B,)>,
{
    const OK: u8 = {
        let _ = <Left as Against<A>>::OK;
        let _ = <Left as Cross<(B,)>>::OK;
        CLEAR
    };
}

impl<Left, A, B, C> Cross<(A, B, C)> for Left
where
    Left: Against<A> + Cross<(B, C)>,
{
    const OK: u8 = {
        let _ = <Left as Against<A>>::OK;
        let _ = <Left as Cross<(B, C)>>::OK;
        CLEAR
    };
}

impl<Left, A, B, C, D> Cross<(A, B, C, D)> for Left
where
    Left: Against<A> + Cross<(B, C, D)>,
{
    const OK: u8 = {
        let _ = <Left as Against<A>>::OK;
        let _ = <Left as Cross<(B, C, D)>>::OK;
        CLEAR
    };
}

/// Provide lists from the outer proxies, not the direct one.
trait ChainClear<Beans> {
    const OK: u8;
}

impl<Beans> ChainClear<Beans> for Stop {
    const OK: u8 = CLEAR;
}

impl<Beans, O0> ChainClear<Beans> for (O0,)
where
    O0: BeanInfo,
    Beans: Cross<O0::Provides>,
{
    const OK: u8 = <Beans as Cross<O0::Provides>>::OK;
}

impl<Beans, O0, O1> ChainClear<Beans> for (O0, O1)
where
    O0: BeanInfo,
    O1: BeanInfo,
    Beans: Cross<O0::Provides> + Cross<O1::Provides>,
{
    const OK: u8 = {
        let _ = <Beans as Cross<O0::Provides>>::OK;
        let _ = <Beans as Cross<O1::Provides>>::OK;
        CLEAR
    };
}

impl<Beans, O0, O1, O2> ChainClear<Beans> for (O0, O1, O2)
where
    O0: BeanInfo,
    O1: BeanInfo,
    O2: BeanInfo,
    Beans: Cross<O0::Provides> + Cross<O1::Provides> + Cross<O2::Provides>,
{
    const OK: u8 = {
        let _ = <Beans as Cross<O0::Provides>>::OK;
        let _ = <Beans as Cross<O1::Provides>>::OK;
        let _ = <Beans as Cross<O2::Provides>>::OK;
        CLEAR
    };
}

impl<B, U> LevelDesc for B
where
    B: Parts + Host<Proxies = U>,
    B::Proxy: Parts,
    B::Beans: Cross<<B::Proxy as BeanInfo>::Provides>,
    <B::Proxy as Parts>::Chain: ChainClear<B::Beans>,
{
    type Rows = U;
    type Status = Valid;

    const OK: u8 = {
        let _ = <B::Beans as Cross<<B::Proxy as BeanInfo>::Provides>>::OK;
        let _ = <<B::Proxy as Parts>::Chain as ChainClear<B::Beans>>::OK;
        CLEAR
    };
}

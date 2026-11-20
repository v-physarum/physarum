//! [`Take`](crate::Take) for tuple widths 1 through 4.
//!
//! The consumer below has no numeric literals. Each invocation is one slot of
//! one width. After the split, the tuple width is unchanged and the hole is
//! [`Off`](crate::Off).

use crate::{Ix0, Ix1, Ix2, Ix3, Off, Take};

macro_rules! impl_take {
    (
        before: [$($before:ident),*],
        out: $out:ident,
        after: [$($after:ident),*],
        ix: $ix:ident,
        rest: [$($rest:ident),*]
    ) => {
        #[allow(clippy::similar_names, non_snake_case)]
        impl<$($before,)* $out $(, $after)*> Take<$ix> for ($($before,)* $out, $($after,)*) {
            type Out = $out;
            type Rest = ($($rest,)*);

            #[allow(clippy::inline_always)]
            #[inline(always)]
            fn take(self) -> (Self::Out, Self::Rest) {
                let ($($before,)* $out, $($after,)*) = self;
                ($out, ($($rest,)*))
            }
        }
    };
}

impl_take!(before: [], out: T0, after: [], ix: Ix0, rest: [Off]);

impl_take!(before: [], out: T0, after: [T1], ix: Ix0, rest: [Off, T1]);
impl_take!(before: [T0], out: T1, after: [], ix: Ix1, rest: [T0, Off]);

impl_take!(before: [], out: T0, after: [T1, T2], ix: Ix0, rest: [Off, T1, T2]);
impl_take!(before: [T0], out: T1, after: [T2], ix: Ix1, rest: [T0, Off, T2]);
impl_take!(before: [T0, T1], out: T2, after: [], ix: Ix2, rest: [T0, T1, Off]);

impl_take!(before: [], out: T0, after: [T1, T2, T3], ix: Ix0, rest: [Off, T1, T2, T3]);
impl_take!(before: [T0], out: T1, after: [T2, T3], ix: Ix1, rest: [T0, Off, T2, T3]);
impl_take!(before: [T0, T1], out: T2, after: [T3], ix: Ix2, rest: [T0, T1, Off, T3]);
impl_take!(before: [T0, T1, T2], out: T3, after: [], ix: Ix3, rest: [T0, T1, T2, Off]);

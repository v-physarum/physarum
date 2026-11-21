//! Take impls for tuple widths 1 through 4.
//!
//! One [`impl_foreach`](ph_util_foreach::impl_foreach) call supplies every
//! width. [`impl_take`](impl_take) only pastes the parameters it is given.

use crate::{Ix0, Ix1, Ix2, Ix3, Off, Take};
use ph_util_foreach::impl_foreach;

macro_rules! impl_take {
    (
        args: ($($arg:ident),* $(,)?);
        ix: $ix:ident;
        out: $out:ident;
        rest: $($rest:ident),* $(,)?;
    ) => {
        #[allow(clippy::similar_names, non_snake_case)]
        impl<$($arg),*> Take<$ix> for ($($arg,)*) {
            type Out = $out;
            type Rest = ($($rest,)*);

            #[allow(clippy::inline_always)]
            #[inline(always)]
            fn take(self) -> (Self::Out, Self::Rest) {
                let ($($arg,)*) = self;
                ($out, ($($rest,)*))
            }
        }
    };
    (
        (
            args: ($($arg:ident),* $(,)?);
            ix: $ix:ident;
            out: $out:ident;
            rest: $($rest:ident),* $(,)?;
        )
        $(, $next:tt)*
    ) => {
        impl_take!(
            args: ($($arg),*);
            ix: $ix;
            out: $out;
            rest: $($rest),*;
        );
        impl_take!($($next),*);
    };
    () => {};
}

impl_foreach!(
    range = 0..4,
    body = {
        impl_take!(
            (
                args: (T[..]);
                ix: Ix + #;
                out: T + #;
                rest: T[..#] + Off[#] + T[#+1..];
            )[..=#]
        );
    }
);

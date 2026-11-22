//! Compile failure carried as a span and a reason.
//!
//! `thiserror` writes the message. `quote` turns that message into
//! `compile_error!`.

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

/// A span plus the reason it failed.
#[derive(Debug, Error)]
#[error("{kind}")]
pub(crate) struct Error {
    span: Span,
    kind: ErrorKind,
}

/// Why a compile failed. The span lives on [`Error`].
#[derive(Debug, Error)]
pub(crate) enum ErrorKind {
    #[error("expected a range")]
    ExpectedRange,
    #[error("expected `..` or `..=`")]
    ExpectedDots,
    #[error("expected a step in `(range).step_by(step)`")]
    ExpectedStep,
    #[error("expected exactly one range pattern inside `[]`")]
    OnePattern,
    #[error("unexpected token in range")]
    UnexpectedInRange,
    #[error("open outer range is unbounded; write `A..B`, `A..=B`, or `(A..B).step_by(S)`")]
    OpenOuter,
    #[error("range is too large")]
    RangeTooLarge,
    #[error("bound {value} is outside the domain {lo}..={hi}")]
    OutsideDomain { value: i128, lo: i128, hi: i128 },
    #[error("outer range cannot use the index or `end`")]
    OuterHash,
    #[error("step must be a positive integer")]
    StepNotPositive,
    #[error("step must be a positive integer literal or the index plus or minus an integer")]
    StepLastIndex,
    #[error("index overflow computing an index bound")]
    IndexOverflow,
    #[error(
        "invalid range {start}{sep}{end}",
        sep = if *inclusive { "..=" } else { ".." }
    )]
    InvalidRange {
        start: i128,
        end: i128,
        inclusive: bool,
    },
    #[error("expected an unsuffixed integer literal")]
    UnsuffixedLiteral,
    #[error("integer literal overflows `i128`")]
    LiteralOverflow,
    #[error("fan mode domain `0..=index` is empty because the index is negative")]
    FanEmpty,
    #[error("write `impl_foreach!(n in 0..10 {{ ... }})`")]
    CallForm,
    #[error("the loop variable cannot be named `end`")]
    IndexName,
    #[error("expected a `{{ ... }}` body after the range")]
    ExpectedBody,
    #[error("unexpected token after the body")]
    UnexpectedAfterBody,
    #[error("write `repeat(Name, range)`, separated by commas; `X Name[..]` is not a list")]
    NotConcat,
    #[error("write `repeat(Ident, range)` or `repeat((row), range)`")]
    RepeatForm,
    #[error("cannot paste a negative index into an identifier")]
    NegativePaste,
    #[error("write `paste(Ident, n)` to paste an index")]
    SpacedPaste,
    #[error("write `$name = paste(Ident, n);` or `$name = n;`")]
    BindingForm,
    #[error("unknown index binding")]
    UnknownBinding,
    #[error("body is too large")]
    BodyTooLarge,
}

impl Error {
    pub(crate) fn new(span: Span, kind: ErrorKind) -> Self {
        Self { span, kind }
    }

    /// The compiled body does not fit in the instruction tape.
    pub(crate) fn too_large() -> Self {
        Self::new(Span::call_site(), ErrorKind::BodyTooLarge)
    }

    pub(crate) fn to_compile_error(&self) -> TokenStream {
        let span = self.span;
        let message = self.to_string();
        quote_spanned!(span=> compile_error!(#message))
    }
}

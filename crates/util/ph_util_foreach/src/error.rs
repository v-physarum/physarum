//! Compile failure carried as a span and a reason.
//!
//! The stable `proc_macro` crate has no error type. `thiserror` writes the
//! message when the value becomes `compile_error!`.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
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
    #[error("expected a step after `::`")]
    ExpectedStep,
    #[error("expected a bound")]
    ExpectedBound,
    #[error("expected `::`")]
    ExpectedPathSep,
    #[error("expected exactly one range pattern inside `[]`")]
    OnePattern,
    #[error("unexpected token in range")]
    UnexpectedInRange,
    #[error("open outer range is unbounded; write `A..B` or `A..=B`, optionally with a `::S` step")]
    OpenOuter,
    #[error("range is too large")]
    RangeTooLarge,
    #[error("bound {value} is outside the domain {lo}..={hi}")]
    OutsideDomain { value: i128, lo: i128, hi: i128 },
    #[error("outer range cannot use `#`")]
    OuterHash,
    #[error("step must be a positive integer")]
    StepNotPositive,
    #[error("step must be a positive integer literal or `#±N`")]
    StepLastIndex,
    #[error("index overflow computing a `#` bound")]
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
    #[error("expected an integer literal after `#` `+` or `#` `-`")]
    MissingAfterSign,
    #[error("fan mode domain `0..=index` is empty because the index is negative")]
    FanEmpty,
    #[error("write `impl_foreach!(range = .., body = {{ .. }})`")]
    CallForm,
    #[error("expected a `{{ ... }}` body after the range")]
    ExpectedBody,
    #[error("unexpected token after the body")]
    UnexpectedAfterBody,
    #[error("write `+` to concatenate; `X Name[..]` is not concatenation")]
    NotConcat,
    #[error("cannot paste a negative index into an identifier")]
    NegativePaste,
    #[error("write `Name + #` to paste an index; `Name #` is not a paste")]
    SpacedPaste,
    #[error("body is too large")]
    BodyTooLarge,
}

impl Error {
    pub(crate) fn new(span: Span, kind: ErrorKind) -> Self {
        Self { span, kind }
    }

    pub(crate) fn to_compile_error(&self) -> TokenStream {
        let span = self.span;
        let message = self.to_string();
        let ident = Ident::new("compile_error", span);
        let mut bang = Punct::new('!', Spacing::Alone);
        bang.set_span(span);
        let mut literal = Literal::string(&message);
        literal.set_span(span);
        let mut group = Group::new(
            Delimiter::Parenthesis,
            TokenStream::from(TokenTree::Literal(literal)),
        );
        group.set_span(span);
        TokenStream::from_iter([
            TokenTree::Ident(ident),
            TokenTree::Punct(bang),
            TokenTree::Group(group),
        ])
    }
}

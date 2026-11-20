//! Compile failure carried as a span and a reason.
//!
//! The stable `proc_macro` crate has no error type. `thiserror` writes the
//! message when the value becomes `compile_error!`.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("expected a range")]
    ExpectedRange(Span),
    #[error("expected `..` or `..=`")]
    ExpectedDots(Span),
    #[error("expected a step after `::`")]
    ExpectedStep(Span),
    #[error("expected a bound")]
    ExpectedBound(Span),
    #[error("expected `::`")]
    ExpectedPathSep(Span),
    #[error("expected exactly one range pattern inside `[]`")]
    OnePattern(Span),
    #[error("unexpected token in range")]
    UnexpectedInRange(Span),
    #[error("open outer range is unbounded; write `A..B` or `A..=B`, optionally with a `::S` step")]
    OpenOuter(Span),
    #[error("range is too large")]
    RangeTooLarge(Span),
    #[error("bound {value} is outside the domain {lo}..={hi}")]
    OutsideDomain {
        span: Span,
        value: i128,
        lo: i128,
        hi: i128,
    },
    #[error("outer range cannot use `#`")]
    OuterHash(Span),
    #[error("step must be a positive integer")]
    StepNotPositive(Span),
    #[error("step must be a positive integer literal or `#±N`")]
    StepLastIndex(Span),
    #[error("index overflow computing a `#` bound")]
    IndexOverflow(Span),
    #[error(
        "invalid range {start}{sep}{end}",
        sep = if *inclusive { "..=" } else { ".." }
    )]
    InvalidRange {
        span: Span,
        start: i128,
        end: i128,
        inclusive: bool,
    },
    #[error("expected an unsuffixed integer literal")]
    UnsuffixedLiteral(Span),
    #[error("integer literal overflows `i128`")]
    LiteralOverflow(Span),
    #[error("expected an integer literal after `#` `+` or `#` `-`")]
    MissingAfterSign(Span),
    #[error("fan mode domain `0..=index` is empty because the index is negative")]
    FanEmpty(Span),
    #[error("write `impl_foreach!(range = .., body = {{ .. }})`")]
    CallForm(Span),
    #[error("expected a `{{ ... }}` body after the range")]
    ExpectedBody(Span),
    #[error("unexpected token after the body")]
    UnexpectedAfterBody(Span),
    #[error("write `+` to concatenate; `X Name[..]` is not concatenation")]
    NotConcat(Span),
    #[error("cannot paste a negative index into an identifier")]
    NegativePaste(Span),
    #[error("write `Name + #` to paste an index; `Name #` is not a paste")]
    SpacedPaste(Span),
    #[error("body is too large")]
    BodyTooLarge(Span),
}

impl Error {
    pub(crate) fn span(&self) -> Span {
        match *self {
            Self::OutsideDomain { span, .. }
            | Self::InvalidRange { span, .. }
            | Self::ExpectedRange(span)
            | Self::ExpectedDots(span)
            | Self::ExpectedStep(span)
            | Self::ExpectedBound(span)
            | Self::ExpectedPathSep(span)
            | Self::OnePattern(span)
            | Self::UnexpectedInRange(span)
            | Self::OpenOuter(span)
            | Self::RangeTooLarge(span)
            | Self::OuterHash(span)
            | Self::StepNotPositive(span)
            | Self::StepLastIndex(span)
            | Self::IndexOverflow(span)
            | Self::UnsuffixedLiteral(span)
            | Self::LiteralOverflow(span)
            | Self::MissingAfterSign(span)
            | Self::FanEmpty(span)
            | Self::CallForm(span)
            | Self::ExpectedBody(span)
            | Self::UnexpectedAfterBody(span)
            | Self::NotConcat(span)
            | Self::NegativePaste(span)
            | Self::SpacedPaste(span)
            | Self::BodyTooLarge(span) => span,
        }
    }

    pub(crate) fn to_compile_error(&self) -> TokenStream {
        let span = self.span();
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

//! Compile failure carried as a span and a reason.
//!
//! The stable `proc_macro` crate has no error type. The message text is written
//! here, when the value becomes `compile_error!`.

use std::fmt;

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) enum Error {
    ExpectedRange(Span),
    ExpectedDots(Span),
    ExpectedStep(Span),
    ExpectedBound(Span),
    ExpectedPathSep(Span),
    OnePattern(Span),
    UnexpectedInRange(Span),
    OpenOuter(Span),
    RangeTooLarge(Span),
    OutsideDomain {
        span: Span,
        value: i128,
        lo: i128,
        hi: i128,
    },
    OuterHash(Span),
    StepNotPositive(Span),
    StepLastIndex(Span),
    IndexOverflow(Span),
    InvalidRange {
        span: Span,
        start: i128,
        end: i128,
        inclusive: bool,
    },
    UnsuffixedLiteral(Span),
    LiteralOverflow(Span),
    MissingAfterSign(Span),
    FanEmpty(Span),
    CallForm(Span),
    ExpectedBody(Span),
    UnexpectedAfterBody(Span),
    NotConcat(Span),
    NegativePaste(Span),
    SpacedPaste(Span),
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

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedRange(_) => formatter.write_str("expected a range"),
            Self::ExpectedDots(_) => formatter.write_str("expected `..` or `..=`"),
            Self::ExpectedStep(_) => formatter.write_str("expected a step after `::`"),
            Self::ExpectedBound(_) => formatter.write_str("expected a bound"),
            Self::ExpectedPathSep(_) => formatter.write_str("expected `::`"),
            Self::OnePattern(_) => {
                formatter.write_str("expected exactly one range pattern inside `[]`")
            }
            Self::UnexpectedInRange(_) => formatter.write_str("unexpected token in range"),
            Self::OpenOuter(_) => formatter.write_str(
                "open outer range is unbounded; write `A..B` or `A..=B`, optionally with a `::S` step",
            ),
            Self::RangeTooLarge(_) => formatter.write_str("range is too large"),
            Self::OutsideDomain {
                value, lo, hi, ..
            } => write!(
                formatter,
                "bound {value} is outside the domain {lo}..={hi}"
            ),
            Self::OuterHash(_) => formatter.write_str("outer range cannot use `#`"),
            Self::StepNotPositive(_) => formatter.write_str("step must be a positive integer"),
            Self::StepLastIndex(_) => {
                formatter.write_str("step must be a positive integer literal or `#±N`")
            }
            Self::IndexOverflow(_) => {
                formatter.write_str("index overflow computing a `#` bound")
            }
            Self::InvalidRange {
                start,
                end,
                inclusive,
                ..
            } => {
                formatter.write_str("invalid range ")?;
                write!(formatter, "{start}")?;
                formatter.write_str(if *inclusive { "..=" } else { ".." })?;
                write!(formatter, "{end}")
            }
            Self::UnsuffixedLiteral(_) => {
                formatter.write_str("expected an unsuffixed integer literal")
            }
            Self::LiteralOverflow(_) => formatter.write_str("integer literal overflows `i128`"),
            Self::MissingAfterSign(_) => {
                formatter.write_str("expected an integer literal after `#` `+` or `#` `-`")
            }
            Self::FanEmpty(_) => formatter.write_str(
                "fan mode domain `0..=index` is empty because the index is negative",
            ),
            Self::CallForm(_) => {
                formatter.write_str("write `impl_foreach!(range = .., body = { .. })`")
            }
            Self::ExpectedBody(_) => {
                formatter.write_str("expected a `{ ... }` body after the range")
            }
            Self::UnexpectedAfterBody(_) => {
                formatter.write_str("unexpected token after the body")
            }
            Self::NotConcat(_) => {
                formatter.write_str("write `+` to concatenate; `X Name[..]` is not concatenation")
            }
            Self::NegativePaste(_) => {
                formatter.write_str("cannot paste a negative index into an identifier")
            }
            Self::SpacedPaste(_) => {
                formatter.write_str("write `Name + #` to paste an index; `Name #` is not a paste")
            }
            Self::BodyTooLarge(_) => formatter.write_str("body is too large"),
        }
    }
}

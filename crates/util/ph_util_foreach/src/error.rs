//! Compile error carried as a span plus a message.
//!
//! The stable `proc_macro` crate has no error type. `compile_error!` is emitted
//! as tokens so the compiler reports the message at the span.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) struct Error {
    span: Span,
    message: String,
}

impl Error {
    pub(crate) fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    pub(crate) fn to_compile_error(&self) -> TokenStream {
        let ident = Ident::new("compile_error", self.span);
        let mut bang = Punct::new('!', Spacing::Alone);
        bang.set_span(self.span);
        let mut literal = Literal::string(&self.message);
        literal.set_span(self.span);
        let mut group = Group::new(
            Delimiter::Parenthesis,
            TokenStream::from(TokenTree::Literal(literal)),
        );
        group.set_span(self.span);
        TokenStream::from_iter([
            TokenTree::Ident(ident),
            TokenTree::Punct(bang),
            TokenTree::Group(group),
        ])
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

//! Placeholder substitution for one `impl_foreach` invocation.
//!
//! The body is compiled once. Each index then only substitutes values. Pasted
//! identifiers are written into a stack buffer. Flat mode repeats the body once
//! per outer index and clips patterns to that full index set. Fan mode is
//! selected when a bracket pattern mentions `#` (including `Name[#]` and
//! `Off[#]`). Each fan copy still uses one outer index, but the domain is the
//! prefix `0..=index`.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::error::{Error, Result};
use crate::range::{
    BoundStyle, Domain, EvalCtx, Pattern, compile_outer, parse_pattern, parse_range, visit_range,
};

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let (range_tokens, body) = split_input(input)?;
    let range = parse_range(&range_tokens, BoundStyle::Outer)?;
    let outer = compile_outer(&range)?;
    if outer.indices.is_empty() {
        return Ok(TokenStream::new());
    }
    let compiled = compile_tokens(&body)?;
    let mut output = TokenStream::new();
    if compiled.fan {
        for index in outer.indices {
            let domain = Domain::fan(index, range.span)?;
            emit(
                &compiled.nodes,
                &EvalCtx {
                    index,
                    domain: &domain,
                },
                &mut output,
            )?;
        }
    } else {
        let domain = Domain::from_indices(outer.indices, outer.gap)?;
        for position in 0..domain.index_count() {
            emit(
                &compiled.nodes,
                &EvalCtx {
                    index: domain.index_at(position),
                    domain: &domain,
                },
                &mut output,
            )?;
        }
    }
    Ok(output)
}

fn split_input(input: TokenStream) -> Result<(Vec<TokenTree>, Vec<TokenTree>)> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let span = tokens.first().map_or_else(Span::call_site, TokenTree::span);
    let body_at = tokens.iter().position(
        |token| matches!(token, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace),
    );
    let Some(body_at) = body_at else {
        return Err(Error::new(
            tokens.last().map_or(span, TokenTree::span),
            "expected a `{ ... }` body after the range",
        ));
    };
    if body_at + 1 != tokens.len() {
        return Err(Error::new(
            tokens[body_at + 1].span(),
            "unexpected token after the body",
        ));
    }
    if body_at == 0
        || !matches!(&tokens[body_at - 1], TokenTree::Punct(punct) if punct.as_char() == ',')
    {
        return Err(Error::new(span, "write `impl_foreach!(range, { body })`"));
    }
    if body_at == 1 {
        return Err(Error::new(span, "expected a range"));
    }
    let TokenTree::Group(body) = &tokens[body_at] else {
        return Err(Error::new(
            span,
            "expected a `{ ... }` body after the range",
        ));
    };
    Ok((
        tokens[..body_at - 1].to_vec(),
        body.stream().into_iter().collect(),
    ))
}

struct Compiled {
    nodes: Vec<Node>,
    expands: bool,
    fan: bool,
}

enum Node {
    Copy(Vec<TokenTree>),
    Group {
        delimiter: Delimiter,
        span: Span,
        children: Vec<Node>,
    },
    Index {
        span: Span,
    },
    Paste {
        base: String,
        span: Span,
    },
    List(Vec<Piece>),
}

enum Piece {
    Ident(Ident),
    Off(Ident),
    Named {
        base: String,
        span: Span,
        pattern: Pattern,
    },
    Rows {
        template: Vec<Node>,
        pattern: Pattern,
        span: Span,
    },
}

struct Compiler {
    nodes: Vec<Node>,
    verbatim: Vec<TokenTree>,
    expands: bool,
    fan: bool,
}

impl Compiler {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            verbatim: Vec::new(),
            expands: false,
            fan: false,
        }
    }

    fn flush(&mut self) {
        if self.verbatim.is_empty() {
            return;
        }
        self.nodes
            .push(Node::Copy(std::mem::take(&mut self.verbatim)));
    }

    fn finish(mut self) -> Compiled {
        self.flush();
        Compiled {
            nodes: self.nodes,
            expands: self.expands,
            fan: self.fan,
        }
    }
}

fn compile_tokens(tokens: &[TokenTree]) -> Result<Compiled> {
    let mut compiler = Compiler::new();
    let mut index = 0;
    while index < tokens.len() {
        if let Some((pieces, next, fan)) = compile_list(tokens, index)? {
            compiler.flush();
            compiler.expands = true;
            compiler.fan |= fan;
            compiler.nodes.push(Node::List(pieces));
            index = next;
            continue;
        }
        if let Some(paste) = compile_paste(tokens, index) {
            compiler.flush();
            compiler.expands = true;
            compiler.nodes.push(paste);
            index += 3;
            continue;
        }
        reject_spaced_paste(tokens, index)?;
        if let Some((pieces, next, fan)) = compile_prefixed(tokens, index)? {
            compiler.flush();
            compiler.expands = true;
            compiler.fan |= fan;
            compiler.nodes.push(Node::List(pieces));
            index = next;
            continue;
        }
        if is_bare_hash(tokens, index) {
            compiler.flush();
            compiler.expands = true;
            compiler.nodes.push(Node::Index {
                span: hash_span(tokens, index),
            });
            index += 1;
            continue;
        }
        match &tokens[index] {
            TokenTree::Group(group) => {
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                let nested = compile_tokens(&inner)?;
                if nested.expands {
                    compiler.flush();
                    compiler.expands = true;
                    compiler.fan |= nested.fan;
                    compiler.nodes.push(Node::Group {
                        delimiter: group.delimiter(),
                        span: group.span(),
                        children: nested.nodes,
                    });
                } else {
                    compiler.verbatim.push(tokens[index].clone());
                }
            }
            other => compiler.verbatim.push(other.clone()),
        }
        index += 1;
    }
    Ok(compiler.finish())
}

struct CompiledPiece {
    piece: Piece,
    next: usize,
    fan: bool,
}

fn compile_list(tokens: &[TokenTree], start: usize) -> Result<Option<(Vec<Piece>, usize, bool)>> {
    let Some(first) = compile_piece(tokens, start)? else {
        return Ok(None);
    };
    let mut pieces = vec![first.piece];
    let mut fan = first.fan;
    let mut next = first.next;
    while is_plus(tokens, next) {
        let Some(more) = compile_piece(tokens, next + 1)? else {
            break;
        };
        fan |= more.fan;
        pieces.push(more.piece);
        next = more.next;
    }
    Ok(Some((pieces, next, fan)))
}

fn compile_prefixed(
    tokens: &[TokenTree],
    start: usize,
) -> Result<Option<(Vec<Piece>, usize, bool)>> {
    let Some(TokenTree::Ident(name)) = tokens.get(start) else {
        return Ok(None);
    };
    if is_plus(tokens, start + 1)
        && let Some(first) = compile_piece(tokens, start + 2)?
    {
        let mut pieces = vec![Piece::Ident(name.clone()), first.piece];
        let mut fan = first.fan;
        let mut next = first.next;
        while is_plus(tokens, next) {
            let Some(more) = compile_piece(tokens, next + 1)? else {
                break;
            };
            fan |= more.fan;
            pieces.push(more.piece);
            next = more.next;
        }
        return Ok(Some((pieces, next, fan)));
    }
    if is_sequence_start(tokens, start + 1)? {
        return Err(Error::new(
            name.span(),
            "write `+` to concatenate; `X Name[..]` is not concatenation",
        ));
    }
    Ok(None)
}

fn compile_piece(tokens: &[TokenTree], index: usize) -> Result<Option<CompiledPiece>> {
    match (tokens.get(index), tokens.get(index + 1)) {
        (Some(TokenTree::Ident(name)), Some(TokenTree::Group(brackets)))
            if brackets.delimiter() == Delimiter::Bracket =>
        {
            match classify(brackets)? {
                Class::Pass => Ok(None),
                Class::Pattern(pattern) => Ok(Some(CompiledPiece {
                    fan: pattern.uses_index(),
                    piece: name_piece(name, pattern),
                    next: index + 2,
                })),
            }
        }
        (Some(TokenTree::Group(paren)), Some(TokenTree::Group(brackets)))
            if paren.delimiter() == Delimiter::Parenthesis
                && brackets.delimiter() == Delimiter::Bracket =>
        {
            match classify(brackets)? {
                Class::Pass => Ok(None),
                Class::Pattern(pattern) => {
                    let inner: Vec<TokenTree> = paren.stream().into_iter().collect();
                    let compiled = compile_tokens(&inner)?;
                    Ok(Some(CompiledPiece {
                        fan: pattern.uses_index() || compiled.fan,
                        piece: Piece::Rows {
                            template: compiled.nodes,
                            pattern,
                            span: paren.span(),
                        },
                        next: index + 2,
                    }))
                }
            }
        }
        _ => Ok(None),
    }
}

fn name_piece(name: &Ident, pattern: Pattern) -> Piece {
    if matches!(pattern, Pattern::Shorthand { .. }) && name.to_string() == "Off" {
        return Piece::Off(Ident::new("Off", name.span()));
    }
    Piece::Named {
        base: ident_base(name),
        span: name.span(),
        pattern,
    }
}

fn compile_paste(tokens: &[TokenTree], index: usize) -> Option<Node> {
    let TokenTree::Ident(name) = tokens.get(index)? else {
        return None;
    };
    if !is_plus(tokens, index + 1) || !is_bare_hash(tokens, index + 2) {
        return None;
    }
    Some(Node::Paste {
        base: ident_base(name),
        span: name.span(),
    })
}

fn ident_base(name: &Ident) -> String {
    let raw = name.to_string();
    match raw.strip_prefix("r#") {
        Some(base) => base.to_string(),
        None => raw,
    }
}

fn emit(nodes: &[Node], ctx: &EvalCtx<'_>, out: &mut TokenStream) -> Result<()> {
    for node in nodes {
        match node {
            Node::Copy(tokens) => out.extend(tokens.iter().cloned()),
            Node::Group {
                delimiter,
                span,
                children,
            } => {
                let mut inner = TokenStream::new();
                emit(children, ctx, &mut inner)?;
                let mut group = Group::new(*delimiter, inner);
                group.set_span(*span);
                out.extend([TokenTree::Group(group)]);
            }
            Node::Index { span } => {
                out.extend([TokenTree::Literal(index_literal(ctx.index, *span))]);
            }
            Node::Paste { base, span } => emit_pasted(base, ctx.index, *span, out)?,
            Node::List(pieces) => emit_list(pieces, ctx, out)?,
        }
    }
    Ok(())
}

fn emit_list(pieces: &[Piece], ctx: &EvalCtx<'_>, out: &mut TokenStream) -> Result<()> {
    let mut emitted = false;
    for piece in pieces {
        match piece {
            Piece::Ident(ident) | Piece::Off(ident) => {
                push_comma(&mut emitted, out);
                out.extend([TokenTree::Ident(ident.clone())]);
            }
            Piece::Named {
                base,
                span,
                pattern,
            } => {
                let range = pattern.to_range();
                visit_range(&range, ctx, |index| {
                    push_comma(&mut emitted, out);
                    emit_pasted(base, index, *span, out)
                })?;
            }
            Piece::Rows {
                template,
                pattern,
                span,
            } => {
                let range = pattern.to_range();
                let domain = ctx.domain;
                visit_range(&range, ctx, |index| {
                    let mut inner = TokenStream::new();
                    emit(template, &EvalCtx { index, domain }, &mut inner)?;
                    push_comma(&mut emitted, out);
                    let mut group = Group::new(Delimiter::Parenthesis, inner);
                    group.set_span(*span);
                    out.extend([TokenTree::Group(group)]);
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

fn push_comma(emitted: &mut bool, out: &mut TokenStream) {
    if *emitted {
        out.extend([TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
    }
    *emitted = true;
}

const PASTE_CAP: usize = 128;

fn emit_pasted(base: &str, index: i128, span: Span, out: &mut TokenStream) -> Result<()> {
    let Ok(value) = u128::try_from(index) else {
        return Err(Error::new(
            span,
            "cannot paste a negative index into an identifier",
        ));
    };
    let mut stack = [0u8; PASTE_CAP];
    if let Some(text) = write_paste(&mut stack, base, value) {
        out.extend([TokenTree::Ident(Ident::new(text, span))]);
        return Ok(());
    }
    let mut heap = String::with_capacity(base.len().saturating_add(decimal_len(value)));
    heap.push_str(base);
    push_decimal(&mut heap, value);
    out.extend([TokenTree::Ident(Ident::new(&heap, span))]);
    Ok(())
}

fn write_paste<'a>(buf: &'a mut [u8], base: &str, value: u128) -> Option<&'a str> {
    let digits = decimal_len(value);
    let len = base.len().checked_add(digits)?;
    if len > buf.len() {
        return None;
    }
    buf[..base.len()].copy_from_slice(base.as_bytes());
    write_decimal(&mut buf[base.len()..len], value);
    std::str::from_utf8(&buf[..len]).ok()
}

fn decimal_len(value: u128) -> usize {
    if value == 0 {
        return 1;
    }
    match usize::try_from(value.ilog10()) {
        Ok(width) => width + 1,
        Err(_) => 39,
    }
}

fn write_decimal(slot: &mut [u8], mut value: u128) {
    if slot.is_empty() {
        return;
    }
    if value == 0 {
        slot[0] = b'0';
        return;
    }
    for byte in slot.iter_mut().rev() {
        *byte = digit_byte(value);
        value /= 10;
    }
}

fn push_decimal(out: &mut String, mut value: u128) {
    if value == 0 {
        out.push('0');
        return;
    }
    let mut digits = [0u8; 39];
    let mut count = 0;
    while value > 0 {
        digits[count] = digit_byte(value);
        value /= 10;
        count += 1;
    }
    for index in (0..count).rev() {
        out.push(char::from(digits[index]));
    }
}

fn digit_byte(value: u128) -> u8 {
    const DIGITS: &[u8; 10] = b"0123456789";
    let index = usize::try_from(value % 10).unwrap_or(0);
    DIGITS[index]
}

enum Class {
    Pass,
    Pattern(Pattern),
}

fn classify(group: &Group) -> Result<Class> {
    let tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    if is_exactly_hash(&tokens) {
        return Ok(Class::Pattern(Pattern::Shorthand { span: group.span() }));
    }
    match parse_pattern(&tokens)? {
        Some(mut range) => {
            range.span = group.span();
            Ok(Class::Pattern(Pattern::Range(range)))
        }
        None => Ok(Class::Pass),
    }
}

fn is_exactly_hash(tokens: &[TokenTree]) -> bool {
    match tokens {
        [TokenTree::Punct(punct)] => punct.as_char() == '#',
        _ => false,
    }
}

fn is_sequence_start(tokens: &[TokenTree], index: usize) -> Result<bool> {
    match (tokens.get(index), tokens.get(index + 1)) {
        (Some(TokenTree::Ident(_)), Some(TokenTree::Group(brackets)))
            if brackets.delimiter() == Delimiter::Bracket =>
        {
            Ok(matches!(classify(brackets)?, Class::Pattern(_)))
        }
        (Some(TokenTree::Group(paren)), Some(TokenTree::Group(brackets)))
            if paren.delimiter() == Delimiter::Parenthesis
                && brackets.delimiter() == Delimiter::Bracket =>
        {
            Ok(matches!(classify(brackets)?, Class::Pattern(_)))
        }
        _ => Ok(false),
    }
}

fn reject_spaced_paste(tokens: &[TokenTree], index: usize) -> Result<()> {
    if !matches!(tokens.get(index), Some(TokenTree::Ident(_))) || !is_bare_hash(tokens, index + 1) {
        return Ok(());
    }
    Err(Error::new(
        hash_span(tokens, index + 1),
        "write `Name + #` to paste an index; `Name #` is not a paste",
    ))
}

fn is_bare_hash(tokens: &[TokenTree], index: usize) -> bool {
    is_hash(tokens, index) && !hash_is_attribute(tokens, index)
}

fn is_hash(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == '#')
}

fn hash_is_attribute(tokens: &[TokenTree], hash_index: usize) -> bool {
    match tokens.get(hash_index + 1) {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket => true,
        Some(TokenTree::Punct(punct)) if punct.as_char() == '!' => true,
        _ => false,
    }
}

fn hash_span(tokens: &[TokenTree], index: usize) -> Span {
    tokens
        .get(index)
        .map_or_else(Span::call_site, TokenTree::span)
}

fn index_literal(index: i128, span: Span) -> Literal {
    let mut literal = Literal::i128_unsuffixed(index);
    literal.set_span(span);
    literal
}

fn is_plus(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == '+')
}

#[cfg(test)]
mod paste_buffer {
    use super::{decimal_len, write_paste};

    #[test]
    fn stack_buffer_holds_the_suffix() {
        let mut buf = [0u8; 8];
        assert_eq!(write_paste(&mut buf, "Ix", 0), Some("Ix0"));
        assert_eq!(write_paste(&mut buf, "Ix", 42), Some("Ix42"));
        assert_eq!(write_paste(&mut buf, "N", 1000), Some("N1000"));
        assert_eq!(decimal_len(0), 1);
        assert_eq!(decimal_len(9), 1);
        assert_eq!(decimal_len(10), 2);
        assert_eq!(write_paste(&mut buf, "abcdefgh", 1), None);
    }
}

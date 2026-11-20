//! Placeholder substitution for one `impl_foreach` invocation.
//!
//! Flat mode repeats the body once per outer index and clips patterns to that
//! full index set. Fan mode is selected when a bracket pattern mentions `#`
//! (including `Name[#]` and `Off[#]`). Each fan copy still uses one outer
//! index, but the domain is the prefix `0..=index`.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::error::{Error, Result};

use crate::range::{
    BoundStyle, Domain, EvalCtx, Pattern, compile_outer, parse_range, resolve_range,
    starts_like_range,
};

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let (range_tokens, body) = split_input(input)?;
    let range = parse_range(&range_tokens, BoundStyle::Outer)?;
    let outer = compile_outer(&range)?;
    if outer.indices.is_empty() {
        return Ok(TokenStream::new());
    }
    let mode = if uses_fan(&body)? {
        Mode::Fan
    } else {
        Mode::Flat(Domain::from_indices(&outer.indices, outer.gap)?)
    };
    let mut output = TokenStream::new();
    for index in outer.indices {
        match &mode {
            Mode::Fan => {
                let domain = Domain::fan(index, range.span)?;
                output.extend(expand_tokens(
                    &body,
                    &EvalCtx {
                        index,
                        domain: &domain,
                    },
                )?);
            }
            Mode::Flat(domain) => {
                output.extend(expand_tokens(&body, &EvalCtx { index, domain })?);
            }
        }
    }
    Ok(output)
}

enum Mode {
    Flat(Domain),
    Fan,
}

fn split_input(input: TokenStream) -> Result<(Vec<TokenTree>, Vec<TokenTree>)> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let span = tokens.first().map_or_else(Span::call_site, TokenTree::span);
    match tokens.as_slice() {
        [TokenTree::Group(range), TokenTree::Group(body)]
            if range.delimiter() == Delimiter::Parenthesis
                && body.delimiter() == Delimiter::Brace =>
        {
            let range_tokens: Vec<TokenTree> = range.stream().into_iter().collect();
            if range_tokens.is_empty() {
                return Err(Error::new(range.span(), "expected a range"));
            }
            Ok((range_tokens, body.stream().into_iter().collect()))
        }
        [TokenTree::Group(range)] if range.delimiter() == Delimiter::Parenthesis => Err(
            Error::new(range.span(), "expected a `{ ... }` body after the range"),
        ),
        [TokenTree::Group(range), _, extra, ..] if range.delimiter() == Delimiter::Parenthesis => {
            Err(Error::new(extra.span(), "unexpected token after the body"))
        }
        _ => Err(Error::new(
            span,
            "write `impl_foreach! { (range) { body } }`",
        )),
    }
}

fn uses_fan(tokens: &[TokenTree]) -> Result<bool> {
    let mut index = 0;
    while index < tokens.len() {
        if let Some(found) = classified_at(tokens, index)? {
            if found.pattern.uses_index() {
                return Ok(true);
            }
            if let Some(template) = found.template
                && uses_fan(&template)?
            {
                return Ok(true);
            }
            index = found.next;
            continue;
        }
        if let TokenTree::Group(group) = &tokens[index] {
            let inner: Vec<TokenTree> = group.stream().into_iter().collect();
            if uses_fan(&inner)? {
                return Ok(true);
            }
        }
        index += 1;
    }
    Ok(false)
}

struct Found {
    pattern: Pattern,
    next: usize,
    template: Option<Vec<TokenTree>>,
}

fn classified_at(tokens: &[TokenTree], index: usize) -> Result<Option<Found>> {
    match (tokens.get(index), tokens.get(index + 1)) {
        (Some(TokenTree::Ident(_)), Some(TokenTree::Group(brackets)))
            if brackets.delimiter() == Delimiter::Bracket =>
        {
            match classify(brackets)? {
                Class::Pass => Ok(None),
                Class::Pattern(pattern) => Ok(Some(Found {
                    pattern,
                    next: index + 2,
                    template: None,
                })),
            }
        }
        (Some(TokenTree::Group(paren)), Some(TokenTree::Group(brackets)))
            if paren.delimiter() == Delimiter::Parenthesis
                && brackets.delimiter() == Delimiter::Bracket =>
        {
            match classify(brackets)? {
                Class::Pass => Ok(None),
                Class::Pattern(pattern) => Ok(Some(Found {
                    pattern,
                    next: index + 2,
                    template: Some(paren.stream().into_iter().collect()),
                })),
            }
        }
        _ => Ok(None),
    }
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
    if !starts_like_range(&tokens)? {
        return Ok(Class::Pass);
    }
    let mut range = parse_range(&tokens, BoundStyle::Pattern)?;
    range.span = group.span();
    Ok(Class::Pattern(Pattern::Range(range)))
}

fn is_exactly_hash(tokens: &[TokenTree]) -> bool {
    match tokens {
        [TokenTree::Punct(punct)] => punct.as_char() == '#',
        _ => false,
    }
}

fn expand_tokens(tokens: &[TokenTree], ctx: &EvalCtx<'_>) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    let mut index = 0;
    while index < tokens.len() {
        if let Some((stream, next)) = try_chain(tokens, index, ctx)? {
            output.extend(stream);
            index = next;
            continue;
        }
        if let Some(ident) = try_paste(tokens, index, ctx)? {
            output.extend([TokenTree::Ident(ident)]);
            index += 3;
            continue;
        }
        reject_spaced_paste(tokens, index)?;
        if let Some((stream, next)) = try_prefixed(tokens, index, ctx)? {
            output.extend(stream);
            index = next;
            continue;
        }
        if is_bare_hash(tokens, index) {
            output.extend([TokenTree::Literal(index_literal(
                ctx.index,
                hash_span(tokens, index),
            ))]);
            index += 1;
            continue;
        }
        match &tokens[index] {
            TokenTree::Group(group) => {
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                let expanded = expand_tokens(&inner, ctx)?;
                let mut rebuilt = Group::new(group.delimiter(), expanded);
                rebuilt.set_span(group.span());
                output.extend([TokenTree::Group(rebuilt)]);
            }
            other => output.extend([other.clone()]),
        }
        index += 1;
    }
    Ok(output)
}

fn try_prefixed(
    tokens: &[TokenTree],
    start: usize,
    ctx: &EvalCtx<'_>,
) -> Result<Option<(TokenStream, usize)>> {
    let Some(TokenTree::Ident(name)) = tokens.get(start) else {
        return Ok(None);
    };
    if is_sequence_start(tokens, start)? {
        return Ok(None);
    }
    if is_plus(tokens, start + 1) && is_sequence_start(tokens, start + 2)? {
        let mut elements = vec![TokenStream::from(TokenTree::Ident(name.clone()))];
        let mut next = start + 2;
        loop {
            let Some(part) = try_one(tokens, next, ctx)? else {
                break;
            };
            elements.extend(part.elements);
            next = part.next;
            if is_plus(tokens, next) && is_sequence_start(tokens, next + 1)? {
                next += 1;
                continue;
            }
            break;
        }
        return Ok(Some((join_elements(&elements), next)));
    }
    if is_sequence_start(tokens, start + 1)? {
        return Err(Error::new(
            name.span(),
            "write `+` to concatenate; `X Name[..]` is not concatenation",
        ));
    }
    Ok(None)
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

fn try_chain(
    tokens: &[TokenTree],
    start: usize,
    ctx: &EvalCtx<'_>,
) -> Result<Option<(TokenStream, usize)>> {
    let Some(mut part) = try_one(tokens, start, ctx)? else {
        return Ok(None);
    };
    while is_plus(tokens, part.next) {
        match try_one(tokens, part.next + 1, ctx)? {
            Some(more) => {
                part.elements.extend(more.elements);
                part.next = more.next;
            }
            None => break,
        }
    }
    Ok(Some((join_elements(&part.elements), part.next)))
}

struct Expanded {
    elements: Vec<TokenStream>,
    next: usize,
}

fn try_one(tokens: &[TokenTree], index: usize, ctx: &EvalCtx<'_>) -> Result<Option<Expanded>> {
    match (tokens.get(index), tokens.get(index + 1)) {
        (Some(TokenTree::Ident(name)), Some(TokenTree::Group(brackets)))
            if brackets.delimiter() == Delimiter::Bracket =>
        {
            match classify(brackets)? {
                Class::Pass => Ok(None),
                Class::Pattern(pattern) => Ok(Some(Expanded {
                    elements: expand_named(name, pattern, ctx)?,
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
                Class::Pattern(pattern) => Ok(Some(Expanded {
                    elements: expand_rows(paren, pattern, ctx)?,
                    next: index + 2,
                })),
            }
        }
        _ => Ok(None),
    }
}

fn expand_named(name: &Ident, pattern: Pattern, ctx: &EvalCtx<'_>) -> Result<Vec<TokenStream>> {
    if matches!(pattern, Pattern::Shorthand { .. }) && name.to_string() == "Off" {
        return Ok(vec![TokenStream::from(TokenTree::Ident(Ident::new(
            "Off",
            name.span(),
        )))]);
    }
    let indices = resolve_range(&pattern.to_range(), ctx)?;
    let mut elements = Vec::with_capacity(indices.len());
    for index in indices {
        elements.push(TokenStream::from(TokenTree::Ident(paste_ident(
            name, index,
        )?)));
    }
    Ok(elements)
}

fn expand_rows(template: &Group, pattern: Pattern, ctx: &EvalCtx<'_>) -> Result<Vec<TokenStream>> {
    let indices = resolve_range(&pattern.to_range(), ctx)?;
    let inner: Vec<TokenTree> = template.stream().into_iter().collect();
    let mut elements = Vec::with_capacity(indices.len());
    for index in indices {
        let row = EvalCtx {
            index,
            domain: ctx.domain,
        };
        let expanded = expand_tokens(&inner, &row)?;
        let mut group = Group::new(Delimiter::Parenthesis, expanded);
        group.set_span(template.span());
        elements.push(TokenStream::from(TokenTree::Group(group)));
    }
    Ok(elements)
}

fn join_elements(elements: &[TokenStream]) -> TokenStream {
    let mut output = TokenStream::new();
    for (index, element) in elements.iter().enumerate() {
        if index > 0 {
            output.extend(TokenStream::from(TokenTree::Punct(Punct::new(
                ',',
                Spacing::Alone,
            ))));
        }
        output.extend(element.clone());
    }
    output
}

fn try_paste(tokens: &[TokenTree], index: usize, ctx: &EvalCtx<'_>) -> Result<Option<Ident>> {
    let Some(TokenTree::Ident(name)) = tokens.get(index) else {
        return Ok(None);
    };
    if !is_plus(tokens, index + 1) || !is_bare_hash(tokens, index + 2) {
        return Ok(None);
    }
    Ok(Some(paste_ident(name, ctx.index)?))
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

fn paste_ident(name: &Ident, index: i128) -> Result<Ident> {
    if index < 0 {
        return Err(Error::new(
            name.span(),
            "cannot paste a negative index into an identifier",
        ));
    }
    let raw = name.to_string();
    let base = raw.strip_prefix("r#").unwrap_or(&raw);
    let pasted = format!("{base}{index}");
    // The base is already an identifier, and a decimal suffix cannot make it a
    // keyword or an invalid identifier.
    Ok(Ident::new(&pasted, name.span()))
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

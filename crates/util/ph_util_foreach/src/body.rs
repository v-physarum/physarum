//! Placeholder substitution for one `impl_foreach` invocation.
//!
//! The body is compiled once into one instruction tape. A header is followed
//! by its body on that same tape, so a group does not allocate its own vector.
//! Each index then only substitutes values. Pasted identifiers are written into
//! a stack buffer. Flat mode repeats the body once per outer index and clips
//! patterns to that full index set. Fan mode is selected when a bracket pattern
//! mentions `#` (including `Name[#]` and `Off[#]`). Each fan copy still uses one
//! outer index, but the domain is the prefix `0..=index`.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::error::{Error, Result};
use crate::range::{
    BoundStyle, Domain, EvalCtx, Pattern, compile_outer, parse_pattern, parse_range, visit_range,
};

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let (tokens, range_len, body) = split_input(input)?;
    let range = parse_range(&tokens[..range_len], BoundStyle::Outer)?;
    let outer = compile_outer(&range)?;
    if outer.is_empty() {
        return Ok(TokenStream::new());
    }
    let mut program = Program::from_body(body);
    let root = program.source.len();
    let stat = compile_seq(&mut program, 0, root)?;
    let mut output = TokenStream::new();
    if stat.fan {
        for position in 0..outer.count() {
            let index = outer.index_at(position);
            let domain = Domain::fan(index, range.span)?;
            emit_seq(
                &program,
                0,
                stat.logical,
                &EvalCtx {
                    index,
                    domain: &domain,
                },
                &mut output,
            )?;
        }
    } else {
        let domain = Domain::flat(&outer);
        for position in 0..outer.count() {
            emit_seq(
                &program,
                0,
                stat.logical,
                &EvalCtx {
                    index: outer.index_at(position),
                    domain: &domain,
                },
                &mut output,
            )?;
        }
    }
    Ok(output)
}

fn split_input(input: TokenStream) -> Result<(Vec<TokenTree>, usize, Vec<TokenTree>)> {
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
    let body_tokens = {
        let TokenTree::Group(body) = &tokens[body_at] else {
            return Err(Error::new(
                span,
                "expected a `{ ... }` body after the range",
            ));
        };
        body.stream().into_iter().collect()
    };
    Ok((tokens, body_at - 1, body_tokens))
}

/// One compiled body.
///
/// `ops` is the instruction tape. `tokens` holds verbatim runs. `bytes` holds
/// identifier text. `source` is only the token arena used while compiling.
struct Program {
    ops: Vec<Inst>,
    tokens: Vec<TokenTree>,
    bytes: Vec<u8>,
    source: Vec<TokenTree>,
}

#[derive(Clone)]
struct Inst {
    op: Op,
    /// Slots occupied by this instruction, including the body that follows it.
    width: u32,
}

#[derive(Clone)]
enum Op {
    Copy {
        at: u32,
        len: u32,
    },
    Group {
        delimiter: Delimiter,
        span: Span,
        /// Logical instructions in the body that follows this header.
        body: u32,
    },
    Index {
        span: Span,
    },
    Paste {
        at: u32,
        len: u32,
        span: Span,
    },
    List {
        pieces: u32,
    },
    Ident(Ident),
    Off(Ident),
    Named {
        at: u32,
        len: u32,
        span: Span,
        pattern: Pattern,
    },
    Rows {
        span: Span,
        pattern: Pattern,
        /// Logical instructions in the row template that follows this header.
        body: u32,
    },
}

struct SeqStat {
    logical: u32,
    physical: u32,
    expands: bool,
    fan: bool,
}

struct Counts {
    logical: u32,
    physical: u32,
    expands: bool,
    fan: bool,
    pending: Option<PendingCopy>,
}

struct PendingCopy {
    at: u32,
    len: u32,
}

struct Pushed {
    physical: u32,
    next: usize,
    fan: bool,
}

enum PieceHead {
    Ident {
        span: Span,
        raw: String,
        pattern: Pattern,
    },
    Rows {
        span: Span,
        pattern: Pattern,
        stream: TokenStream,
    },
}

enum Class {
    Pass,
    Pattern(Pattern),
}

impl Program {
    fn from_body(body: Vec<TokenTree>) -> Self {
        Self {
            ops: Vec::new(),
            tokens: Vec::new(),
            bytes: Vec::new(),
            source: body,
        }
    }

    fn pack_str(&mut self, text: &str) -> Result<(u32, u32)> {
        let at = fit(self.bytes.len())?;
        let len = fit(text.len())?;
        self.bytes.extend(text.as_bytes());
        Ok((at, len))
    }

    fn text(&self, at: u32, len: u32) -> Result<&str> {
        let start = widen(at)?;
        let end = start.checked_add(widen(len)?).ok_or_else(too_large)?;
        let bytes = self.bytes.get(start..end).ok_or_else(too_large)?;
        std::str::from_utf8(bytes).map_err(|_| too_large())
    }
}

impl Counts {
    fn new() -> Self {
        Self {
            logical: 0,
            physical: 0,
            expands: false,
            fan: false,
            pending: None,
        }
    }
}

fn compile_seq(prog: &mut Program, source_at: usize, source_len: usize) -> Result<SeqStat> {
    let source_limit = prog.source.len();
    let stat = compile_into(prog, source_at, source_len)?;
    prog.source.truncate(source_limit);
    Ok(stat)
}

fn compile_into(prog: &mut Program, source_at: usize, source_len: usize) -> Result<SeqStat> {
    let mut counts = Counts::new();
    compile_range(prog, &mut counts, source_at, source_len)?;
    Ok(SeqStat {
        logical: counts.logical,
        physical: counts.physical,
        expands: counts.expands,
        fan: counts.fan,
    })
}

fn try_compile_list(
    prog: &mut Program,
    counts: &mut Counts,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<Option<usize>> {
    let Some(first) = try_push_piece(prog, source_at, source_len, index)? else {
        return Ok(None);
    };
    let saved = rewind(prog.ops.len(), first.physical)?;
    let (pieces, physical, next, fan) = chain_pieces(
        prog,
        source_at,
        source_len,
        1,
        first.physical,
        first.next,
        first.fan,
    )?;
    let width = checked_add(physical, 1)?;
    insert_before_body(prog, counts, saved, Op::List { pieces }, width, fan)?;
    Ok(Some(next))
}

fn try_compile_prefixed(
    prog: &mut Program,
    counts: &mut Counts,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<Option<usize>> {
    let Some(ident) = leading_ident(prog, source_at, source_len, index)? else {
        return Ok(None);
    };
    let Some(after_name) = index.checked_add(1) else {
        return Ok(None);
    };
    if is_plus(prog, source_at, source_len, after_name)? {
        let Some(piece_at) = index.checked_add(2) else {
            return Ok(None);
        };
        let Some(first) = try_push_piece(prog, source_at, source_len, piece_at)? else {
            return Ok(None);
        };
        let saved = rewind(prog.ops.len(), first.physical)?;
        prog.ops.insert(
            saved,
            Inst {
                op: Op::Ident(ident),
                width: 1,
            },
        );
        let physical = checked_add(1, first.physical)?;
        let (pieces, physical, next, fan) = chain_pieces(
            prog, source_at, source_len, 2, physical, first.next, first.fan,
        )?;
        let width = checked_add(physical, 1)?;
        insert_before_body(prog, counts, saved, Op::List { pieces }, width, fan)?;
        return Ok(Some(next));
    }
    if is_sequence_start(prog, source_at, source_len, after_name)? {
        return Err(Error::new(
            ident.span(),
            "write `+` to concatenate; `X Name[..]` is not concatenation",
        ));
    }
    Ok(None)
}

fn chain_pieces(
    prog: &mut Program,
    source_at: usize,
    source_len: usize,
    mut pieces: u32,
    mut physical: u32,
    mut next: usize,
    mut fan: bool,
) -> Result<(u32, u32, usize, bool)> {
    while is_plus(prog, source_at, source_len, next)? {
        let Some(followed) = next.checked_add(1) else {
            break;
        };
        let Some(chained) = try_push_piece(prog, source_at, source_len, followed)? else {
            break;
        };
        pieces = checked_add(pieces, 1)?;
        physical = checked_add(physical, chained.physical)?;
        next = chained.next;
        fan |= chained.fan;
    }
    Ok((pieces, physical, next, fan))
}

fn try_push_piece(
    prog: &mut Program,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<Option<Pushed>> {
    let Some(head) = peek_piece(prog, source_at, source_len, index)? else {
        return Ok(None);
    };
    let (physical, fan) = match head {
        PieceHead::Ident { span, raw, pattern } => push_named(prog, span, raw, pattern)?,
        PieceHead::Rows {
            span,
            pattern,
            stream,
        } => push_rows(prog, span, pattern, stream)?,
    };
    Ok(Some(Pushed {
        physical,
        next: index.checked_add(2).ok_or_else(too_large)?,
        fan,
    }))
}

fn push_named(
    prog: &mut Program,
    span: Span,
    raw: String,
    pattern: Pattern,
) -> Result<(u32, bool)> {
    let fan = pattern.uses_index();
    let op = if matches!(pattern, Pattern::Shorthand { .. }) && raw == "Off" {
        Op::Off(Ident::new("Off", span))
    } else {
        let base = ident_base_owned(raw);
        let (at, len) = prog.pack_str(&base)?;
        Op::Named {
            at,
            len,
            span,
            pattern,
        }
    };
    prog.ops.push(Inst { op, width: 1 });
    Ok((1, fan))
}

fn push_rows(
    prog: &mut Program,
    span: Span,
    pattern: Pattern,
    stream: TokenStream,
) -> Result<(u32, bool)> {
    let saved = prog.ops.len();
    let nested = compile_stream(prog, stream)?;
    let width = checked_add(nested.physical, 1)?;
    prog.ops.insert(
        saved,
        Inst {
            op: Op::Rows {
                span,
                pattern,
                body: nested.logical,
            },
            width,
        },
    );
    Ok((width, pattern.uses_index() || nested.fan))
}

fn try_compile_paste(
    prog: &mut Program,
    counts: &mut Counts,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<Option<usize>> {
    let prepared = {
        let tokens = source_slice(prog, source_at, source_len)?;
        match tokens.get(index) {
            Some(TokenTree::Ident(name))
                if is_plus_at(tokens, index + 1) && is_bare_hash(tokens, index + 2) =>
            {
                Some((ident_base(name), name.span()))
            }
            _ => None,
        }
    };
    let Some((text, span)) = prepared else {
        return Ok(None);
    };
    let (at, len) = prog.pack_str(&text)?;
    flush_copy(prog, counts)?;
    push_op(prog, counts, Op::Paste { at, len, span }, false)?;
    Ok(Some(index + 3))
}

fn try_compile_hash(
    prog: &mut Program,
    counts: &mut Counts,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<bool> {
    let span = {
        let tokens = source_slice(prog, source_at, source_len)?;
        if !is_bare_hash(tokens, index) {
            return Ok(false);
        }
        hash_span(tokens, index)
    };
    flush_copy(prog, counts)?;
    push_op(prog, counts, Op::Index { span }, false)?;
    Ok(true)
}

fn try_compile_group(
    prog: &mut Program,
    counts: &mut Counts,
    source_at: usize,
    index: usize,
) -> Result<bool> {
    let pos = source_pos(source_at, index)?;
    let Some((delimiter, span, stream)) = group_parts(prog, pos) else {
        return Ok(false);
    };
    let saved_ops = prog.ops.len();
    let saved_tokens = prog.tokens.len();
    let saved_bytes = prog.bytes.len();
    let nested = compile_stream(prog, stream)?;
    if nested.expands {
        let width = checked_add(nested.physical, 1)?;
        insert_before_body(
            prog,
            counts,
            saved_ops,
            Op::Group {
                delimiter,
                span,
                body: nested.logical,
            },
            width,
            nested.fan,
        )?;
    } else {
        prog.ops.truncate(saved_ops);
        prog.tokens.truncate(saved_tokens);
        prog.bytes.truncate(saved_bytes);
        push_verbatim(prog, counts, cloned_token(prog, pos)?)?;
    }
    Ok(true)
}

fn compile_stream(prog: &mut Program, stream: TokenStream) -> Result<SeqStat> {
    let inner_at = prog.source.len();
    prog.source.extend(stream);
    let inner_len = prog
        .source
        .len()
        .checked_sub(inner_at)
        .ok_or_else(too_large)?;
    let stat = compile_into(prog, inner_at, inner_len)?;
    prog.source.truncate(inner_at);
    Ok(stat)
}

fn compile_range(
    prog: &mut Program,
    counts: &mut Counts,
    source_at: usize,
    source_len: usize,
) -> Result<()> {
    let mut index = 0;
    while index < source_len {
        if let Some(next) = try_compile_list(prog, counts, source_at, source_len, index)? {
            index = next;
            continue;
        }
        if let Some(next) = try_compile_paste(prog, counts, source_at, source_len, index)? {
            index = next;
            continue;
        }
        reject_spaced_paste(prog, source_at, source_len, index)?;
        if let Some(next) = try_compile_prefixed(prog, counts, source_at, source_len, index)? {
            index = next;
            continue;
        }
        if try_compile_hash(prog, counts, source_at, source_len, index)? {
            index += 1;
            continue;
        }
        if try_compile_group(prog, counts, source_at, index)? {
            index += 1;
            continue;
        }
        let token = cloned_token(prog, source_pos(source_at, index)?)?;
        push_verbatim(prog, counts, token)?;
        index += 1;
    }
    flush_copy(prog, counts)?;
    Ok(())
}

fn push_op(prog: &mut Program, counts: &mut Counts, op: Op, fan: bool) -> Result<()> {
    prog.ops.push(Inst { op, width: 1 });
    counts.logical = checked_add(counts.logical, 1)?;
    counts.physical = checked_add(counts.physical, 1)?;
    counts.expands = true;
    counts.fan |= fan;
    Ok(())
}

fn push_verbatim(prog: &mut Program, counts: &mut Counts, token: TokenTree) -> Result<()> {
    let at = fit(prog.tokens.len())?;
    prog.tokens.push(token);
    match &mut counts.pending {
        Some(pending) => pending.len = checked_add(pending.len, 1)?,
        None => counts.pending = Some(PendingCopy { at, len: 1 }),
    }
    Ok(())
}

fn flush_copy(prog: &mut Program, counts: &mut Counts) -> Result<()> {
    let Some(pending) = counts.pending.take() else {
        return Ok(());
    };
    prog.ops.push(Inst {
        op: Op::Copy {
            at: pending.at,
            len: pending.len,
        },
        width: 1,
    });
    counts.logical = checked_add(counts.logical, 1)?;
    counts.physical = checked_add(counts.physical, 1)?;
    Ok(())
}

fn insert_before_body(
    prog: &mut Program,
    counts: &mut Counts,
    at: usize,
    op: Op,
    width: u32,
    fan: bool,
) -> Result<()> {
    prog.ops.insert(at, Inst { op, width });
    if let Some(pending) = counts.pending.take() {
        prog.ops.insert(
            at,
            Inst {
                op: Op::Copy {
                    at: pending.at,
                    len: pending.len,
                },
                width: 1,
            },
        );
        counts.logical = checked_add(counts.logical, 1)?;
        counts.physical = checked_add(counts.physical, 1)?;
    }
    counts.logical = checked_add(counts.logical, 1)?;
    counts.physical = checked_add(counts.physical, width)?;
    counts.expands = true;
    counts.fan |= fan;
    Ok(())
}

fn peek_piece(
    prog: &mut Program,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<Option<PieceHead>> {
    let Some(pattern) = piece_pattern(prog, source_at, source_len, index)? else {
        return Ok(None);
    };
    let head = source_pos(source_at, index)?;
    match prog.source.get(head) {
        Some(TokenTree::Ident(name)) => Ok(Some(PieceHead::Ident {
            span: name.span(),
            raw: name.to_string(),
            pattern,
        })),
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            Ok(Some(PieceHead::Rows {
                span: group.span(),
                pattern,
                stream: group.stream(),
            }))
        }
        _ => Ok(None),
    }
}

fn is_sequence_start(
    prog: &mut Program,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<bool> {
    Ok(piece_pattern(prog, source_at, source_len, index)?.is_some())
}

fn piece_pattern(
    prog: &mut Program,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<Option<Pattern>> {
    let Some(next) = index.checked_add(1) else {
        return Ok(None);
    };
    if next >= source_len {
        return Ok(None);
    }
    let head = source_pos(source_at, index)?;
    let bracket = source_pos(source_at, next)?;
    let Some((span, stream)) = bracket_of_piece(prog, head, bracket) else {
        return Ok(None);
    };
    match classify_stream(prog, stream, span)? {
        Class::Pattern(pattern) => Ok(Some(pattern)),
        Class::Pass => Ok(None),
    }
}

fn bracket_of_piece(prog: &Program, head: usize, bracket: usize) -> Option<(Span, TokenStream)> {
    let token = prog.source.get(head)?;
    if !opens_piece(token) {
        return None;
    }
    match prog.source.get(bracket) {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket => {
            Some((group.span(), group.stream()))
        }
        _ => None,
    }
}

fn opens_piece(token: &TokenTree) -> bool {
    match token {
        TokenTree::Ident(_) => true,
        TokenTree::Group(group) => group.delimiter() == Delimiter::Parenthesis,
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    }
}

fn classify_stream(prog: &mut Program, stream: TokenStream, span: Span) -> Result<Class> {
    let start = prog.source.len();
    prog.source.extend(stream);
    let class = classify_at(prog, start, span)?;
    prog.source.truncate(start);
    Ok(class)
}

fn classify_at(prog: &Program, start: usize, span: Span) -> Result<Class> {
    let Some(tokens) = prog.source.get(start..) else {
        return Err(too_large());
    };
    if is_exactly_hash(tokens) {
        return Ok(Class::Pattern(Pattern::Shorthand { span }));
    }
    match parse_pattern(tokens)? {
        Some(mut range) => {
            range.span = span;
            Ok(Class::Pattern(Pattern::Range(range)))
        }
        None => Ok(Class::Pass),
    }
}

fn leading_ident(
    prog: &Program,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<Option<Ident>> {
    let tokens = source_slice(prog, source_at, source_len)?;
    Ok(match tokens.get(index) {
        Some(TokenTree::Ident(name)) => Some(name.clone()),
        _ => None,
    })
}

fn group_parts(prog: &Program, pos: usize) -> Option<(Delimiter, Span, TokenStream)> {
    match prog.source.get(pos) {
        Some(TokenTree::Group(group)) => Some((group.delimiter(), group.span(), group.stream())),
        _ => None,
    }
}

fn cloned_token(prog: &Program, pos: usize) -> Result<TokenTree> {
    prog.source.get(pos).cloned().ok_or_else(too_large)
}

fn is_plus(prog: &Program, source_at: usize, source_len: usize, index: usize) -> Result<bool> {
    let tokens = source_slice(prog, source_at, source_len)?;
    Ok(is_plus_at(tokens, index))
}

fn source_slice(prog: &Program, source_at: usize, source_len: usize) -> Result<&[TokenTree]> {
    let end = source_at.checked_add(source_len).ok_or_else(too_large)?;
    prog.source.get(source_at..end).ok_or_else(too_large)
}

fn source_pos(source_at: usize, index: usize) -> Result<usize> {
    source_at.checked_add(index).ok_or_else(too_large)
}

fn emit_seq(
    prog: &Program,
    mut at: usize,
    logical: u32,
    ctx: &EvalCtx<'_>,
    out: &mut TokenStream,
) -> Result<()> {
    for _ in 0..logical {
        let inst = prog.ops.get(at).cloned().ok_or_else(too_large)?;
        let width = widen(inst.width)?;
        if width == 0 {
            return Err(too_large());
        }
        match inst.op {
            Op::Copy { at: tok_at, len } => emit_copy(prog, tok_at, len, out)?,
            Op::Group {
                delimiter,
                span,
                body,
            } => emit_group(prog, at, delimiter, span, body, ctx, out)?,
            Op::Index { span } => {
                out.extend([TokenTree::Literal(index_literal(ctx.index, span))]);
            }
            Op::Paste {
                at: base_at,
                len,
                span,
            } => {
                emit_pasted(prog.text(base_at, len)?, ctx.index, span, out)?;
            }
            Op::List { pieces } => {
                let body_at = at.checked_add(1).ok_or_else(too_large)?;
                emit_list(prog, body_at, pieces, ctx, out)?;
            }
            Op::Ident(_) | Op::Off(_) | Op::Named { .. } | Op::Rows { .. } => {
                let mut emitted = false;
                emit_piece(prog, at, ctx, &mut emitted, out)?;
            }
        }
        at = at.checked_add(width).ok_or_else(too_large)?;
    }
    Ok(())
}

fn emit_group(
    prog: &Program,
    at: usize,
    delimiter: Delimiter,
    span: Span,
    body: u32,
    ctx: &EvalCtx<'_>,
    out: &mut TokenStream,
) -> Result<()> {
    let mut inner = TokenStream::new();
    let body_at = at.checked_add(1).ok_or_else(too_large)?;
    emit_seq(prog, body_at, body, ctx, &mut inner)?;
    let mut group = Group::new(delimiter, inner);
    group.set_span(span);
    out.extend([TokenTree::Group(group)]);
    Ok(())
}

fn emit_list(
    prog: &Program,
    mut at: usize,
    pieces: u32,
    ctx: &EvalCtx<'_>,
    out: &mut TokenStream,
) -> Result<()> {
    let mut emitted = false;
    for _ in 0..pieces {
        let width = prog
            .ops
            .get(at)
            .map(|inst| inst.width)
            .ok_or_else(too_large)?;
        let width = widen(width)?;
        if width == 0 {
            return Err(too_large());
        }
        emit_piece(prog, at, ctx, &mut emitted, out)?;
        at = at.checked_add(width).ok_or_else(too_large)?;
    }
    Ok(())
}

fn emit_piece(
    prog: &Program,
    at: usize,
    ctx: &EvalCtx<'_>,
    emitted: &mut bool,
    out: &mut TokenStream,
) -> Result<()> {
    let op = prog.ops.get(at).cloned().ok_or_else(too_large)?.op;
    match op {
        Op::Ident(ident) | Op::Off(ident) => {
            push_comma(emitted, out);
            out.extend([TokenTree::Ident(ident)]);
        }
        Op::Named {
            at: base_at,
            len,
            span,
            pattern,
        } => {
            let range = pattern.to_range();
            visit_range(&range, ctx, |index| {
                push_comma(emitted, out);
                emit_pasted(prog.text(base_at, len)?, index, span, out)
            })?;
        }
        Op::Rows {
            span,
            pattern,
            body,
        } => {
            let range = pattern.to_range();
            let domain = ctx.domain;
            let body_at = at.checked_add(1).ok_or_else(too_large)?;
            visit_range(&range, ctx, |index| {
                let mut inner = TokenStream::new();
                emit_seq(prog, body_at, body, &EvalCtx { index, domain }, &mut inner)?;
                push_comma(emitted, out);
                let mut group = Group::new(Delimiter::Parenthesis, inner);
                group.set_span(span);
                out.extend([TokenTree::Group(group)]);
                Ok(())
            })?;
        }
        Op::Copy { .. }
        | Op::Group { .. }
        | Op::Index { .. }
        | Op::Paste { .. }
        | Op::List { .. } => {
            return Err(too_large());
        }
    }
    Ok(())
}

fn emit_copy(prog: &Program, at: u32, len: u32, out: &mut TokenStream) -> Result<()> {
    let start = widen(at)?;
    let end = start.checked_add(widen(len)?).ok_or_else(too_large)?;
    let Some(tokens) = prog.tokens.get(start..end) else {
        return Err(too_large());
    };
    out.extend(tokens.iter().cloned());
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

fn is_exactly_hash(tokens: &[TokenTree]) -> bool {
    match tokens {
        [TokenTree::Punct(punct)] => punct.as_char() == '#',
        _ => false,
    }
}

fn reject_spaced_paste(
    prog: &Program,
    source_at: usize,
    source_len: usize,
    index: usize,
) -> Result<()> {
    let tokens = source_slice(prog, source_at, source_len)?;
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

fn is_plus_at(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == '+')
}

fn ident_base(name: &Ident) -> String {
    ident_base_owned(name.to_string())
}

fn ident_base_owned(raw: String) -> String {
    match raw.strip_prefix("r#") {
        Some(base) => base.to_string(),
        None => raw,
    }
}

fn fit(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| too_large())
}

fn widen(value: u32) -> Result<usize> {
    usize::try_from(value).map_err(|_| too_large())
}

fn checked_add(left: u32, right: u32) -> Result<u32> {
    left.checked_add(right).ok_or_else(too_large)
}

fn rewind(len: usize, physical: u32) -> Result<usize> {
    len.checked_sub(widen(physical)?).ok_or_else(too_large)
}

fn too_large() -> Error {
    Error::new(Span::call_site(), "body is too large")
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

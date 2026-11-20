//! Placeholder substitution for one `impl_foreach` invocation.
//!
//! The body is compiled once into one instruction tape. A header is followed
//! by its body on that same tape, so a group does not allocate its own vector.
//! Each index then only substitutes values. Pasted identifiers are written into
//! a stack buffer. Flat mode repeats the body once per outer index and clips
//! patterns to that full index set. Fan mode is selected when a bracket pattern
//! mentions `#` (including `Name[#]` and `Off[#]`). Each fan copy still uses one
//! outer index, but the domain is the prefix `0..=index`.

use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::error::{Error, ErrorKind, Result};
use crate::range::{BoundStyle, Domain, EvalCtx, Pattern, RangeExpr};

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let (range_tokens, body) = input.split_call()?;
    let range = RangeExpr::parse(&range_tokens, BoundStyle::Outer)?;
    let outer = range.compile_outer()?;
    if outer.is_empty() {
        return Ok(TokenStream::new());
    }
    let mut program = Program::from_body(body);
    let root = program.source.len();
    let stat = program.compile_seq(Source::new(0, root))?;
    let mut output = TokenStream::new();
    if stat.fan {
        if outer.first() < 0 {
            return Err(Error::new(range.span, ErrorKind::FanEmpty));
        }
        outer.each(|index| {
            let domain = Domain::prefix(index);
            let tape = Tape::new(0, stat.logical);
            output
                .with_program(&program)
                .emit_seq(tape, &EvalCtx::new(index, &domain))
        })?;
    } else {
        let domain = Domain::flat(&outer);
        outer.each(|index| {
            let tape = Tape::new(0, stat.logical);
            output
                .with_program(&program)
                .emit_seq(tape, &EvalCtx::new(index, &domain))
        })?;
    }
    Ok(output)
}

trait Call {
    fn split_call(self) -> Result<(Vec<TokenTree>, Vec<TokenTree>)>;
}

impl Call for TokenStream {
    fn split_call(self) -> Result<(Vec<TokenTree>, Vec<TokenTree>)> {
        let tokens: Vec<TokenTree> = self.into_iter().collect();
        let span = tokens.first().map_or_else(Span::call_site, TokenTree::span);
        let Some(comma_at) = tokens.iter().position(OneToken::is_comma) else {
            if tokens.iter().any(OneToken::is_brace) {
                return Err(Error::new(span, ErrorKind::CallForm));
            }
            return Err(Error::new(
                tokens.last().map_or(span, TokenTree::span),
                ErrorKind::ExpectedBody,
            ));
        };
        let (left_name, left_value) = tokens[..comma_at].named_value(span)?;
        let right_span = tokens.get(comma_at + 1).map_or(span, TokenTree::span);
        let (right_name, right_value) = tokens[comma_at + 1..].named_value(right_span)?;
        let (range, body) = match (left_name.as_str(), right_name.as_str()) {
            ("range", "body") => (left_value, right_value),
            ("body", "range") => (right_value, left_value),
            _ => return Err(Error::new(span, ErrorKind::CallForm)),
        };
        if range.is_empty() {
            return Err(Error::new(span, ErrorKind::ExpectedRange));
        }
        Ok((range, body.brace_body(span)?))
    }
}

trait OneToken {
    fn is_comma(&self) -> bool;
    fn is_brace(&self) -> bool;
    fn opens_piece(&self) -> bool;
}

impl OneToken for TokenTree {
    fn is_comma(&self) -> bool {
        matches!(self, TokenTree::Punct(punct) if punct.as_char() == ',')
    }

    fn is_brace(&self) -> bool {
        matches!(self, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace)
    }

    fn opens_piece(&self) -> bool {
        match self {
            TokenTree::Ident(_) => true,
            TokenTree::Group(group) => group.delimiter() == Delimiter::Parenthesis,
            TokenTree::Punct(_) | TokenTree::Literal(_) => false,
        }
    }
}

trait ArgTokens {
    fn named_value(&self, span: Span) -> Result<(String, Vec<TokenTree>)>;
    fn brace_body(&self, span: Span) -> Result<Vec<TokenTree>>;
}

impl ArgTokens for [TokenTree] {
    fn named_value(&self, span: Span) -> Result<(String, Vec<TokenTree>)> {
        let Some(TokenTree::Ident(name)) = self.first() else {
            return Err(Error::new(span, ErrorKind::CallForm));
        };
        if !self.is_punct_at(1, '=') {
            return Err(Error::new(name.span(), ErrorKind::CallForm));
        }
        Ok((name.to_string(), self[2..].to_vec()))
    }

    fn brace_body(&self, span: Span) -> Result<Vec<TokenTree>> {
        match self {
            [TokenTree::Group(group)] if group.delimiter() == Delimiter::Brace => {
                Ok(group.stream().into_iter().collect())
            }
            [TokenTree::Group(group), extra, ..] if group.delimiter() == Delimiter::Brace => {
                Err(Error::new(extra.span(), ErrorKind::UnexpectedAfterBody))
            }
            _ => Err(Error::new(
                self.first().map_or(span, TokenTree::span),
                ErrorKind::ExpectedBody,
            )),
        }
    }
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

struct Inst {
    op: Op,
    /// Slots occupied by this instruction, including the body that follows it.
    width: u32,
}

impl Inst {
    fn new(op: Op, width: u32) -> Self {
        Self { op, width }
    }
}

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

impl SeqStat {
    fn new(counts: &Counts) -> Self {
        Self {
            logical: counts.logical,
            physical: counts.physical,
            expands: counts.expands,
            fan: counts.fan,
        }
    }
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

impl PendingCopy {
    fn new(at: u32, len: u32) -> Self {
        Self { at, len }
    }
}

struct Pushed {
    physical: u32,
    next: usize,
    fan: bool,
}

impl Pushed {
    fn new(physical: u32, next: usize, fan: bool) -> Self {
        Self { physical, next, fan }
    }
}

/// One window of the token arena.
#[derive(Clone, Copy)]
struct Source {
    at: usize,
    len: usize,
}

impl Source {
    fn new(at: usize, len: usize) -> Self {
        Self { at, len }
    }

    fn pos(self, index: usize) -> Result<usize> {
        self.at.checked_add(index).ok_or_else(Error::too_large)
    }
}

/// An identifier and the `+` that follows it.
struct Prefix {
    ident: Ident,
    plus_at: usize,
}

impl Prefix {
    fn new(ident: Ident, plus_at: usize) -> Self {
        Self { ident, plus_at }
    }
}

/// An instruction inserted ahead of the body that was just compiled.
struct Inserted {
    at: usize,
    op: Op,
    width: u32,
    fan: bool,
}

impl Inserted {
    fn new(at: usize, op: Op, width: u32, fan: bool) -> Self {
        Self { at, op, width, fan }
    }
}

/// A delimited group whose body expanded.
struct Expanding {
    delimiter: Delimiter,
    span: Span,
    nested: SeqStat,
}

impl Expanding {
    fn new(delimiter: Delimiter, span: Span, nested: SeqStat) -> Self {
        Self { delimiter, span, nested }
    }
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
        Self { ops: Vec::new(), tokens: Vec::new(), bytes: Vec::new(), source: body }
    }

    fn pack_str(&mut self, text: &str) -> Result<(u32, u32)> {
        let at = Self::fit(self.bytes.len())?;
        let len = Self::fit(text.len())?;
        self.bytes.extend(text.as_bytes());
        Ok((at, len))
    }

    fn text(&self, at: u32, len: u32) -> Result<&str> {
        let start = Program::widen(at)?;
        let end = start
            .checked_add(Program::widen(len)?)
            .ok_or_else(Error::too_large)?;
        let bytes = self.bytes.get(start..end).ok_or_else(Error::too_large)?;
        std::str::from_utf8(bytes).map_err(|_| Error::too_large())
    }
    fn compile_seq(&mut self, source: Source) -> Result<SeqStat> {
        let source_limit = self.source.len();
        let stat = self.compile_into(source)?;
        self.source.truncate(source_limit);
        Ok(stat)
    }
    fn compile_into(&mut self, source: Source) -> Result<SeqStat> {
        let mut counts = Counts::new();
        self.compile_range(&mut counts, source)?;
        Ok(SeqStat::new(&counts))
    }
    fn compile_bracket_sequence(
        &mut self,
        counts: &mut Counts,
        source: Source,
        index: usize,
    ) -> Result<Option<usize>> {
        let Some(first) = self.try_push_piece(source, index)? else {
            return Ok(None);
        };
        let saved = Self::rewind(self.ops.len(), first.physical)?;
        let (pieces, first) = self.chain_pieces(source, 1, first)?;
        let width = Self::checked_add(first.physical, 1)?;
        let inserted = Inserted::new(saved, Op::List { pieces }, width, first.fan);
        self.insert_before_body(counts, inserted)?;
        Ok(Some(first.next))
    }
    fn compile_prefixed_sequence(
        &mut self,
        counts: &mut Counts,
        source: Source,
        index: usize,
    ) -> Result<Option<usize>> {
        let Some(ident) = self.leading_ident(source, index)? else {
            return Ok(None);
        };
        let Some(after_name) = index.checked_add(1) else {
            return Ok(None);
        };
        if self.is_plus(source, after_name)? {
            let prefix = Prefix::new(ident, after_name);
            return self.concat_ident_and_sequence(counts, source, prefix);
        }
        self.reject_ident_before_sequence(source, &ident, after_name)
    }
    fn concat_ident_and_sequence(
        &mut self,
        counts: &mut Counts,
        source: Source,
        prefix: Prefix,
    ) -> Result<Option<usize>> {
        let Some(piece_at) = prefix.plus_at.checked_add(1) else {
            return Ok(None);
        };
        let Some(first) = self.try_push_piece(source, piece_at)? else {
            return Ok(None);
        };
        let saved = Self::rewind(self.ops.len(), first.physical)?;
        self.place_in_front(saved, Inst::new(Op::Ident(prefix.ident), 1));
        let physical = Self::checked_add(1, first.physical)?;
        let first = Pushed::new(physical, first.next, first.fan);
        let (pieces, first) = self.chain_pieces(source, 2, first)?;
        let width = Self::checked_add(first.physical, 1)?;
        let inserted = Inserted::new(saved, Op::List { pieces }, width, first.fan);
        self.insert_before_body(counts, inserted)?;
        Ok(Some(first.next))
    }
    fn reject_ident_before_sequence(
        &mut self,
        source: Source,
        ident: &Ident,
        after_name: usize,
    ) -> Result<Option<usize>> {
        if self.is_sequence_start(source, after_name)? {
            return Err(Error::new(ident.span(), ErrorKind::NotConcat));
        }
        Ok(None)
    }
    fn chain_pieces(
        &mut self,
        source: Source,
        mut pieces: u32,
        mut first: Pushed,
    ) -> Result<(u32, Pushed)> {
        while self.is_plus(source, first.next)? {
            let Some(followed) = first.next.checked_add(1) else {
                break;
            };
            let Some(chained) = self.try_push_piece(source, followed)? else {
                break;
            };
            pieces = Self::checked_add(pieces, 1)?;
            let physical = Self::checked_add(first.physical, chained.physical)?;
            let fan = first.fan || chained.fan;
            first = Pushed::new(physical, chained.next, fan);
        }
        Ok((pieces, first))
    }
    fn try_push_piece(&mut self, source: Source, index: usize) -> Result<Option<Pushed>> {
        let Some(head) = self.peek_piece(source, index)? else {
            return Ok(None);
        };
        let (physical, fan) = match head {
            PieceHead::Ident { span, raw, pattern } => self.push_named(span, &raw, pattern)?,
            PieceHead::Rows { span, pattern, stream } => self.push_rows(span, pattern, stream)?,
        };
        Ok(Some(Pushed::new(
            physical,
            index.checked_add(2).ok_or_else(Error::too_large)?,
            fan,
        )))
    }
    fn push_named(&mut self, span: Span, raw: &str, pattern: Pattern) -> Result<(u32, bool)> {
        let fan = pattern.uses_index();
        let op = if pattern.is_off(raw) {
            Op::Off(Ident::new("Off", span))
        } else {
            let (at, len) = self.pack_str(paste_base(raw))?;
            Op::Named { at, len, span, pattern }
        };
        self.ops.push(Inst::new(op, 1));
        Ok((1, fan))
    }
    fn push_rows(
        &mut self,
        span: Span,
        pattern: Pattern,
        stream: TokenStream,
    ) -> Result<(u32, bool)> {
        let saved = self.ops.len();
        let nested = self.compile_stream(stream)?;
        let width = Self::checked_add(nested.physical, 1)?;
        self.place_in_front(
            saved,
            Inst::new(Op::Rows { span, pattern, body: nested.logical }, width),
        );
        Ok((width, pattern.uses_index() || nested.fan))
    }
    fn compile_pasted_ident(
        &mut self,
        counts: &mut Counts,
        source: Source,
        index: usize,
    ) -> Result<Option<usize>> {
        let prepared = {
            let tokens = self.source_slice(source)?;
            match tokens.get(index) {
                Some(TokenTree::Ident(name)) if tokens.is_paste_suffix(index) => {
                    Some((name.to_string(), name.span()))
                }
                _ => None,
            }
        };
        let Some((text, span)) = prepared else {
            return Ok(None);
        };
        let (at, len) = self.pack_str(paste_base(&text))?;
        self.flush_copy(counts)?;
        self.push_op(counts, Op::Paste { at, len, span }, false)?;
        Ok(Some(index + 3))
    }
    fn compile_index_literal(
        &mut self,
        counts: &mut Counts,
        source: Source,
        index: usize,
    ) -> Result<bool> {
        let span = {
            let tokens = self.source_slice(source)?;
            if !tokens.is_bare_hash(index) {
                return Ok(false);
            }
            tokens.hash_span(index)
        };
        self.flush_copy(counts)?;
        self.push_op(counts, Op::Index { span }, false)?;
        Ok(true)
    }
    fn compile_delimited_group(
        &mut self,
        counts: &mut Counts,
        source: Source,
        index: usize,
    ) -> Result<bool> {
        let pos = source.pos(index)?;
        let Some((delimiter, span, stream)) = self.group_parts(pos) else {
            return Ok(false);
        };
        let mark = self.mark();
        let nested = self.compile_stream(stream)?;
        if nested.expands {
            let group = Expanding::new(delimiter, span, nested);
            self.install_expanding_group(counts, mark.ops, &group)?;
        } else {
            mark.restore(self);
            self.keep_original_group(counts, pos)?;
        }
        Ok(true)
    }
    fn install_expanding_group(
        &mut self,
        counts: &mut Counts,
        at: usize,
        group: &Expanding,
    ) -> Result<()> {
        let width = Self::checked_add(group.nested.physical, 1)?;
        let op =
            Op::Group { delimiter: group.delimiter, span: group.span, body: group.nested.logical };
        self.insert_before_body(counts, Inserted::new(at, op, width, group.nested.fan))
    }
    fn keep_original_group(&mut self, counts: &mut Counts, pos: usize) -> Result<()> {
        self.push_verbatim(counts, self.cloned_token(pos)?)
    }
    fn compile_stream(&mut self, stream: TokenStream) -> Result<SeqStat> {
        let inner_at = self.source.len();
        self.source.extend(stream);
        let inner_len = self
            .source
            .len()
            .checked_sub(inner_at)
            .ok_or_else(Error::too_large)?;
        let stat = self.compile_into(Source::new(inner_at, inner_len))?;
        self.source.truncate(inner_at);
        Ok(stat)
    }
    fn compile_range(&mut self, counts: &mut Counts, source: Source) -> Result<()> {
        let mut index = 0;
        while index < source.len {
            index = self.compile_one(counts, source, index)?;
        }
        self.flush_copy(counts)
    }
    fn compile_one(&mut self, counts: &mut Counts, source: Source, index: usize) -> Result<usize> {
        if let Some(next) = self.compile_bracket_sequence(counts, source, index)? {
            return Ok(next);
        }
        if let Some(next) = self.compile_pasted_ident(counts, source, index)? {
            return Ok(next);
        }
        self.reject_spaced_ident_hash(source, index)?;
        if let Some(next) = self.compile_prefixed_sequence(counts, source, index)? {
            return Ok(next);
        }
        if self.compile_index_literal(counts, source, index)? {
            return Self::next_token(index);
        }
        if self.compile_delimited_group(counts, source, index)? {
            return Self::next_token(index);
        }
        self.copy_verbatim_token(counts, source, index)?;
        Self::next_token(index)
    }
    fn copy_verbatim_token(
        &mut self,
        counts: &mut Counts,
        source: Source,
        index: usize,
    ) -> Result<()> {
        let token = self.cloned_token(source.pos(index)?)?;
        self.push_verbatim(counts, token)
    }
    fn push_op(&mut self, counts: &mut Counts, op: Op, fan: bool) -> Result<()> {
        self.ops.push(Inst::new(op, 1));
        counts.logical = Self::checked_add(counts.logical, 1)?;
        counts.physical = Self::checked_add(counts.physical, 1)?;
        counts.expands = true;
        counts.fan |= fan;
        Ok(())
    }
    fn push_verbatim(&mut self, counts: &mut Counts, token: TokenTree) -> Result<()> {
        let at = Self::fit(self.tokens.len())?;
        self.tokens.push(token);
        match &mut counts.pending {
            Some(pending) => pending.len = Self::checked_add(pending.len, 1)?,
            None => counts.pending = Some(PendingCopy::new(at, 1)),
        }
        Ok(())
    }
    fn flush_copy(&mut self, counts: &mut Counts) -> Result<()> {
        let Some(pending) = counts.pending.take() else {
            return Ok(());
        };
        self.ops
            .push(Inst::new(Op::Copy { at: pending.at, len: pending.len }, 1));
        counts.logical = Self::checked_add(counts.logical, 1)?;
        counts.physical = Self::checked_add(counts.physical, 1)?;
        Ok(())
    }
    fn insert_before_body(&mut self, counts: &mut Counts, inserted: Inserted) -> Result<()> {
        let at = inserted.at;
        let width = inserted.width;
        let fan = inserted.fan;
        let tail = self.ops.split_off(at);
        if let Some(pending) = counts.pending.take() {
            self.ops
                .push(Inst::new(Op::Copy { at: pending.at, len: pending.len }, 1));
            counts.logical = Self::checked_add(counts.logical, 1)?;
            counts.physical = Self::checked_add(counts.physical, 1)?;
        }
        self.ops.push(Inst::new(inserted.op, width));
        self.ops.extend(tail);
        counts.logical = Self::checked_add(counts.logical, 1)?;
        counts.physical = Self::checked_add(counts.physical, width)?;
        counts.expands = true;
        counts.fan |= fan;
        Ok(())
    }

    /// Puts `inst` at `at` and keeps the instructions already there after it.
    fn place_in_front(&mut self, at: usize, inst: Inst) {
        let tail = self.ops.split_off(at);
        self.ops.push(inst);
        self.ops.extend(tail);
    }
    fn peek_piece(&mut self, source: Source, index: usize) -> Result<Option<PieceHead>> {
        let Some(pattern) = self.piece_pattern(source, index)? else {
            return Ok(None);
        };
        let head = source.pos(index)?;
        match self.source.get(head) {
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
    fn is_sequence_start(&mut self, source: Source, index: usize) -> Result<bool> {
        Ok(self.piece_pattern(source, index)?.is_some())
    }
    fn piece_pattern(&mut self, source: Source, index: usize) -> Result<Option<Pattern>> {
        let Some(next) = index.checked_add(1) else {
            return Ok(None);
        };
        if next >= source.len {
            return Ok(None);
        }
        let head = source.pos(index)?;
        let bracket = source.pos(next)?;
        let Some((span, stream)) = self.bracket_of_piece(head, bracket) else {
            return Ok(None);
        };
        match self.classify_stream(stream, span)? {
            Class::Pattern(pattern) => Ok(Some(pattern)),
            Class::Pass => Ok(None),
        }
    }
    fn bracket_of_piece(&self, head: usize, bracket: usize) -> Option<(Span, TokenStream)> {
        let token = self.source.get(head)?;
        if !token.opens_piece() {
            return None;
        }
        match self.source.get(bracket) {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket => {
                Some((group.span(), group.stream()))
            }
            _ => None,
        }
    }
    fn classify_stream(&mut self, stream: TokenStream, span: Span) -> Result<Class> {
        let start = self.source.len();
        self.source.extend(stream);
        let class = self.classify_at(start, span)?;
        self.source.truncate(start);
        Ok(class)
    }
    fn classify_at(&self, start: usize, span: Span) -> Result<Class> {
        let Some(tokens) = self.source.get(start..) else {
            return Err(Error::too_large());
        };
        if tokens.is_exactly_hash() {
            return Ok(Class::Pattern(Pattern::Shorthand { span }));
        }
        match RangeExpr::parse_pattern(tokens)? {
            Some(mut range) => {
                range.span = span;
                Ok(Class::Pattern(Pattern::Range(range)))
            }
            None => Ok(Class::Pass),
        }
    }
    fn leading_ident(&self, source: Source, index: usize) -> Result<Option<Ident>> {
        let tokens = self.source_slice(source)?;
        Ok(match tokens.get(index) {
            Some(TokenTree::Ident(name)) => Some(name.clone()),
            _ => None,
        })
    }
    fn group_parts(&self, pos: usize) -> Option<(Delimiter, Span, TokenStream)> {
        match self.source.get(pos) {
            Some(TokenTree::Group(group)) => {
                Some((group.delimiter(), group.span(), group.stream()))
            }
            _ => None,
        }
    }
    fn cloned_token(&self, pos: usize) -> Result<TokenTree> {
        self.source.get(pos).cloned().ok_or_else(Error::too_large)
    }
    fn is_plus(&self, source: Source, index: usize) -> Result<bool> {
        let tokens = self.source_slice(source)?;
        Ok(tokens.is_plus_at(index))
    }
    fn source_slice(&self, source: Source) -> Result<&[TokenTree]> {
        let end = source
            .at
            .checked_add(source.len)
            .ok_or_else(Error::too_large)?;
        self.source.get(source.at..end).ok_or_else(Error::too_large)
    }
    fn reject_spaced_ident_hash(&self, source: Source, index: usize) -> Result<()> {
        let tokens = self.source_slice(source)?;
        let Some(span) = tokens.spaced_paste_span(index) else {
            return Ok(());
        };
        Err(Error::new(span, ErrorKind::SpacedPaste))
    }

    fn mark(&self) -> BufferMark {
        BufferMark { ops: self.ops.len(), tokens: self.tokens.len(), bytes: self.bytes.len() }
    }

    fn next_token(index: usize) -> Result<usize> {
        index.checked_add(1).ok_or_else(Error::too_large)
    }

    fn fit(value: usize) -> Result<u32> {
        u32::try_from(value).map_err(|_| Error::too_large())
    }

    fn widen(value: u32) -> Result<usize> {
        usize::try_from(value).map_err(|_| Error::too_large())
    }

    fn checked_add(left: u32, right: u32) -> Result<u32> {
        left.checked_add(right).ok_or_else(Error::too_large)
    }

    fn rewind(len: usize, physical: u32) -> Result<usize> {
        len.checked_sub(Self::widen(physical)?)
            .ok_or_else(Error::too_large)
    }
}

impl Counts {
    fn new() -> Self {
        Self { logical: 0, physical: 0, expands: false, fan: false, pending: None }
    }
}

struct BufferMark {
    ops: usize,
    tokens: usize,
    bytes: usize,
}

impl BufferMark {
    fn restore(self, prog: &mut Program) {
        prog.ops.truncate(self.ops);
        prog.tokens.truncate(self.tokens);
        prog.bytes.truncate(self.bytes);
    }
}

/// A run of instructions on the tape.
#[derive(Clone, Copy)]
struct Tape {
    at: usize,
    count: u32,
}

impl Tape {
    fn new(at: usize, count: u32) -> Self {
        Self { at, count }
    }
}

/// One group instruction and the index where it sits.
struct GroupEmit {
    at: usize,
    delimiter: Delimiter,
    span: Span,
    body: u32,
}

impl GroupEmit {
    fn new(at: usize, delimiter: Delimiter, span: Span, body: u32) -> Self {
        Self { at, delimiter, span, body }
    }
}

/// Comma state while one list is written.
struct ListOut<'a> {
    ctx: &'a EvalCtx<'a>,
    emitted: &'a mut bool,
}

impl<'a> ListOut<'a> {
    fn new(ctx: &'a EvalCtx<'a>, emitted: &'a mut bool) -> Self {
        Self { ctx, emitted }
    }
}

/// A pasted name and the pattern that fills it.
struct NameEmit<'a> {
    base: &'a str,
    span: Span,
    pattern: Pattern,
}

impl<'a> NameEmit<'a> {
    fn new(base: &'a str, span: Span, pattern: Pattern) -> Self {
        Self { base, span, pattern }
    }
}

/// One row template and the index where its body starts.
struct RowHead {
    at: usize,
    span: Span,
    pattern: Pattern,
    body: u32,
}

impl RowHead {
    fn new(at: usize, span: Span, pattern: Pattern, body: u32) -> Self {
        Self { at, span, pattern, body }
    }
}

/// One instruction read by reference. An identifier is cloned only when it is written out.
enum Step {
    Copy {
        at: u32,
        len: u32,
    },
    Group {
        delimiter: Delimiter,
        span: Span,
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
    Piece,
}

/// A list piece. The identifier is owned because the output token keeps it.
enum Piece {
    Ident(Ident),
    Named {
        at: u32,
        len: u32,
        span: Span,
        pattern: Pattern,
    },
    Rows {
        span: Span,
        pattern: Pattern,
        body: u32,
    },
}

/// An output stream bound to the program it reads.
struct Emitting<'a> {
    out: &'a mut TokenStream,
    prog: &'a Program,
}

impl<'a> Emitting<'a> {
    fn new(out: &'a mut TokenStream, prog: &'a Program) -> Self {
        Self { out, prog }
    }

    fn emit_seq(&mut self, tape: Tape, ctx: &EvalCtx<'_>) -> Result<()> {
        let mut at = tape.at;
        for _ in 0..tape.count {
            let (width, step) = self.step_at(at)?;
            match step {
                Step::Copy { at: tok_at, len } => self.emit_copy(tok_at, len)?,
                Step::Group { delimiter, span, body } => {
                    let group = GroupEmit::new(at, delimiter, span, body);
                    self.emit_group(&group, ctx)?;
                }
                Step::Index { span } => {
                    self.out
                        .extend([TokenTree::Literal(ctx.index_literal(span))]);
                }
                Step::Paste { at: base_at, len, span } => {
                    self.emit_pasted(self.prog.text(base_at, len)?, ctx.index, span)?;
                }
                Step::List { pieces } => {
                    let body_at = at.checked_add(1).ok_or_else(Error::too_large)?;
                    self.emit_list(Tape::new(body_at, pieces), ctx)?;
                }
                Step::Piece => {
                    let mut emitted = false;
                    let mut out = ListOut::new(ctx, &mut emitted);
                    self.emit_piece(at, &mut out)?;
                }
            }
            at = at.checked_add(width).ok_or_else(Error::too_large)?;
        }
        Ok(())
    }

    /// Width and opcode of the instruction at `at`, without cloning it.
    fn step_at(&self, at: usize) -> Result<(usize, Step)> {
        let inst = self.prog.ops.get(at).ok_or_else(Error::too_large)?;
        let width = Program::widen(inst.width)?;
        if width == 0 {
            return Err(Error::too_large());
        }
        let step = match &inst.op {
            Op::Copy { at, len } => Step::Copy { at: *at, len: *len },
            Op::Group { delimiter, span, body } => {
                Step::Group { delimiter: *delimiter, span: *span, body: *body }
            }
            Op::Index { span } => Step::Index { span: *span },
            Op::Paste { at, len, span } => Step::Paste { at: *at, len: *len, span: *span },
            Op::List { pieces } => Step::List { pieces: *pieces },
            Op::Ident(_) | Op::Off(_) | Op::Named { .. } | Op::Rows { .. } => Step::Piece,
        };
        Ok((width, step))
    }

    fn emit_group(&mut self, group: &GroupEmit, ctx: &EvalCtx<'_>) -> Result<()> {
        let mut inner = TokenStream::new();
        let body_at = group.at.checked_add(1).ok_or_else(Error::too_large)?;
        let tape = Tape::new(body_at, group.body);
        inner.with_program(self.prog).emit_seq(tape, ctx)?;
        let mut built = Group::new(group.delimiter, inner);
        built.set_span(group.span);
        self.out.extend([TokenTree::Group(built)]);
        Ok(())
    }

    fn emit_list(&mut self, tape: Tape, ctx: &EvalCtx<'_>) -> Result<()> {
        let mut emitted = false;
        let mut out = ListOut::new(ctx, &mut emitted);
        let mut at = tape.at;
        for _ in 0..tape.count {
            let width = self
                .prog
                .ops
                .get(at)
                .map(|inst| inst.width)
                .ok_or_else(Error::too_large)?;
            let width = Program::widen(width)?;
            if width == 0 {
                return Err(Error::too_large());
            }
            self.emit_piece(at, &mut out)?;
            at = at.checked_add(width).ok_or_else(Error::too_large)?;
        }
        Ok(())
    }

    fn emit_piece(&mut self, at: usize, out: &mut ListOut<'_>) -> Result<()> {
        let piece = {
            let op = &self.prog.ops.get(at).ok_or_else(Error::too_large)?.op;
            match op {
                Op::Ident(ident) | Op::Off(ident) => Piece::Ident(ident.clone()),
                Op::Named { at, len, span, pattern } => {
                    Piece::Named { at: *at, len: *len, span: *span, pattern: *pattern }
                }
                Op::Rows { span, pattern, body } => {
                    Piece::Rows { span: *span, pattern: *pattern, body: *body }
                }
                Op::Copy { .. }
                | Op::Group { .. }
                | Op::Index { .. }
                | Op::Paste { .. }
                | Op::List { .. } => return Err(Error::too_large()),
            }
        };
        match piece {
            Piece::Ident(ident) => {
                self.emit_ident(out.emitted, ident);
                Ok(())
            }
            Piece::Named { at: base_at, len, span, pattern } => {
                let name = NameEmit::new(self.prog.text(base_at, len)?, span, pattern);
                self.emit_named(out, &name)
            }
            Piece::Rows { span, pattern, body } => {
                let body_at = at.checked_add(1).ok_or_else(Error::too_large)?;
                let row = RowHead::new(body_at, span, pattern, body);
                self.emit_rows(out, &row)
            }
        }
    }

    fn emit_copy(&mut self, at: u32, len: u32) -> Result<()> {
        let start = Program::widen(at)?;
        let end = start
            .checked_add(Program::widen(len)?)
            .ok_or_else(Error::too_large)?;
        let Some(tokens) = self.prog.tokens.get(start..end) else {
            return Err(Error::too_large());
        };
        self.out.extend(tokens.iter().cloned());
        Ok(())
    }

    fn push_comma(&mut self, emitted: &mut bool) {
        if *emitted {
            self.out
                .extend([TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
        }
        *emitted = true;
    }

    fn emit_pasted(&mut self, base: &str, index: i128, span: Span) -> Result<()> {
        let Ok(raw) = u128::try_from(index) else {
            return Err(Error::new(span, ErrorKind::NegativePaste));
        };
        let value = Decimal::new(raw);
        let mut stack = [0u8; Decimal::CAP];
        if let Some(text) = value.write(&mut stack, base) {
            self.out.extend([TokenTree::Ident(Ident::new(text, span))]);
            return Ok(());
        }
        let mut heap = String::with_capacity(base.len().saturating_add(value.len()));
        heap.push_str(base);
        value.push(&mut heap);
        self.out.extend([TokenTree::Ident(Ident::new(&heap, span))]);
        Ok(())
    }

    fn emit_ident(&mut self, emitted: &mut bool, ident: Ident) {
        self.push_comma(emitted);
        self.out.extend([TokenTree::Ident(ident)]);
    }

    fn emit_named(&mut self, out: &mut ListOut<'_>, name: &NameEmit<'_>) -> Result<()> {
        let ctx = *out.ctx;
        let range = name.pattern.to_range();
        range.visit(&ctx, |index| {
            self.push_comma(out.emitted);
            self.emit_pasted(name.base, index, name.span)
        })
    }

    fn emit_rows(&mut self, out: &mut ListOut<'_>, row: &RowHead) -> Result<()> {
        let ctx = *out.ctx;
        let domain = ctx.domain;
        let range = row.pattern.to_range();
        range.visit(&ctx, |index| {
            let mut inner = TokenStream::new();
            let tape = Tape::new(row.at, row.body);
            inner
                .with_program(self.prog)
                .emit_seq(tape, &EvalCtx::new(index, domain))?;
            self.push_comma(out.emitted);
            let mut group = Group::new(Delimiter::Parenthesis, inner);
            group.set_span(row.span);
            self.out.extend([TokenTree::Group(group)]);
            Ok(())
        })
    }
}

trait WithProgram {
    fn with_program<'a>(&'a mut self, prog: &'a Program) -> Emitting<'a>;
}

impl WithProgram for TokenStream {
    fn with_program<'a>(&'a mut self, prog: &'a Program) -> Emitting<'a> {
        Emitting::new(self, prog)
    }
}

/// Decimal digits of one non-negative index.
struct Decimal {
    value: u128,
}

impl Decimal {
    const RADIX: u128 = 10;
    /// `u128::MAX` is 39 digits.
    const MAX_DIGITS: usize = (u128::MAX.ilog10() + 1) as usize;
    /// Stack buffer that holds one pasted identifier.
    const CAP: usize = 128;
    const ZERO: u8 = b'0';
    const DIGITS: &[u8] = b"0123456789";

    fn new(value: u128) -> Self {
        Self { value }
    }

    fn len(&self) -> usize {
        if self.value == 0 {
            return 1;
        }
        match usize::try_from(self.value.ilog10()) {
            Ok(width) => width + 1,
            Err(_) => Self::MAX_DIGITS,
        }
    }

    fn write<'a>(&self, buf: &'a mut [u8], base: &str) -> Option<&'a str> {
        let digits = self.len();
        let len = base.len().checked_add(digits)?;
        if len > buf.len() {
            return None;
        }
        buf[..base.len()].copy_from_slice(base.as_bytes());
        self.write_digits(&mut buf[base.len()..len]);
        std::str::from_utf8(&buf[..len]).ok()
    }

    fn write_digits(&self, slot: &mut [u8]) {
        let mut value = self.value;
        if slot.is_empty() {
            return;
        }
        if value == 0 {
            slot[0] = Self::ZERO;
            return;
        }
        for byte in slot.iter_mut().rev() {
            *byte = Self::digit(value);
            value /= Self::RADIX;
        }
    }

    fn push(&self, out: &mut String) {
        let mut value = self.value;
        if value == 0 {
            out.push(char::from(Self::ZERO));
            return;
        }
        let mut digits = [0u8; Self::MAX_DIGITS];
        let mut count = 0;
        while value > 0 {
            digits[count] = Self::digit(value);
            value /= Self::RADIX;
            count += 1;
        }
        for index in (0..count).rev() {
            out.push(char::from(digits[index]));
        }
    }

    fn digit(value: u128) -> u8 {
        Self::DIGITS[(value % Self::RADIX) as usize]
    }
}

trait SourceTokens {
    fn is_exactly_hash(&self) -> bool;
    fn is_bare_hash(&self, index: usize) -> bool;
    fn is_hash(&self, index: usize) -> bool;
    fn hash_is_attribute(&self, hash_index: usize) -> bool;
    fn hash_span(&self, index: usize) -> Span;
    fn is_plus_at(&self, index: usize) -> bool;
    fn is_punct_at(&self, index: usize, mark: char) -> bool;
    fn is_ident_at(&self, index: usize) -> bool;
    fn is_paste_suffix(&self, index: usize) -> bool;
    fn spaced_paste_span(&self, index: usize) -> Option<Span>;
}

impl SourceTokens for [TokenTree] {
    fn is_exactly_hash(&self) -> bool {
        match self {
            [TokenTree::Punct(punct)] => punct.as_char() == '#',
            _ => false,
        }
    }

    fn is_bare_hash(&self, index: usize) -> bool {
        self.is_hash(index) && !self.hash_is_attribute(index)
    }

    fn is_hash(&self, index: usize) -> bool {
        self.is_punct_at(index, '#')
    }

    fn hash_is_attribute(&self, hash_index: usize) -> bool {
        match self.get(hash_index + 1) {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket => true,
            Some(TokenTree::Punct(punct)) if punct.as_char() == '!' => true,
            _ => false,
        }
    }

    fn hash_span(&self, index: usize) -> Span {
        self.get(index)
            .map_or_else(Span::call_site, TokenTree::span)
    }

    fn is_plus_at(&self, index: usize) -> bool {
        self.is_punct_at(index, '+')
    }

    fn is_punct_at(&self, index: usize, mark: char) -> bool {
        matches!(self.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == mark)
    }

    fn is_ident_at(&self, index: usize) -> bool {
        matches!(self.get(index), Some(TokenTree::Ident(_)))
    }

    fn is_paste_suffix(&self, index: usize) -> bool {
        self.is_plus_at(index + 1) && self.is_bare_hash(index + 2)
    }

    fn spaced_paste_span(&self, index: usize) -> Option<Span> {
        if self.is_ident_at(index) && self.is_bare_hash(index + 1) {
            Some(self.hash_span(index + 1))
        } else {
            None
        }
    }
}

fn paste_base(text: &str) -> &str {
    text.strip_prefix("r#").unwrap_or(text)
}

//! Placeholder substitution for one `impl_foreach` invocation.
//!
//! The body is compiled once into one instruction tape. A header is followed
//! by its body on that same tape, so a group does not allocate its own vector.
//! Each index then only substitutes values. Pasted identifiers are written into
//! a stack buffer. `$name = Ident #;` and `$name = #;` capture that substitution
//! for the rest of the enclosing group, and `$name` reads the captured token.
//! Flat mode repeats the body once per outer index and clips
//! patterns to that full index set. Fan mode is selected when a bracket pattern
//! mentions `#` (including `Name[#]` and `Off[#]`). Each fan copy still uses one
//! outer index, but the domain is the prefix `0..=index`.

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

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
    let mut env = BindEnv::with_capacity(Program::widen(program.bind_slots)?);
    if stat.fan {
        if outer.first() < 0 {
            return Err(Error::new(range.span, ErrorKind::FanEmpty));
        }
        outer.each(|index| {
            let domain = Domain::prefix(index);
            let tape = Tape::new(0, stat.logical);
            output
                .with_program(&program)
                .emit_seq(tape, &EvalCtx::new(index, &domain), &mut env)
        })?;
    } else {
        let domain = Domain::flat(&outer);
        outer.each(|index| {
            let tape = Tape::new(0, stat.logical);
            output
                .with_program(&program)
                .emit_seq(tape, &EvalCtx::new(index, &domain), &mut env)
        })?;
    }
    Ok(output)
}

const RANGE_ARG: &str = "range";
const BODY_ARG: &str = "body";

trait Call {
    fn split_call(self) -> Result<(Vec<TokenTree>, Vec<TokenTree>)>;
}

impl Call for TokenStream {
    fn split_call(self) -> Result<(Vec<TokenTree>, Vec<TokenTree>)> {
        let tokens: Vec<TokenTree> = self.into_iter().collect();
        let span = tokens.first().map_or_else(Span::call_site, TokenTree::span);
        if let Some(split) = tokens.unnamed_block(span)? {
            return Ok(split);
        }
        tokens.named_call(span)
    }
}

trait CallParts {
    fn unnamed_block(&self, span: Span) -> Result<Option<(Vec<TokenTree>, Vec<TokenTree>)>>;
    fn named_body_at(&self, brace_at: usize) -> bool;
    fn range_of_block(&self, span: Span) -> Result<Vec<TokenTree>>;
    fn named_call(&self, span: Span) -> Result<(Vec<TokenTree>, Vec<TokenTree>)>;
}

impl CallParts for [TokenTree] {
    /// A trailing `{ ... }` is the body. It needs no name. A comma before it is
    /// optional. `range = ...` still names the range when that name is written.
    fn unnamed_block(&self, span: Span) -> Result<Option<(Vec<TokenTree>, Vec<TokenTree>)>> {
        let Some(brace_at) = self.iter().rposition(OneToken::is_brace) else {
            return Ok(None);
        };
        if brace_at + 1 != self.len() {
            if self.named_body_at(brace_at) {
                return Ok(None);
            }
            let after = self.get(brace_at + 1).map_or(span, TokenTree::span);
            return Err(Error::new(after, ErrorKind::UnexpectedAfterBody));
        }
        if self.named_body_at(brace_at) {
            return Ok(None);
        }
        let mut head = self[..brace_at].to_vec();
        if head.last().is_some_and(OneToken::is_comma) {
            head.pop();
        }
        if head.is_empty() {
            return Err(Error::new(span, ErrorKind::ExpectedRange));
        }
        let range = head.range_of_block(span)?;
        let TokenTree::Group(group) = &self[brace_at] else {
            return Err(Error::new(span, ErrorKind::ExpectedBody));
        };
        Ok(Some((range, group.stream().into_iter().collect())))
    }

    fn named_body_at(&self, brace_at: usize) -> bool {
        brace_at >= 2
            && self
                .get(brace_at - 2)
                .is_some_and(|tree| matches!(tree, TokenTree::Ident(name) if name == BODY_ARG))
            && self.is_punct_at(brace_at - 1, '=')
    }

    fn range_of_block(&self, span: Span) -> Result<Vec<TokenTree>> {
        let named_range = self
            .first()
            .is_some_and(|tree| matches!(tree, TokenTree::Ident(name) if name == RANGE_ARG))
            && self.is_punct_at(1, '=');
        if named_range {
            let value = self[2..].to_vec();
            if value.is_empty() {
                return Err(Error::new(span, ErrorKind::ExpectedRange));
            }
            return Ok(value);
        }
        if self.is_ident_at(0) && self.is_punct_at(1, '=') {
            return Err(Error::new(span, ErrorKind::CallForm));
        }
        Ok(self.to_vec())
    }

    fn named_call(&self, span: Span) -> Result<(Vec<TokenTree>, Vec<TokenTree>)> {
        let Some(comma_at) = self.iter().position(OneToken::is_comma) else {
            return Err(Error::new(
                self.last().map_or(span, TokenTree::span),
                ErrorKind::ExpectedBody,
            ));
        };
        let (left_name, left_value) = self[..comma_at].named_value(span)?;
        let right_span = self.get(comma_at + 1).map_or(span, TokenTree::span);
        let (right_name, right_value) = self[comma_at + 1..].named_value(right_span)?;
        let (range, body) = match (left_name.as_str(), right_name.as_str()) {
            (RANGE_ARG, BODY_ARG) => (left_value, right_value),
            (BODY_ARG, RANGE_ARG) => (right_value, left_value),
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
    /// Binding names visible to later tokens. A nested group truncates this
    /// back to the length it started with.
    scope: Vec<NameRef>,
    /// `Bind` instructions on the tape. Emission reserves the binding stack once.
    bind_slots: u32,
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
    /// `$name = ...;` captures the current index and emits nothing.
    Bind {
        name_at: u32,
        name_len: u32,
        rhs: BindRhs,
    },
    /// `$name` after a binding in this group.
    Use {
        name_at: u32,
        name_len: u32,
        span: Span,
    },
}

/// Right-hand side of `$name = ...;`, before the index is known.
#[derive(Clone, Copy)]
enum BindRhs {
    /// `Ident #` or `Ident + #`.
    Paste { at: u32, len: u32, span: Span },
    /// `#`.
    Index,
}

/// The token captured when a binding runs.
#[derive(Clone, Copy)]
enum Captured {
    Paste { at: u32, len: u32, index: i128 },
    Index { value: i128 },
}

#[derive(Clone, Copy)]
struct Binding {
    name_at: u32,
    name_len: u32,
    value: Captured,
}

impl Binding {
    fn same_name(self, bytes: &[u8], at: u32, len: u32) -> bool {
        self.name_len == len && names_equal(bytes, self.name_at, at, len)
    }
}

/// Bindings captured while one index is emitted.
///
/// `expand` reserves one slot per binding instruction. Pushing during emission
/// does not allocate.
struct BindEnv {
    stack: Vec<Binding>,
}

impl BindEnv {
    fn with_capacity(slots: usize) -> Self {
        Self { stack: Vec::with_capacity(slots) }
    }

    fn len(&self) -> usize {
        self.stack.len()
    }

    fn truncate(&mut self, len: usize) {
        self.stack.truncate(len);
    }

    fn push(&mut self, binding: Binding) {
        debug_assert!(
            self.stack.len() < self.stack.capacity(),
            "index binding stack reserved too little capacity"
        );
        self.stack.push(binding);
    }

    /// Captures the current index. The binding statement emits no tokens.
    fn capture(&mut self, name_at: u32, name_len: u32, rhs: BindRhs, index: i128) -> Result<()> {
        let value = match rhs {
            BindRhs::Index => Captured::Index { value: index },
            BindRhs::Paste { at, len, span } => {
                if index < 0 {
                    return Err(Error::new(span, ErrorKind::NegativePaste));
                }
                Captured::Paste { at, len, index }
            }
        };
        self.push(Binding { name_at, name_len, value });
        Ok(())
    }

    fn find(&self, bytes: &[u8], at: u32, len: u32) -> Option<Captured> {
        self.stack
            .iter()
            .rev()
            .find(|binding| binding.same_name(bytes, at, len))
            .map(|binding| binding.value)
    }
}

/// A binding name packed into `Program::bytes`.
#[derive(Clone, Copy)]
struct NameRef {
    at: u32,
    len: u32,
}

impl NameRef {
    fn new(at: u32, len: u32) -> Self {
        Self { at, len }
    }
}

/// `$name` or `$name = ...;` read from the body.
struct DollarHead {
    name: String,
    span: Span,
    kind: DollarKind,
}

enum DollarKind {
    /// `$name` with no `=`. Substituted only when `name` is already bound.
    Mention,
    Bind(ParsedRhs),
}

struct ParsedRhs {
    next: usize,
    form: RhsForm,
}

enum RhsForm {
    Index,
    Paste { base: String, span: Span },
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
        Self {
            ops: Vec::new(),
            tokens: Vec::new(),
            bytes: Vec::new(),
            source: body,
            scope: Vec::new(),
            bind_slots: 0,
        }
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
        let mark = self.scope.len();
        let mut counts = Counts::new();
        let compiled = self.compile_range(&mut counts, source);
        self.scope.truncate(mark);
        compiled?;
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
    /// `$name = Ident #;`, `$name = Ident + #;`, or `$name = #;`, and a later `$name`.
    ///
    /// A `$name` that is not in scope is left unchanged, so a nested `macro_rules`
    /// metavariable stays in the output.
    fn compile_binding(
        &mut self,
        counts: &mut Counts,
        source: Source,
        index: usize,
    ) -> Result<Option<usize>> {
        let head = {
            let tokens = self.source_slice(source)?;
            read_dollar(tokens, index)?
        };
        let Some(head) = head else {
            return Ok(None);
        };
        match head.kind {
            DollarKind::Mention => self.compile_mention(counts, &head.name, head.span, index),
            DollarKind::Bind(rhs) => self.compile_defined(counts, &head.name, rhs),
        }
    }

    fn compile_mention(
        &mut self,
        counts: &mut Counts,
        name: &str,
        span: Span,
        index: usize,
    ) -> Result<Option<usize>> {
        if !self.scope_has(name) {
            return Ok(None);
        }
        let (name_at, name_len) = self.pack_str(name)?;
        self.flush_copy(counts)?;
        self.push_op(counts, Op::Use { name_at, name_len, span }, false)?;
        Ok(Some(index + 2))
    }

    fn compile_defined(
        &mut self,
        counts: &mut Counts,
        name: &str,
        rhs: ParsedRhs,
    ) -> Result<Option<usize>> {
        let next = rhs.next;
        let (name_at, name_len) = self.pack_str(name)?;
        self.scope.push(NameRef::new(name_at, name_len));
        let rhs = self.pack_rhs(rhs.form)?;
        self.flush_copy(counts)?;
        self.push_op(counts, Op::Bind { name_at, name_len, rhs }, false)?;
        self.bind_slots = Self::checked_add(self.bind_slots, 1)?;
        Ok(Some(next))
    }

    fn pack_rhs(&mut self, form: RhsForm) -> Result<BindRhs> {
        match form {
            RhsForm::Index => Ok(BindRhs::Index),
            RhsForm::Paste { base, span } => {
                let (at, len) = self.pack_str(&base)?;
                Ok(BindRhs::Paste { at, len, span })
            }
        }
    }

    fn scope_has(&self, name: &str) -> bool {
        let Ok(len) = u32::try_from(name.len()) else {
            return false;
        };
        let needle = name.as_bytes();
        self.scope
            .iter()
            .rev()
            .any(|entry| entry.len == len && byte_range(&self.bytes, entry.at, len) == Some(needle))
    }

    fn compile_range(&mut self, counts: &mut Counts, source: Source) -> Result<()> {
        let mut index = 0;
        while index < source.len {
            index = self.compile_one(counts, source, index)?;
        }
        self.flush_copy(counts)
    }
    fn compile_one(&mut self, counts: &mut Counts, source: Source, index: usize) -> Result<usize> {
        if let Some(next) = self.compile_binding(counts, source, index)? {
            return Ok(next);
        }
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
    Bind {
        name_at: u32,
        name_len: u32,
        rhs: BindRhs,
    },
    Use {
        name_at: u32,
        name_len: u32,
        span: Span,
    },
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

    /// Emits `tape`. Bindings pushed for this tape are dropped before return.
    fn emit_seq(&mut self, tape: Tape, ctx: &EvalCtx<'_>, env: &mut BindEnv) -> Result<()> {
        let mark = env.len();
        let emitted = self.emit_tape(tape, ctx, env);
        env.truncate(mark);
        emitted
    }

    fn emit_tape(&mut self, tape: Tape, ctx: &EvalCtx<'_>, env: &mut BindEnv) -> Result<()> {
        let mut at = tape.at;
        for _ in 0..tape.count {
            let (width, step) = self.step_at(at)?;
            match step {
                Step::Copy { at: tok_at, len } => self.emit_copy(tok_at, len)?,
                Step::Group { delimiter, span, body } => {
                    let group = GroupEmit::new(at, delimiter, span, body);
                    self.emit_group(&group, ctx, env)?;
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
                    self.emit_list(Tape::new(body_at, pieces), ctx, env)?;
                }
                Step::Piece => {
                    let mut emitted = false;
                    let mut out = ListOut::new(ctx, &mut emitted);
                    self.emit_piece(at, &mut out, env)?;
                }
                Step::Bind { name_at, name_len, rhs } => {
                    env.capture(name_at, name_len, rhs, ctx.index)?;
                }
                Step::Use { name_at, name_len, span } => {
                    self.emit_use(env, name_at, name_len, span)?;
                }
            }
            at = at.checked_add(width).ok_or_else(Error::too_large)?;
        }
        Ok(())
    }

    fn emit_use(&mut self, env: &BindEnv, name_at: u32, name_len: u32, span: Span) -> Result<()> {
        let Some(value) = env.find(&self.prog.bytes, name_at, name_len) else {
            return Err(Error::new(span, ErrorKind::UnknownBinding));
        };
        match value {
            Captured::Index { value } => {
                let mut literal = Literal::i128_unsuffixed(value);
                literal.set_span(span);
                self.out.extend([TokenTree::Literal(literal)]);
                Ok(())
            }
            Captured::Paste { at, len, index } => {
                self.emit_pasted(self.prog.text(at, len)?, index, span)
            }
        }
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
            Op::Bind { name_at, name_len, rhs } => {
                Step::Bind { name_at: *name_at, name_len: *name_len, rhs: *rhs }
            }
            Op::Use { name_at, name_len, span } => {
                Step::Use { name_at: *name_at, name_len: *name_len, span: *span }
            }
            Op::Ident(_) | Op::Off(_) | Op::Named { .. } | Op::Rows { .. } => Step::Piece,
        };
        Ok((width, step))
    }

    fn emit_group(
        &mut self,
        group: &GroupEmit,
        ctx: &EvalCtx<'_>,
        env: &mut BindEnv,
    ) -> Result<()> {
        let mut inner = TokenStream::new();
        let body_at = group.at.checked_add(1).ok_or_else(Error::too_large)?;
        let tape = Tape::new(body_at, group.body);
        inner.with_program(self.prog).emit_seq(tape, ctx, env)?;
        let mut built = Group::new(group.delimiter, inner);
        built.set_span(group.span);
        self.out.extend([TokenTree::Group(built)]);
        Ok(())
    }

    fn emit_list(&mut self, tape: Tape, ctx: &EvalCtx<'_>, env: &mut BindEnv) -> Result<()> {
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
            self.emit_piece(at, &mut out, env)?;
            at = at.checked_add(width).ok_or_else(Error::too_large)?;
        }
        Ok(())
    }

    fn emit_piece(&mut self, at: usize, out: &mut ListOut<'_>, env: &mut BindEnv) -> Result<()> {
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
                | Op::List { .. }
                | Op::Bind { .. }
                | Op::Use { .. } => return Err(Error::too_large()),
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
                self.emit_rows(out, &row, env)
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

    fn emit_rows(&mut self, out: &mut ListOut<'_>, row: &RowHead, env: &mut BindEnv) -> Result<()> {
        let ctx = *out.ctx;
        let domain = ctx.domain;
        let range = row.pattern.to_range();
        range.visit(&ctx, |index| {
            let mut inner = TokenStream::new();
            let tape = Tape::new(row.at, row.body);
            inner
                .with_program(self.prog)
                .emit_seq(tape, &EvalCtx::new(index, domain), env)?;
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

fn normalized_ident(name: &Ident) -> String {
    let text = name.to_string();
    match text.strip_prefix("r#") {
        Some(raw) => raw.to_string(),
        None => text,
    }
}

fn byte_range(bytes: &[u8], at: u32, len: u32) -> Option<&[u8]> {
    let start = usize::try_from(at).ok()?;
    let width = usize::try_from(len).ok()?;
    let end = start.checked_add(width)?;
    bytes.get(start..end)
}

fn names_equal(bytes: &[u8], left_at: u32, right_at: u32, len: u32) -> bool {
    match (
        byte_range(bytes, left_at, len),
        byte_range(bytes, right_at, len),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn read_dollar(tokens: &[TokenTree], index: usize) -> Result<Option<DollarHead>> {
    if !tokens.is_punct_at(index, '$') {
        return Ok(None);
    }
    let Some(TokenTree::Ident(ident)) = tokens.get(index + 1) else {
        return Ok(None);
    };
    let name = normalized_ident(ident);
    let span = ident.span();
    if !tokens.is_punct_at(index + 2, '=') {
        return Ok(Some(DollarHead { name, span, kind: DollarKind::Mention }));
    }
    let rhs = parse_binding_rhs(tokens, index)?;
    Ok(Some(DollarHead { name, span, kind: DollarKind::Bind(rhs) }))
}

fn parse_binding_rhs(tokens: &[TokenTree], index: usize) -> Result<ParsedRhs> {
    let rhs = index + 3;
    if let Some(parsed) = binding_index(tokens, rhs) {
        return Ok(parsed);
    }
    if let Some(parsed) = binding_paste(tokens, rhs) {
        return Ok(parsed);
    }
    Err(Error::new(
        binding_span(tokens, index),
        ErrorKind::BindingForm,
    ))
}

fn binding_index(tokens: &[TokenTree], rhs: usize) -> Option<ParsedRhs> {
    if tokens.is_bare_hash(rhs) && tokens.is_punct_at(rhs + 1, ';') {
        Some(ParsedRhs { next: rhs + 2, form: RhsForm::Index })
    } else {
        None
    }
}

fn binding_paste(tokens: &[TokenTree], rhs: usize) -> Option<ParsedRhs> {
    let TokenTree::Ident(ident) = tokens.get(rhs)? else {
        return None;
    };
    let base = normalized_ident(ident);
    let span = ident.span();
    if tokens.is_bare_hash(rhs + 1) && tokens.is_punct_at(rhs + 2, ';') {
        return Some(ParsedRhs { next: rhs + 3, form: RhsForm::Paste { base, span } });
    }
    if tokens.is_plus_at(rhs + 1)
        && tokens.is_bare_hash(rhs + 2)
        && tokens.is_punct_at(rhs + 3, ';')
    {
        return Some(ParsedRhs { next: rhs + 4, form: RhsForm::Paste { base, span } });
    }
    None
}

fn binding_span(tokens: &[TokenTree], index: usize) -> Span {
    tokens
        .get(index + 3)
        .or_else(|| tokens.get(index + 2))
        .map_or_else(Span::call_site, TokenTree::span)
}

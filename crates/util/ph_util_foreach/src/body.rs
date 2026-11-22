//! Placeholder substitution for one `impl_foreach` invocation.
//!
//! The body is parsed once into a recursive token tree. Each index then only
//! substitutes values. `#` is the index. `Ix + #` pastes that index onto an
//! identifier. `$name = ...` bindings are scoped to the group that contains
//! them. An unbound `$name` is copied through, so a nested `macro_rules`
//! metavariable still works.
//!
//! Flat mode repeats the body once per outer index and clips patterns to that
//! full index set. Fan mode is selected when a bracket pattern mentions `#`
//! (including `Name[#]` and `Off[#]`). Each fan copy still uses one outer
//! index, but the domain is the prefix `0..=index`.

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::error::{Error, ErrorKind, Result};
use crate::range::{Domain, EvalCtx, Pattern, RangeExpr};

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let (range_tokens, body) = input.split_call()?;
    let range = RangeExpr::parse(&range_tokens, crate::range::BoundStyle::Outer)?;
    let outer = range.compile_outer()?;
    if outer.is_empty() {
        return Ok(TokenStream::new());
    }
    let (seq, binds) = parse_body(&body)?;
    let mut output = TokenStream::new();
    let mut env = BindEnv::with_capacity(usize::try_from(binds).map_err(|_| Error::too_large())?);
    if seq.fan {
        if outer.first() < 0 {
            return Err(Error::new(range.span, ErrorKind::FanEmpty));
        }
        outer.each(|index| {
            let domain = Domain::prefix(index);
            emit(&seq, &EvalCtx::new(index, &domain), &mut env, &mut output)
        })?;
    } else {
        let domain = Domain::flat(&outer);
        outer.each(|index| emit(&seq, &EvalCtx::new(index, &domain), &mut env, &mut output))?;
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
    fn is_punct_at(&self, index: usize, mark: char) -> bool;
    fn is_ident_at(&self, index: usize) -> bool;
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

    fn is_punct_at(&self, index: usize, mark: char) -> bool {
        matches!(self.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == mark)
    }

    fn is_ident_at(&self, index: usize) -> bool {
        matches!(self.get(index), Some(TokenTree::Ident(_)))
    }
}

/// One parsed body, or the body of one group or row.
struct Seq {
    nodes: Vec<Node>,
    /// A bracket pattern in this tree mentions `#`.
    fan: bool,
    /// A placeholder, binding, or nested expanding group is present.
    expands: bool,
}

enum Node {
    /// Tokens copied unchanged, including groups that do not expand.
    Copy(Vec<TokenTree>),
    Group {
        delimiter: Delimiter,
        span: Span,
        body: Seq,
    },
    Index {
        span: Span,
    },
    Paste {
        base: String,
        span: Span,
    },
    List {
        pieces: Vec<Piece>,
    },
    Bind {
        name: String,
        rhs: Rhs,
    },
    Use {
        name: String,
        span: Span,
    },
}

/// Right-hand side of `$name = ...;`, before the index is known.
enum Rhs {
    /// `#`.
    Index,
    /// `Ident + #`.
    Paste { base: String, span: Span },
}

enum Piece {
    Ident {
        ident: Ident,
        /// `Off[#]` selects fan mode and emits the bare identifier.
        fan: bool,
    },
    Named {
        base: String,
        span: Span,
        pattern: Pattern,
    },
    Rows {
        span: Span,
        pattern: Pattern,
        body: Seq,
    },
}

enum Found {
    Named {
        raw: String,
        span: Span,
        pattern: Pattern,
    },
    Rows {
        span: Span,
        pattern: Pattern,
        inner: Vec<TokenTree>,
    },
}

impl Node {
    fn fan(&self) -> bool {
        match self {
            Self::List { pieces } => pieces.iter().any(Piece::is_fan),
            Self::Group { body, .. } => body.fan,
            Self::Copy(_)
            | Self::Index { .. }
            | Self::Paste { .. }
            | Self::Bind { .. }
            | Self::Use { .. } => false,
        }
    }

    fn expands(&self) -> bool {
        !matches!(self, Self::Copy(_))
    }
}

impl Piece {
    fn is_fan(&self) -> bool {
        match self {
            Self::Ident { fan, .. } => *fan,
            Self::Named { pattern, .. } => pattern.uses_index(),
            Self::Rows { pattern, body, .. } => pattern.uses_index() || body.fan,
        }
    }
}

fn parse_body(tokens: &[TokenTree]) -> Result<(Seq, u32)> {
    let mut scope = Vec::new();
    let mut binds = 0;
    let seq = parse_seq(tokens, &mut scope, &mut binds)?;
    Ok((seq, binds))
}

fn parse_seq(tokens: &[TokenTree], scope: &mut Vec<String>, binds: &mut u32) -> Result<Seq> {
    let mark = scope.len();
    let mut cursor = Cursor::new(tokens);
    let mut nodes = Vec::new();
    let mut fan = false;
    let mut expands = false;
    while !cursor.done() {
        let node = cursor.pull(scope, binds)?;
        fan |= node.fan();
        expands |= node.expands();
        push_node(&mut nodes, node);
    }
    scope.truncate(mark);
    Ok(Seq { nodes, fan, expands })
}

fn push_node(nodes: &mut Vec<Node>, node: Node) {
    match (nodes.last_mut(), node) {
        (Some(Node::Copy(buf)), Node::Copy(more)) => buf.extend(more),
        (_, node) => nodes.push(node),
    }
}

struct Cursor<'a> {
    tokens: &'a [TokenTree],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(tokens: &'a [TokenTree]) -> Self {
        Self { tokens, at: 0 }
    }

    fn done(&self) -> bool {
        self.at >= self.tokens.len()
    }

    fn peek(&self, offset: usize) -> Option<&'a TokenTree> {
        self.at
            .checked_add(offset)
            .and_then(|index| self.tokens.get(index))
    }

    fn advance(&mut self, count: usize) -> Result<()> {
        self.at = self.at.checked_add(count).ok_or_else(Error::too_large)?;
        Ok(())
    }

    fn is_punct(&self, offset: usize, mark: char) -> bool {
        matches!(self.peek(offset), Some(TokenTree::Punct(punct)) if punct.as_char() == mark)
    }

    fn is_bare_hash(&self, offset: usize) -> bool {
        self.is_punct(offset, '#') && !self.attribute_hash(offset)
    }

    fn attribute_hash(&self, offset: usize) -> bool {
        match self.peek(offset + 1) {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket => true,
            Some(TokenTree::Punct(punct)) if punct.as_char() == '!' => true,
            _ => false,
        }
    }

    fn pull(&mut self, scope: &mut Vec<String>, binds: &mut u32) -> Result<Node> {
        if let Some(node) = self.pull_binding(scope, binds)? {
            return Ok(node);
        }
        if let Some(node) = self.pull_bracket_list(scope, binds)? {
            return Ok(node);
        }
        if let Some(node) = self.pull_paste()? {
            return Ok(node);
        }
        self.reject_spaced_paste()?;
        if let Some(node) = self.pull_prefixed(scope, binds)? {
            return Ok(node);
        }
        if let Some(node) = self.pull_index()? {
            return Ok(node);
        }
        if let Some(node) = self.pull_group(scope, binds)? {
            return Ok(node);
        }
        self.pull_verbatim()
    }

    fn pull_binding(&mut self, scope: &mut Vec<String>, binds: &mut u32) -> Result<Option<Node>> {
        if !self.is_punct(0, '$') {
            return Ok(None);
        }
        let Some(TokenTree::Ident(ident)) = self.peek(1) else {
            return Ok(None);
        };
        let name = normalized(ident);
        let span = ident.span();
        if self.is_punct(2, '=') {
            let rhs = self.binding_rhs()?;
            scope.push(name.clone());
            *binds = binds.checked_add(1).ok_or_else(Error::too_large)?;
            return Ok(Some(Node::Bind { name, rhs }));
        }
        if !scope.iter().rev().any(|bound| bound == &name) {
            return Ok(None);
        }
        self.advance(2)?;
        Ok(Some(Node::Use { name, span }))
    }

    fn binding_rhs(&mut self) -> Result<Rhs> {
        if self.is_bare_hash(3) && self.is_punct(4, ';') {
            self.advance(5)?;
            return Ok(Rhs::Index);
        }
        if let Some(TokenTree::Ident(ident)) = self.peek(3).cloned()
            && self.is_punct(4, '+')
            && self.is_bare_hash(5)
            && self.is_punct(6, ';')
        {
            let base = normalized(&ident);
            let span = ident.span();
            self.advance(7)?;
            return Ok(Rhs::Paste { base, span });
        }
        let span = self
            .peek(3)
            .or_else(|| self.peek(2))
            .map_or_else(Span::call_site, TokenTree::span);
        Err(Error::new(span, ErrorKind::BindingForm))
    }

    fn pull_paste(&mut self) -> Result<Option<Node>> {
        let Some(TokenTree::Ident(ident)) = self.peek(0) else {
            return Ok(None);
        };
        if !(self.is_punct(1, '+') && self.is_bare_hash(2)) {
            return Ok(None);
        }
        let base = paste_base(&ident.to_string()).to_string();
        let span = ident.span();
        self.advance(3)?;
        Ok(Some(Node::Paste { base, span }))
    }

    fn reject_spaced_paste(&self) -> Result<()> {
        if self
            .peek(0)
            .is_some_and(|token| matches!(token, TokenTree::Ident(_)))
            && self.is_bare_hash(1)
        {
            let span = self.peek(1).map_or_else(Span::call_site, TokenTree::span);
            return Err(Error::new(span, ErrorKind::SpacedPaste));
        }
        Ok(())
    }

    fn pull_index(&mut self) -> Result<Option<Node>> {
        if !self.is_bare_hash(0) {
            return Ok(None);
        }
        let span = self.peek(0).map_or_else(Span::call_site, TokenTree::span);
        self.advance(1)?;
        Ok(Some(Node::Index { span }))
    }

    fn pull_group(&mut self, scope: &mut Vec<String>, binds: &mut u32) -> Result<Option<Node>> {
        let Some((delimiter, span, original, inner)) = self.group_at(0) else {
            return Ok(None);
        };
        let body = parse_seq(&inner, scope, binds)?;
        self.advance(1)?;
        if body.expands {
            Ok(Some(Node::Group { delimiter, span, body }))
        } else {
            Ok(Some(Node::Copy(vec![TokenTree::Group(original)])))
        }
    }

    fn group_at(&self, offset: usize) -> Option<(Delimiter, Span, Group, Vec<TokenTree>)> {
        match self.peek(offset) {
            Some(TokenTree::Group(group)) => Some((
                group.delimiter(),
                group.span(),
                group.clone(),
                group.stream().into_iter().collect(),
            )),
            _ => None,
        }
    }

    fn pull_verbatim(&mut self) -> Result<Node> {
        let token = self.peek(0).cloned().ok_or_else(Error::too_large)?;
        self.advance(1)?;
        Ok(Node::Copy(vec![token]))
    }

    fn pull_bracket_list(
        &mut self,
        scope: &mut Vec<String>,
        binds: &mut u32,
    ) -> Result<Option<Node>> {
        if self.pattern_at(0)?.is_none() {
            return Ok(None);
        }
        self.collect_list(Vec::new(), scope, binds).map(Some)
    }

    fn pull_prefixed(&mut self, scope: &mut Vec<String>, binds: &mut u32) -> Result<Option<Node>> {
        let Some(TokenTree::Ident(ident)) = self.peek(0).cloned() else {
            return Ok(None);
        };
        if self.is_punct(1, '+') {
            if self.pattern_at(2)?.is_none() {
                return Ok(None);
            }
            self.advance(2)?;
            let prefix = Piece::Ident { ident, fan: false };
            return self.collect_list(vec![prefix], scope, binds).map(Some);
        }
        if self.pattern_at(1)?.is_some() {
            return Err(Error::new(ident.span(), ErrorKind::NotConcat));
        }
        Ok(None)
    }

    fn collect_list(
        &mut self,
        mut pieces: Vec<Piece>,
        scope: &mut Vec<String>,
        binds: &mut u32,
    ) -> Result<Node> {
        let started = pieces.len();
        loop {
            let found = self.take_piece()?;
            let Some(found) = found else {
                break;
            };
            pieces.push(lower(found, scope, binds)?);
            if self.is_punct(0, '+') && self.pattern_at(1)?.is_some() {
                self.advance(1)?;
                continue;
            }
            break;
        }
        if pieces.len() == started {
            return Err(Error::too_large());
        }
        Ok(Node::List { pieces })
    }

    fn take_piece(&mut self) -> Result<Option<Found>> {
        let Some(pattern) = self.pattern_at(0)? else {
            return Ok(None);
        };
        let head = self.peek(0).cloned().ok_or_else(Error::too_large)?;
        let found = match head {
            TokenTree::Ident(name) => {
                Found::Named { raw: name.to_string(), span: name.span(), pattern }
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => Found::Rows {
                span: group.span(),
                pattern,
                inner: group.stream().into_iter().collect(),
            },
            _ => return Err(Error::too_large()),
        };
        self.advance(2)?;
        Ok(Some(found))
    }

    fn pattern_at(&self, offset: usize) -> Result<Option<Pattern>> {
        let Some(head) = self.peek(offset) else {
            return Ok(None);
        };
        if !head.opens_piece() {
            return Ok(None);
        }
        let Some(TokenTree::Group(group)) = self.peek(offset + 1) else {
            return Ok(None);
        };
        if group.delimiter() != Delimiter::Bracket {
            return Ok(None);
        }
        classify(group.stream(), group.span())
    }
}

fn lower(found: Found, scope: &mut Vec<String>, binds: &mut u32) -> Result<Piece> {
    match found {
        Found::Named { raw, span, pattern } => {
            if pattern.is_off(&raw) {
                return Ok(Piece::Ident { ident: Ident::new("Off", span), fan: true });
            }
            Ok(Piece::Named { base: paste_base(&raw).to_string(), span, pattern })
        }
        Found::Rows { span, pattern, inner } => {
            let body = parse_seq(&inner, scope, binds)?;
            Ok(Piece::Rows { span, pattern, body })
        }
    }
}

fn classify(stream: TokenStream, span: Span) -> Result<Option<Pattern>> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    if is_only_hash(&tokens) {
        return Ok(Some(Pattern::Shorthand { span }));
    }
    match RangeExpr::parse_pattern(&tokens)? {
        Some(mut range) => {
            range.span = span;
            Ok(Some(Pattern::Range(range)))
        }
        None => Ok(None),
    }
}

fn is_only_hash(tokens: &[TokenTree]) -> bool {
    matches!(tokens, [TokenTree::Punct(punct)] if punct.as_char() == '#')
}

/// Bindings captured while one index is emitted.
///
/// `expand` reserves one slot per binding. Pushing during emission does not
/// allocate.
struct BindEnv<'a> {
    stack: Vec<Captured<'a>>,
}

enum Captured<'a> {
    Index {
        name: &'a str,
        value: i128,
    },
    Paste {
        name: &'a str,
        base: &'a str,
        index: i128,
    },
}

impl<'a> BindEnv<'a> {
    fn with_capacity(slots: usize) -> Self {
        Self { stack: Vec::with_capacity(slots) }
    }

    fn capture(&mut self, name: &'a str, rhs: &'a Rhs, index: i128) -> Result<()> {
        let value = match rhs {
            Rhs::Index => Captured::Index { name, value: index },
            Rhs::Paste { base, span } => {
                if index < 0 {
                    return Err(Error::new(*span, ErrorKind::NegativePaste));
                }
                Captured::Paste { name, base, index }
            }
        };
        debug_assert!(
            self.stack.len() < self.stack.capacity(),
            "index binding stack reserved too little capacity"
        );
        self.stack.push(value);
        Ok(())
    }

    fn find(&self, name: &str) -> Option<&Captured<'a>> {
        self.stack
            .iter()
            .rev()
            .find(|captured| captured.name() == name)
    }
}

impl Captured<'_> {
    fn name(&self) -> &str {
        match self {
            Self::Index { name, .. } | Self::Paste { name, .. } => name,
        }
    }
}

fn emit<'a>(
    seq: &'a Seq,
    ctx: &EvalCtx<'_>,
    env: &mut BindEnv<'a>,
    out: &mut TokenStream,
) -> Result<()> {
    let mark = env.stack.len();
    let emitted = emit_nodes(&seq.nodes, ctx, env, out);
    env.stack.truncate(mark);
    emitted
}

fn emit_nodes<'a>(
    nodes: &'a [Node],
    ctx: &EvalCtx<'_>,
    env: &mut BindEnv<'a>,
    out: &mut TokenStream,
) -> Result<()> {
    for node in nodes {
        match node {
            Node::Copy(tokens) => out.extend(tokens.iter().cloned()),
            Node::Index { span } => push_index(out, ctx.index, *span),
            Node::Paste { base, span } => emit_pasted(out, base, ctx.index, *span)?,
            Node::Group { delimiter, span, body } => {
                let mut inner = TokenStream::new();
                emit(body, ctx, env, &mut inner)?;
                let mut group = Group::new(*delimiter, inner);
                group.set_span(*span);
                out.extend([TokenTree::Group(group)]);
            }
            Node::List { pieces } => emit_list(pieces, ctx, env, out)?,
            Node::Bind { name, rhs } => env.capture(name, rhs, ctx.index)?,
            Node::Use { name, span } => emit_use(env, out, name, *span)?,
        }
    }
    Ok(())
}

fn emit_use(env: &BindEnv<'_>, out: &mut TokenStream, name: &str, span: Span) -> Result<()> {
    let Some(value) = env.find(name) else {
        return Err(Error::new(span, ErrorKind::UnknownBinding));
    };
    match value {
        Captured::Index { value, .. } => {
            push_index(out, *value, span);
            Ok(())
        }
        Captured::Paste { base, index, .. } => emit_pasted(out, base, *index, span),
    }
}

fn emit_list<'a>(
    pieces: &'a [Piece],
    ctx: &EvalCtx<'_>,
    env: &mut BindEnv<'a>,
    out: &mut TokenStream,
) -> Result<()> {
    let mut emitted = false;
    for piece in pieces {
        match piece {
            Piece::Ident { ident, .. } => {
                push_comma(out, &mut emitted);
                out.extend([TokenTree::Ident(ident.clone())]);
            }
            Piece::Named { base, span, pattern } => {
                pattern.to_range().visit(ctx, |index| {
                    push_comma(out, &mut emitted);
                    emit_pasted(out, base, index, *span)
                })?;
            }
            Piece::Rows { span, pattern, body } => {
                let domain = *ctx.domain;
                pattern.to_range().visit(ctx, |index| {
                    let mut inner = TokenStream::new();
                    emit(body, &EvalCtx::new(index, &domain), env, &mut inner)?;
                    push_comma(out, &mut emitted);
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

fn push_index(out: &mut TokenStream, value: i128, span: Span) {
    let mut literal = Literal::i128_unsuffixed(value);
    literal.set_span(span);
    out.extend([TokenTree::Literal(literal)]);
}

fn push_comma(out: &mut TokenStream, emitted: &mut bool) {
    if *emitted {
        out.extend([TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
    }
    *emitted = true;
}

fn emit_pasted(out: &mut TokenStream, base: &str, index: i128, span: Span) -> Result<()> {
    let Ok(raw) = u128::try_from(index) else {
        return Err(Error::new(span, ErrorKind::NegativePaste));
    };
    let value = Decimal::new(raw);
    let mut stack = [0u8; Decimal::CAP];
    if let Some(text) = value.write(&mut stack, base) {
        out.extend([TokenTree::Ident(Ident::new(text, span))]);
        return Ok(());
    }
    let mut heap = String::with_capacity(base.len().saturating_add(value.len()));
    heap.push_str(base);
    value.push(&mut heap);
    out.extend([TokenTree::Ident(Ident::new(&heap, span))]);
    Ok(())
}

fn normalized(ident: &Ident) -> String {
    let text = ident.to_string();
    match text.strip_prefix("r#") {
        Some(raw) => raw.to_string(),
        None => text,
    }
}

fn paste_base(text: &str) -> &str {
    text.strip_prefix("r#").unwrap_or(text)
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

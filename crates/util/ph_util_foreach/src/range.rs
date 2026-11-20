//! Range syntax, domain checks, and index lists.
//!
//! Outer ranges and `Name[<pat>]` patterns share one parser. The outer range
//! must already be finite. A pattern may use open ends, `#`, and the literal `-1`.

use proc_macro::{Literal, Punct, Spacing, Span, TokenTree};

use crate::error::{Error, Result};

/// How a bound is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundStyle {
    /// Outer range: integer literals only, and `-1` is the integer negative one.
    Outer,
    /// Pattern bound: `#`, `#±N`, and the literal `-1` (last domain index).
    Pattern,
    /// Step: positive integer literal or `#±N`. The literal `-1` stays negative.
    Step,
}

/// A bound before it is evaluated.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Bound {
    /// An integer literal, already signed.
    Int(i128),
    /// The literal `-1` in a pattern: last index of the domain.
    LastIndex,
    /// `#` plus `offset` (`#-1` is `Here { offset: -1 }`).
    Here { offset: i128 },
}

/// A bound and the span of the token that introduced it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SpannedBound {
    pub(crate) bound: Bound,
    pub(crate) span: Span,
}

/// One range, including an optional step.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RangeExpr {
    pub(crate) start: Option<SpannedBound>,
    pub(crate) end: Option<SpannedBound>,
    pub(crate) end_inclusive: bool,
    pub(crate) step: Option<SpannedBound>,
    pub(crate) span: Span,
}

/// A bracket pattern.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Pattern {
    /// `Name[#]`, defined as `Name[0..=#]`.
    Shorthand { span: Span },
    /// An explicit range.
    Range(RangeExpr),
}

impl Pattern {
    /// Whether a bound of this pattern mentions `#`.
    ///
    /// A step of `#` does not count: the domain stays the flat outer set, and
    /// only the stride changes with the index.
    pub(crate) fn uses_index(self) -> bool {
        match self {
            Self::Shorthand { .. } => true,
            Self::Range(range) => bound_uses_index(range.start) || bound_uses_index(range.end),
        }
    }

    /// The range this pattern selects. `Name[#]` is `0..=#`.
    pub(crate) fn to_range(self) -> RangeExpr {
        match self {
            Self::Shorthand { span } => shorthand_range(span),
            Self::Range(range) => range,
        }
    }
}

fn shorthand_range(span: Span) -> RangeExpr {
    RangeExpr {
        start: Some(SpannedBound {
            bound: Bound::Int(0),
            span,
        }),
        end: Some(SpannedBound {
            bound: Bound::Here { offset: 0 },
            span,
        }),
        end_inclusive: true,
        step: None,
        span,
    }
}

fn bound_uses_index(bound: Option<SpannedBound>) -> bool {
    matches!(
        bound,
        Some(SpannedBound {
            bound: Bound::Here { .. },
            ..
        })
    )
}

/// The indices a pattern is clipped to.
#[derive(Clone, Debug)]
pub(crate) struct Domain {
    lo: i128,
    hi: i128,
    /// Outer step. An exclusive end may land up to this far past `hi`.
    gap: i128,
    /// `true` when every integer from `lo` through `hi` is in the domain.
    dense: bool,
    /// Flat domains own the outer index list. Fan prefixes leave this empty and
    /// use `lo` and `hi` instead, so a fan iteration does not allocate.
    indices: Vec<i128>,
}

impl Domain {
    /// Domain of a fan iteration: the prefix `0..=outer`.
    pub(crate) fn fan(outer: i128, span: Span) -> Result<Self> {
        if outer < 0 {
            return Err(Error::new(
                span,
                "fan mode domain `0..=index` is empty because the index is negative",
            ));
        }
        Ok(Self {
            lo: 0,
            hi: outer,
            gap: 1,
            dense: true,
            indices: Vec::new(),
        })
    }

    /// Domain of a flat iteration. Takes ownership of the outer index list.
    pub(crate) fn from_indices(indices: Vec<i128>, gap: i128) -> Result<Self> {
        let (Some(&lo), Some(&hi)) = (indices.first(), indices.last()) else {
            return Err(Error::new(
                Span::call_site(),
                "internal error: empty domain",
            ));
        };
        let dense = gap == 1 && is_contiguous(&indices, lo, hi);
        Ok(Self {
            lo,
            hi,
            gap,
            dense,
            indices,
        })
    }

    /// Number of indices in a flat domain.
    pub(crate) fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// Index at `position` in a flat domain.
    pub(crate) fn index_at(&self, position: usize) -> i128 {
        self.indices[position]
    }

    fn contains(&self, value: i128) -> bool {
        if self.dense {
            self.lo <= value && value <= self.hi
        } else {
            self.indices.binary_search(&value).is_ok()
        }
    }

    fn inclusive_ok(&self, value: i128) -> bool {
        self.lo <= value && value <= self.hi
    }

    fn exclusive_ok(&self, value: i128) -> bool {
        match self.hi.checked_add(self.gap) {
            Some(limit) => self.lo <= value && value <= limit,
            None => self.lo <= value,
        }
    }
}

fn is_contiguous(indices: &[i128], lo: i128, hi: i128) -> bool {
    let Some(span) = hi.checked_sub(lo).and_then(|diff| diff.checked_add(1)) else {
        return false;
    };
    span == i128::try_from(indices.len()).unwrap_or(i128::MAX)
        && indices
            .windows(2)
            .all(|pair| pair[1] == pair[0].saturating_add(1))
}

/// Values substituted while a pattern is resolved.
#[derive(Clone, Copy)]
pub(crate) struct EvalCtx<'a> {
    /// Current `#`.
    pub(crate) index: i128,
    pub(crate) domain: &'a Domain,
}

/// Finite index list of an outer range, plus the step stored as the domain gap.
#[derive(Clone, Debug)]
pub(crate) struct OuterRange {
    pub(crate) indices: Vec<i128>,
    pub(crate) gap: i128,
}

/// Parses `tokens` as one range. `style` decides what `-1` and `#` mean.
pub(crate) fn parse_range(tokens: &[TokenTree], style: BoundStyle) -> Result<RangeExpr> {
    if tokens.is_empty() {
        return Err(Error::new(Span::call_site(), "expected a range"));
    }
    let mut cursor = Cursor::new(tokens);
    let start = cursor.try_bound(style)?;
    if start.is_some() && !cursor.starts_with_range_op() {
        return Err(Error::new(cursor.error_span(), "expected `..` or `..=`"));
    }
    finish_range(&mut cursor, start, style, tokens)
}

/// Parses a bracket pattern in one pass.
///
/// `None` means the brackets do not open with a range, so they stay ordinary
/// Rust. A broken bound such as `#+` is an error.
pub(crate) fn parse_pattern(tokens: &[TokenTree]) -> Result<Option<RangeExpr>> {
    if tokens.is_empty() {
        return Ok(None);
    }
    let mut cursor = Cursor::new(tokens);
    let start = if cursor.starts_with_range_op() {
        None
    } else {
        match cursor.try_bound(BoundStyle::Pattern)? {
            Some(bound) if cursor.starts_with_range_op() => Some(bound),
            Some(_) | None => return Ok(None),
        }
    };
    finish_range(&mut cursor, start, BoundStyle::Pattern, tokens).map(Some)
}

fn finish_range(
    cursor: &mut Cursor<'_>,
    start: Option<SpannedBound>,
    style: BoundStyle,
    tokens: &[TokenTree],
) -> Result<RangeExpr> {
    let end_inclusive = cursor.parse_range_op()?;
    let end = if cursor.starts_with_pathsep() {
        None
    } else {
        cursor.try_bound(style)?
    };
    let step = if cursor.starts_with_pathsep() {
        cursor.parse_pathsep()?;
        match cursor.try_bound(BoundStyle::Step)? {
            Some(bound) => Some(bound),
            None => {
                return Err(Error::new(
                    cursor.error_span(),
                    "expected a step after `::`",
                ));
            }
        }
    } else {
        None
    };
    if !cursor.is_empty() {
        let message = if style == BoundStyle::Pattern {
            "expected exactly one range pattern inside `[]`"
        } else {
            "unexpected token in range"
        };
        return Err(Error::new(cursor.error_span(), message));
    }
    Ok(RangeExpr {
        start,
        end,
        end_inclusive,
        step,
        span: tokens[0].span(),
    })
}

/// Indices selected by a finite outer range.
pub(crate) fn compile_outer(range: &RangeExpr) -> Result<OuterRange> {
    let (Some(start_bound), Some(end_bound)) = (range.start, range.end) else {
        return Err(Error::new(
            range.span,
            "open outer range is unbounded; write `A..B` or `A..=B`, optionally with a `::S` step",
        ));
    };
    let start = expect_outer_int(start_bound)?;
    let end = expect_outer_int(end_bound)?;
    let gap = match range.step {
        None => 1,
        Some(step) => expect_positive_int_step(step)?,
    };
    reject_inverted(start, end, range.end_inclusive, range.span)?;
    let indices = generate(start, end, range.end_inclusive, gap, None);
    Ok(OuterRange { indices, gap })
}

/// Calls `visit` for each index `range` selects inside `ctx.domain`.
///
/// The walk does not allocate. An empty hole-edge window visits nothing.
pub(crate) fn visit_range(
    range: &RangeExpr,
    ctx: &EvalCtx<'_>,
    mut visit: impl FnMut(i128) -> Result<()>,
) -> Result<()> {
    let step = eval_step(range.step, ctx)?;
    let start = match range.start {
        Some(bound) => eval_pattern_bound(bound, ctx)?,
        None => ctx.domain.lo,
    };
    let (end, end_inclusive) = match range.end {
        Some(bound) => (eval_pattern_bound(bound, ctx)?, range.end_inclusive),
        None => (ctx.domain.hi, true),
    };
    // `T[#+1..=#]` when `#` is the last index: the window is empty and the
    // start sits one past the domain. That hole edge is allowed.
    if is_high_hole_edge(start, end, end_inclusive, ctx.domain) {
        return Ok(());
    }
    reject_inverted(start, end, end_inclusive, range.span)?;
    if !ctx.domain.inclusive_ok(start) {
        return Err(outside(range.span, start, ctx.domain));
    }
    if end_inclusive {
        if !ctx.domain.inclusive_ok(end) {
            return Err(outside(range.span, end, ctx.domain));
        }
    } else if !ctx.domain.exclusive_ok(end) {
        return Err(outside(range.span, end, ctx.domain));
    }
    walk(
        start,
        end,
        end_inclusive,
        step,
        Some(ctx.domain),
        &mut visit,
    )
}

fn is_high_hole_edge(start: i128, end: i128, end_inclusive: bool, domain: &Domain) -> bool {
    let Some(just_past) = domain.hi.checked_add(1) else {
        return false;
    };
    end_inclusive && end == domain.hi && start == just_past
}

fn reject_inverted(start: i128, end: i128, end_inclusive: bool, span: Span) -> Result<()> {
    if start > end {
        let rendered = if end_inclusive {
            format!("{start}..={end}")
        } else {
            format!("{start}..{end}")
        };
        return Err(Error::new(span, format!("invalid range {rendered}")));
    }
    Ok(())
}

fn outside(span: Span, value: i128, domain: &Domain) -> Error {
    Error::new(
        span,
        format!(
            "bound {value} is outside the domain {}..={}",
            domain.lo, domain.hi
        ),
    )
}

fn expect_outer_int(bound: SpannedBound) -> Result<i128> {
    match bound.bound {
        Bound::Int(value) => Ok(value),
        Bound::Here { .. } | Bound::LastIndex => {
            Err(Error::new(bound.span, "outer range cannot use `#`"))
        }
    }
}

fn expect_positive_int_step(bound: SpannedBound) -> Result<i128> {
    match bound.bound {
        Bound::Int(value) if value > 0 => Ok(value),
        Bound::Int(_) => Err(Error::new(bound.span, "step must be a positive integer")),
        Bound::Here { .. } | Bound::LastIndex => {
            Err(Error::new(bound.span, "outer range cannot use `#`"))
        }
    }
}

fn eval_step(step: Option<SpannedBound>, ctx: &EvalCtx<'_>) -> Result<i128> {
    let Some(step) = step else {
        return Ok(1);
    };
    let value = match step.bound {
        Bound::Int(value) => value,
        Bound::Here { offset } => add_offset(ctx.index, offset, step.span)?,
        Bound::LastIndex => {
            return Err(Error::new(
                step.span,
                "step must be a positive integer literal or `#±N`",
            ));
        }
    };
    if value <= 0 {
        return Err(Error::new(step.span, "step must be a positive integer"));
    }
    Ok(value)
}

fn eval_pattern_bound(bound: SpannedBound, ctx: &EvalCtx<'_>) -> Result<i128> {
    match bound.bound {
        Bound::Int(value) => Ok(value),
        Bound::LastIndex => Ok(ctx.domain.hi),
        Bound::Here { offset } => add_offset(ctx.index, offset, bound.span),
    }
}

fn add_offset(index: i128, offset: i128, span: Span) -> Result<i128> {
    index
        .checked_add(offset)
        .ok_or_else(|| Error::new(span, "index overflow computing a `#` bound"))
}

fn generate(
    start: i128,
    end: i128,
    end_inclusive: bool,
    step: i128,
    domain: Option<&Domain>,
) -> Vec<i128> {
    let mut out = Vec::with_capacity(step_len(start, end, end_inclusive, step));
    let mut current = start;
    loop {
        if past_end(current, end, end_inclusive) {
            break;
        }
        if domain.is_none_or(|domain| domain.contains(current)) {
            out.push(current);
        }
        let Some(next) = advance(current, step) else {
            break;
        };
        current = next;
    }
    out
}

fn walk(
    start: i128,
    end: i128,
    end_inclusive: bool,
    step: i128,
    domain: Option<&Domain>,
    visit: &mut impl FnMut(i128) -> Result<()>,
) -> Result<()> {
    let mut current = start;
    loop {
        if past_end(current, end, end_inclusive) {
            break;
        }
        if domain.is_none_or(|domain| domain.contains(current)) {
            visit(current)?;
        }
        let Some(next) = advance(current, step) else {
            break;
        };
        current = next;
    }
    Ok(())
}

fn past_end(current: i128, end: i128, end_inclusive: bool) -> bool {
    if end_inclusive {
        current > end
    } else {
        current >= end
    }
}

fn advance(current: i128, step: i128) -> Option<i128> {
    match current.checked_add(step) {
        Some(next) if next > current => Some(next),
        _ => None,
    }
}

/// How many indices `start`, `start + step`, ... yields before `end`.
fn step_len(start: i128, end: i128, end_inclusive: bool, step: i128) -> usize {
    if step <= 0 {
        return 0;
    }
    let Some(span) = (if end_inclusive {
        end.checked_sub(start)
    } else {
        end.checked_sub(start)
            .and_then(|width| width.checked_sub(1))
    }) else {
        return 0;
    };
    if span < 0 {
        return 0;
    }
    let steps = span / step;
    usize::try_from(steps.saturating_add(1)).unwrap_or(0)
}

struct Cursor<'a> {
    tokens: &'a [TokenTree],
    index: usize,
}

impl<'a> Cursor<'a> {
    fn new(tokens: &'a [TokenTree]) -> Self {
        Self { tokens, index: 0 }
    }

    fn is_empty(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn error_span(&self) -> Span {
        self.tokens
            .get(self.index)
            .map_or_else(Span::call_site, TokenTree::span)
    }

    fn punct(&self, offset: usize) -> Option<&'a Punct> {
        match self.tokens.get(self.index + offset) {
            Some(TokenTree::Punct(punct)) => Some(punct),
            _ => None,
        }
    }

    fn starts_with_joint(&self, first: char, second: char) -> bool {
        match (self.punct(0), self.punct(1)) {
            (Some(left), Some(right)) => {
                left.as_char() == first
                    && right.as_char() == second
                    && left.spacing() == Spacing::Joint
            }
            _ => false,
        }
    }

    fn starts_with_range_op(&self) -> bool {
        self.starts_with_joint('.', '.')
    }

    fn starts_with_pathsep(&self) -> bool {
        self.starts_with_joint(':', ':')
    }

    fn bump(&mut self) -> Option<&'a TokenTree> {
        let token = self.tokens.get(self.index)?;
        self.index += 1;
        Some(token)
    }

    fn parse_range_op(&mut self) -> Result<bool> {
        let Some(first) = self.punct(0) else {
            return Err(Error::new(self.error_span(), "expected `..` or `..=`"));
        };
        if first.as_char() != '.' || first.spacing() != Spacing::Joint {
            return Err(Error::new(first.span(), "expected `..` or `..=`"));
        }
        let start_span = first.span();
        self.bump();
        let Some(second) = self.punct(0) else {
            return Err(Error::new(start_span, "expected `..` or `..=`"));
        };
        if second.as_char() != '.' {
            return Err(Error::new(second.span(), "expected `..` or `..=`"));
        }
        self.bump();
        // `..#` and `..::S` keep the second `.` joint because `#` and `:` are
        // punctuation. Only a following `=` makes the operator inclusive.
        if self.punct(0).is_some_and(|punct| punct.as_char() == '=') {
            self.bump();
            return Ok(true);
        }
        if self.punct(0).is_some_and(|punct| punct.as_char() == '.') {
            return Err(Error::new(start_span, "expected `..` or `..=`"));
        }
        Ok(false)
    }

    fn parse_pathsep(&mut self) -> Result<()> {
        if !self.starts_with_pathsep() {
            return Err(Error::new(self.error_span(), "expected `::`"));
        }
        self.bump();
        self.bump();
        Ok(())
    }

    fn try_bound(&mut self, style: BoundStyle) -> Result<Option<SpannedBound>> {
        if let Some(hash) = self.punct(0).filter(|punct| punct.as_char() == '#') {
            let span = hash.span();
            self.bump();
            let offset = self.parse_hash_offset()?;
            return Ok(Some(SpannedBound {
                bound: Bound::Here { offset },
                span,
            }));
        }
        if self.punct(0).is_some_and(|punct| punct.as_char() == '-') {
            let parsed = match (self.punct(0), self.tokens.get(self.index + 1)) {
                (Some(minus), Some(TokenTree::Literal(literal))) => {
                    Some((minus.span(), parse_unsuffixed_int(literal)?))
                }
                _ => None,
            };
            let Some((span, magnitude)) = parsed else {
                return Ok(None);
            };
            self.bump();
            self.bump();
            let bound = if style == BoundStyle::Pattern && magnitude == 1 {
                Bound::LastIndex
            } else {
                let signed = magnitude
                    .checked_neg()
                    .ok_or_else(|| Error::new(span, "integer literal overflows `i128`"))?;
                Bound::Int(signed)
            };
            return Ok(Some(SpannedBound { bound, span }));
        }
        let parsed = if let Some(TokenTree::Literal(literal)) = self.tokens.get(self.index) {
            Some((parse_unsuffixed_int(literal)?, literal.span()))
        } else {
            None
        };
        if let Some((value, span)) = parsed {
            self.bump();
            return Ok(Some(SpannedBound {
                bound: Bound::Int(value),
                span,
            }));
        }
        Ok(None)
    }

    fn parse_hash_offset(&mut self) -> Result<i128> {
        let sign = match self.punct(0).map(Punct::as_char) {
            Some('+') => 1,
            Some('-') => -1,
            _ => return Ok(0),
        };
        let Some(sign_punct) = self.punct(0) else {
            return Err(Error::new(
                Span::call_site(),
                "expected an integer literal after `#` `+` or `#` `-`",
            ));
        };
        let sign_span = sign_punct.span();
        self.bump();
        let magnitude = if let Some(TokenTree::Literal(literal)) = self.tokens.get(self.index) {
            parse_unsuffixed_int(literal)?
        } else {
            return Err(Error::new(
                sign_span,
                "expected an integer literal after `#` `+` or `#` `-`",
            ));
        };
        self.bump();
        let offset = match sign {
            1 => magnitude,
            _ => magnitude
                .checked_neg()
                .ok_or_else(|| Error::new(sign_span, "integer literal overflows `i128`"))?,
        };
        Ok(offset)
    }
}

fn strip_radix_prefix<'a>(text: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes.iter().find_map(|prefix| text.strip_prefix(prefix))
}

fn parse_unsuffixed_int(literal: &Literal) -> Result<i128> {
    let text = literal.to_string();
    let span = literal.span();
    if text.as_bytes().contains(&b'_') {
        let stripped: String = text.chars().filter(|character| *character != '_').collect();
        parse_int_digits(&stripped, span)
    } else {
        parse_int_digits(&text, span)
    }
}

fn parse_int_digits(text: &str, span: Span) -> Result<i128> {
    let (radix, digits) = if let Some(rest) = strip_radix_prefix(text, &["0x", "0X"]) {
        (16, rest)
    } else if let Some(rest) = strip_radix_prefix(text, &["0o", "0O"]) {
        (8, rest)
    } else if let Some(rest) = strip_radix_prefix(text, &["0b", "0B"]) {
        (2, rest)
    } else {
        (10, text)
    };
    if digits.is_empty() || !digits.chars().all(|character| character.is_digit(radix)) {
        return Err(Error::new(span, "expected an unsuffixed integer literal"));
    }
    i128::from_str_radix(digits, radix)
        .map_err(|_| Error::new(span, "integer literal overflows `i128`"))
}

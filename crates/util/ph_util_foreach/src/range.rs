//! Range syntax, domain checks, and index lists.
//!
//! Outer ranges and `Name[<pat>]` patterns share one parser. The outer range
//! must already be finite. A pattern may use open ends, `#`, and the literal `-1`.

use std::ops::{Range, RangeInclusive};

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

impl SpannedBound {
    fn int(value: i128, span: Span) -> Self {
        Self {
            bound: Bound::Int(value),
            span,
        }
    }

    fn here(offset: i128, span: Span) -> Self {
        Self {
            bound: Bound::Here { offset },
            span,
        }
    }

    fn last_index(span: Span) -> Self {
        Self {
            bound: Bound::LastIndex,
            span,
        }
    }
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

impl RangeExpr {
    fn builder(span: Span) -> RangeExprBuilder {
        RangeExprBuilder {
            start: None,
            end: None,
            end_inclusive: false,
            step: None,
            span,
        }
    }
}

#[derive(Clone, Copy)]
#[must_use]
struct RangeExprBuilder {
    start: Option<SpannedBound>,
    end: Option<SpannedBound>,
    end_inclusive: bool,
    step: Option<SpannedBound>,
    span: Span,
}

impl RangeExprBuilder {
    fn start(mut self, start: Option<SpannedBound>) -> Self {
        self.start = start;
        self
    }

    fn end(mut self, end: Option<SpannedBound>) -> Self {
        self.end = end;
        self
    }

    fn end_inclusive(mut self, end_inclusive: bool) -> Self {
        self.end_inclusive = end_inclusive;
        self
    }

    fn step(mut self, step: Option<SpannedBound>) -> Self {
        self.step = step;
        self
    }

    fn build(self) -> RangeExpr {
        RangeExpr {
            start: self.start,
            end: self.end,
            end_inclusive: self.end_inclusive,
            step: self.step,
            span: self.span,
        }
    }
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
    RangeExpr::builder(span)
        .start(Some(SpannedBound::int(0, span)))
        .end(Some(SpannedBound::here(0, span)))
        .end_inclusive(true)
        .build()
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
///
/// Every domain is an arithmetic progression, so the bounds are enough.
/// `lo + n * gap` is index `n`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Domain {
    lo: i128,
    hi: i128,
    /// Outer step. An exclusive end may land up to this far past `hi`.
    gap: i128,
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
        })
    }

    /// Domain of a flat iteration: the outer progression.
    pub(crate) fn flat(outer: &OuterRange) -> Self {
        Self {
            lo: outer.lo,
            hi: outer.hi,
            gap: outer.gap,
        }
    }

    fn contains(&self, value: i128) -> bool {
        if value < self.lo || value > self.hi || self.gap <= 0 {
            return false;
        }
        self.gap == 1 || (value - self.lo) % self.gap == 0
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

/// Values substituted while a pattern is resolved.
#[derive(Clone, Copy)]
pub(crate) struct EvalCtx<'a> {
    /// Current `#`.
    pub(crate) index: i128,
    pub(crate) domain: &'a Domain,
}

/// Finite outer range stored as an arithmetic progression.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OuterRange {
    lo: i128,
    hi: i128,
    gap: i128,
    count: usize,
}

impl OuterRange {
    /// `true` when the range selects no indices.
    pub(crate) fn is_empty(self) -> bool {
        self.count == 0
    }

    /// How many indices this range selects.
    pub(crate) fn count(self) -> usize {
        self.count
    }

    /// Index `position` of `lo + position * gap`.
    pub(crate) fn index_at(self, position: usize) -> i128 {
        let Ok(pos) = i128::try_from(position) else {
            return self.lo;
        };
        let Some(offset) = pos.checked_mul(self.gap) else {
            return self.lo;
        };
        self.lo.checked_add(offset).unwrap_or(self.lo)
    }
}

/// Parses `tokens` as one range. `style` decides what `-1` and `#` mean.
pub(crate) fn parse_range(tokens: &[TokenTree], style: BoundStyle) -> Result<RangeExpr> {
    if tokens.is_empty() {
        return Err(Error::new(Span::call_site(), "expected a range"));
    }
    RangeParse::open(tokens, style).parse_closed()
}

/// Parses a bracket pattern in one pass.
///
/// `None` means the brackets do not open with a range, so they stay ordinary
/// Rust. A broken bound such as `#+` is an error.
pub(crate) fn parse_pattern(tokens: &[TokenTree]) -> Result<Option<RangeExpr>> {
    if tokens.is_empty() {
        return Ok(None);
    }
    RangeParse::open(tokens, BoundStyle::Pattern).parse_optional()
}

struct RangeParse<'a> {
    cursor: Cursor<'a>,
    style: BoundStyle,
    span: Span,
    start: Option<SpannedBound>,
}

impl<'a> RangeParse<'a> {
    fn open(tokens: &'a [TokenTree], style: BoundStyle) -> Self {
        Self {
            span: tokens.first().map_or_else(Span::call_site, TokenTree::span),
            cursor: Cursor::new(tokens),
            style,
            start: None,
        }
    }

    fn with_start(mut self, start: Option<SpannedBound>) -> Self {
        self.start = start;
        self
    }

    fn parse_closed(mut self) -> Result<RangeExpr> {
        let start = self.cursor.maybe_bound(self.style)?;
        if start.is_some() && !self.cursor.starts_with_range_op() {
            return Err(Error::new(
                self.cursor.error_span(),
                "expected `..` or `..=`",
            ));
        }
        self.with_start(start).finish()
    }

    fn parse_optional(mut self) -> Result<Option<RangeExpr>> {
        let start = if self.cursor.starts_with_range_op() {
            None
        } else {
            match self.cursor.maybe_bound(self.style)? {
                Some(bound) if self.cursor.starts_with_range_op() => Some(bound),
                Some(_) | None => return Ok(None),
            }
        };
        self.with_start(start).finish().map(Some)
    }

    fn finish(mut self) -> Result<RangeExpr> {
        let end_inclusive = self.cursor.parse_range_op()?;
        let end = self.parse_end()?;
        let step = self.parse_step()?;
        self.reject_trailing()?;
        Ok(RangeExpr::builder(self.span)
            .start(self.start)
            .end(end)
            .end_inclusive(end_inclusive)
            .step(step)
            .build())
    }

    fn parse_end(&mut self) -> Result<Option<SpannedBound>> {
        if self.cursor.starts_with_pathsep() {
            Ok(None)
        } else {
            self.cursor.maybe_bound(self.style)
        }
    }

    fn parse_step(&mut self) -> Result<Option<SpannedBound>> {
        if !self.cursor.starts_with_pathsep() {
            return Ok(None);
        }
        self.cursor.parse_pathsep()?;
        self.cursor
            .require_bound(BoundStyle::Step, "expected a step after `::`")
            .map(Some)
    }

    fn reject_trailing(&self) -> Result<()> {
        if self.cursor.is_empty() {
            return Ok(());
        }
        let message = if self.style == BoundStyle::Pattern {
            "expected exactly one range pattern inside `[]`"
        } else {
            "unexpected token in range"
        };
        Err(Error::new(self.cursor.error_span(), message))
    }
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
    let window = if range.end_inclusive {
        IntRange::from(start..=end)
    } else {
        IntRange::from(start..end)
    }
    .with_step(gap);
    window.reject_inverted(range.span)?;
    let Some(count) = window.step_count() else {
        return Err(Error::new(range.span, "range is too large"));
    };
    if count == 0 {
        return Ok(OuterRange {
            lo: start,
            hi: start,
            gap,
            count,
        });
    }
    let Ok(steps) = i128::try_from(count - 1) else {
        return Err(Error::new(range.span, "range is too large"));
    };
    let Some(hi) = steps
        .checked_mul(gap)
        .and_then(|offset| start.checked_add(offset))
    else {
        return Err(Error::new(range.span, "range is too large"));
    };
    Ok(OuterRange {
        lo: start,
        hi,
        gap,
        count,
    })
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
    let window = if end_inclusive {
        IntRange::from(start..=end)
    } else {
        IntRange::from(start..end)
    }
    .with_step(step);
    // `T[#+1..=#]` when `#` is the last index: the window is empty and the
    // start sits one past the domain. That hole edge is allowed.
    if window.is_high_hole(ctx.domain) {
        return Ok(());
    }
    window.reject_inverted(range.span)?;
    if !ctx.domain.inclusive_ok(window.start) {
        return Err(outside(range.span, window.start, ctx.domain));
    }
    if window.end_inclusive {
        if !ctx.domain.inclusive_ok(window.end) {
            return Err(outside(range.span, window.end, ctx.domain));
        }
    } else if !ctx.domain.exclusive_ok(window.end) {
        return Err(outside(range.span, window.end, ctx.domain));
    }
    window.walk(Some(ctx.domain), &mut visit)
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

#[derive(Clone, Copy)]
struct IntRange {
    start: i128,
    end: i128,
    end_inclusive: bool,
    step: i128,
}

impl From<Range<i128>> for IntRange {
    fn from(range: Range<i128>) -> Self {
        Self {
            start: range.start,
            end: range.end,
            end_inclusive: false,
            step: 1,
        }
    }
}

impl From<RangeInclusive<i128>> for IntRange {
    fn from(range: RangeInclusive<i128>) -> Self {
        let (start, end) = range.into_inner();
        Self {
            start,
            end,
            end_inclusive: true,
            step: 1,
        }
    }
}

impl IntRange {
    fn with_step(mut self, step: i128) -> Self {
        self.step = step;
        self
    }

    fn is_high_hole(self, domain: &Domain) -> bool {
        let Some(just_past) = domain.hi.checked_add(1) else {
            return false;
        };
        self.end_inclusive && self.end == domain.hi && self.start == just_past
    }

    fn reject_inverted(self, span: Span) -> Result<()> {
        if self.start > self.end {
            let rendered = if self.end_inclusive {
                format!("{}..={}", self.start, self.end)
            } else {
                format!("{}..{}", self.start, self.end)
            };
            return Err(Error::new(span, format!("invalid range {rendered}")));
        }
        Ok(())
    }

    fn walk(
        self,
        domain: Option<&Domain>,
        visit: &mut impl FnMut(i128) -> Result<()>,
    ) -> Result<()> {
        let mut current = self.start;
        loop {
            if self.past_end(current) {
                break;
            }
            if domain.is_none_or(|domain| domain.contains(current)) {
                visit(current)?;
            }
            let Some(next) = self.advance(current) else {
                break;
            };
            current = next;
        }
        Ok(())
    }

    fn past_end(self, current: i128) -> bool {
        if self.end_inclusive {
            current > self.end
        } else {
            current >= self.end
        }
    }

    fn advance(self, current: i128) -> Option<i128> {
        match current.checked_add(self.step) {
            Some(next) if next > current => Some(next),
            _ => None,
        }
    }

    /// How many indices this window yields. `None` means the count does not fit in `usize`.
    fn step_count(self) -> Option<usize> {
        if self.step <= 0 {
            return Some(0);
        }
        let span = if self.end_inclusive {
            self.end.checked_sub(self.start)?
        } else {
            self.end.checked_sub(self.start)?.checked_sub(1)?
        };
        if span < 0 {
            return Some(0);
        }
        let steps = span / self.step;
        usize::try_from(steps.checked_add(1)?).ok()
    }
}

/// A bound that was missing or invalid.
enum BoundError {
    Absent { span: Span, message: &'static str },
    Invalid(Error),
}

impl BoundError {
    fn absent_message(self, message: &'static str) -> Self {
        match self {
            Self::Absent { span, .. } => Self::Absent { span, message },
            Self::Invalid(error) => Self::Invalid(error),
        }
    }
}

impl From<Error> for BoundError {
    fn from(error: Error) -> Self {
        Self::Invalid(error)
    }
}

impl From<BoundError> for Error {
    fn from(error: BoundError) -> Self {
        match error {
            BoundError::Absent { span, message } => Error::new(span, message),
            BoundError::Invalid(error) => error,
        }
    }
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

    fn maybe_bound(&mut self, style: BoundStyle) -> Result<Option<SpannedBound>> {
        match self.parse_bound(style) {
            Ok(bound) => Ok(Some(bound)),
            Err(BoundError::Absent { .. }) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn require_bound(&mut self, style: BoundStyle, message: &'static str) -> Result<SpannedBound> {
        self.parse_bound(style)
            .map_err(|error| error.absent_message(message).into())
    }

    fn parse_bound(&mut self, style: BoundStyle) -> std::result::Result<SpannedBound, BoundError> {
        if let Some(hash) = self.punct(0).filter(|punct| punct.as_char() == '#') {
            let span = hash.span();
            self.bump();
            let offset = self.parse_hash_offset()?;
            return Ok(SpannedBound::here(offset, span));
        }
        if self.punct(0).is_some_and(|punct| punct.as_char() == '-') {
            let parsed = match (self.punct(0), self.tokens.get(self.index + 1)) {
                (Some(minus), Some(TokenTree::Literal(literal))) => {
                    Some((minus.span(), parse_unsuffixed_int(literal)?))
                }
                _ => None,
            };
            let Some((span, magnitude)) = parsed else {
                return Err(self.absent());
            };
            self.bump();
            self.bump();
            if style == BoundStyle::Pattern && magnitude == 1 {
                return Ok(SpannedBound::last_index(span));
            }
            let signed = magnitude
                .checked_neg()
                .ok_or_else(|| Error::new(span, "integer literal overflows `i128`"))?;
            return Ok(SpannedBound::int(signed, span));
        }
        let parsed = if let Some(TokenTree::Literal(literal)) = self.tokens.get(self.index) {
            Some((parse_unsuffixed_int(literal)?, literal.span()))
        } else {
            None
        };
        if let Some((value, span)) = parsed {
            self.bump();
            return Ok(SpannedBound::int(value, span));
        }
        Err(self.absent())
    }

    fn absent(&self) -> BoundError {
        BoundError::Absent {
            span: self.error_span(),
            message: "expected a bound",
        }
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

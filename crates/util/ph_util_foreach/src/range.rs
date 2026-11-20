//! Range syntax, domain checks, and index lists.
//!
//! Outer ranges and `Name[<pat>]` patterns share one parser. The outer range
//! must already be finite. A pattern may use open ends, `#`, and the literal `-1`.

use std::ops::{Range, RangeInclusive};

use proc_macro::{Literal, Punct, Spacing, Span, TokenTree};

use crate::error::{Error, ErrorKind, Result};

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
        Self { bound: Bound::Int(value), span }
    }

    fn here(offset: i128, span: Span) -> Self {
        Self { bound: Bound::Here { offset }, span }
    }

    fn last_index(span: Span) -> Self {
        Self { bound: Bound::LastIndex, span }
    }

    fn mentions_index(self) -> bool {
        matches!(self.bound, Bound::Here { .. })
    }

    fn expect_outer_int(self) -> Result<i128> {
        match self.bound {
            Bound::Int(value) => Ok(value),
            Bound::Here { .. } | Bound::LastIndex => {
                Err(Error::new(self.span, ErrorKind::OuterHash))
            }
        }
    }

    fn expect_positive_step(self) -> Result<i128> {
        match self.bound {
            Bound::Int(value) if value > 0 => Ok(value),
            Bound::Int(_) => Err(Error::new(self.span, ErrorKind::StepNotPositive)),
            Bound::Here { .. } | Bound::LastIndex => {
                Err(Error::new(self.span, ErrorKind::OuterHash))
            }
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
    fn new(
        start: Option<SpannedBound>,
        end: Option<SpannedBound>,
        end_inclusive: bool,
        step: Option<SpannedBound>,
        span: Span,
    ) -> Self {
        Self { start, end, end_inclusive, step, span }
    }

    fn builder(span: Span) -> RangeExprBuilder {
        RangeExprBuilder::new(span)
    }

    fn shorthand(span: Span) -> Self {
        Self::builder(span)
            .start(Some(SpannedBound::int(0, span)))
            .end(Some(SpannedBound::here(0, span)))
            .end_inclusive(true)
            .build()
    }

    /// Parses `tokens` as one range. `style` decides what `-1` and `#` mean.
    pub(crate) fn parse(tokens: &[TokenTree], style: BoundStyle) -> Result<Self> {
        if tokens.is_empty() {
            return Err(Error::new(Span::call_site(), ErrorKind::ExpectedRange));
        }
        RangeParse::open(tokens, style).parse_closed()
    }

    /// Parses a bracket pattern in one pass.
    ///
    /// `None` means the brackets do not open with a range, so they stay ordinary
    /// Rust. A broken bound such as `#+` is an error.
    pub(crate) fn parse_pattern(tokens: &[TokenTree]) -> Result<Option<Self>> {
        if tokens.is_empty() {
            return Ok(None);
        }
        RangeParse::open(tokens, BoundStyle::Pattern).parse_optional()
    }

    /// Indices selected by a finite outer range.
    pub(crate) fn compile_outer(&self) -> Result<OuterRange> {
        let (Some(start_bound), Some(end_bound)) = (self.start, self.end) else {
            return Err(Error::new(self.span, ErrorKind::OpenOuter));
        };
        let start = start_bound.expect_outer_int()?;
        let end = end_bound.expect_outer_int()?;
        let gap = match self.step {
            None => 1,
            Some(step) => step.expect_positive_step()?,
        };
        let window = if self.end_inclusive {
            Window::Inclusive(start..=end)
        } else {
            Window::Exclusive(start..end)
        };
        window.reject_inverted(self.span)?;
        let Some(count) = window.step_count(gap) else {
            return Err(Error::new(self.span, ErrorKind::RangeTooLarge));
        };
        if count == 0 {
            return Ok(OuterRange::new(start, start, gap, count));
        }
        let Ok(steps) = i128::try_from(count - 1) else {
            return Err(Error::new(self.span, ErrorKind::RangeTooLarge));
        };
        let Some(hi) = steps
            .checked_mul(gap)
            .and_then(|offset| start.checked_add(offset))
        else {
            return Err(Error::new(self.span, ErrorKind::RangeTooLarge));
        };
        Ok(OuterRange::new(start, hi, gap, count))
    }

    /// Calls `visit` for each index this range selects inside `ctx.domain`.
    ///
    /// The walk does not allocate. An empty hole-edge window visits nothing.
    pub(crate) fn visit(
        &self,
        ctx: &EvalCtx<'_>,
        mut visit: impl FnMut(i128) -> Result<()>,
    ) -> Result<()> {
        let step = ctx.eval_step(self.step)?;
        let start = match self.start {
            Some(bound) => ctx.eval_bound(bound)?,
            None => ctx.domain.lo,
        };
        let (end, end_inclusive) = match self.end {
            Some(bound) => (ctx.eval_bound(bound)?, self.end_inclusive),
            None => (ctx.domain.hi, true),
        };
        let window = if end_inclusive {
            Window::Inclusive(start..=end)
        } else {
            Window::Exclusive(start..end)
        };
        // `T[#+1..=#]` when `#` is the last index: the window is empty and the
        // start sits one past the domain. That hole edge is allowed.
        if window.is_high_hole(ctx.domain) {
            return Ok(());
        }
        window.reject_inverted(self.span)?;
        if !ctx.domain.inclusive_ok(window.start()) {
            return Err(ctx.domain.outside(self.span, window.start()));
        }
        if window.is_inclusive() {
            if !ctx.domain.inclusive_ok(window.end()) {
                return Err(ctx.domain.outside(self.span, window.end()));
            }
        } else if !ctx.domain.exclusive_ok(window.end()) {
            return Err(ctx.domain.outside(self.span, window.end()));
        }
        window.walk(step, ctx.domain, &mut visit)
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
    fn new(span: Span) -> Self {
        Self { start: None, end: None, end_inclusive: false, step: None, span }
    }

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
        RangeExpr::new(
            self.start,
            self.end,
            self.end_inclusive,
            self.step,
            self.span,
        )
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
            Self::Range(range) => {
                range.start.is_some_and(SpannedBound::mentions_index)
                    || range.end.is_some_and(SpannedBound::mentions_index)
            }
        }
    }

    /// The range this pattern selects. `Name[#]` is `0..=#`.
    pub(crate) fn to_range(self) -> RangeExpr {
        match self {
            Self::Shorthand { span } => RangeExpr::shorthand(span),
            Self::Range(range) => range,
        }
    }
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
    fn new(lo: i128, hi: i128, gap: i128) -> Self {
        Self { lo, hi, gap }
    }

    /// Domain of a fan iteration: the prefix `0..=outer`.
    ///
    /// `outer` is already known to be non-negative. The caller checks that once.
    pub(crate) fn prefix(outer: i128) -> Self {
        Self::new(0, outer, 1)
    }

    /// Domain of a flat iteration: the outer progression.
    pub(crate) fn flat(outer: &OuterRange) -> Self {
        Self::new(outer.lo, outer.hi, outer.gap)
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

    fn outside(&self, span: Span, value: i128) -> Error {
        Error::new(
            span,
            ErrorKind::OutsideDomain { value, lo: self.lo, hi: self.hi },
        )
    }
}

/// Values substituted while a pattern is resolved.
#[derive(Clone, Copy)]
pub(crate) struct EvalCtx<'a> {
    /// Current `#`.
    pub(crate) index: i128,
    pub(crate) domain: &'a Domain,
}

impl<'a> EvalCtx<'a> {
    /// The index and domain for one substitution.
    pub(crate) fn new(index: i128, domain: &'a Domain) -> Self {
        Self { index, domain }
    }

    fn eval_step(self, step: Option<SpannedBound>) -> Result<i128> {
        let Some(step) = step else {
            return Ok(1);
        };
        let value = match step.bound {
            Bound::Int(value) => value,
            Bound::Here { offset } => add_offset(self.index, offset, step.span)?,
            Bound::LastIndex => return Err(Error::new(step.span, ErrorKind::StepLastIndex)),
        };
        if value <= 0 {
            return Err(Error::new(step.span, ErrorKind::StepNotPositive));
        }
        Ok(value)
    }

    fn eval_bound(self, bound: SpannedBound) -> Result<i128> {
        match bound.bound {
            Bound::Int(value) => Ok(value),
            Bound::LastIndex => Ok(self.domain.hi),
            Bound::Here { offset } => add_offset(self.index, offset, bound.span),
        }
    }

    /// The unsuffixed integer literal for the current index.
    pub(crate) fn index_literal(self, span: Span) -> Literal {
        let mut literal = Literal::i128_unsuffixed(self.index);
        literal.set_span(span);
        literal
    }
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
    fn new(lo: i128, hi: i128, gap: i128, count: usize) -> Self {
        Self { lo, hi, gap, count }
    }

    /// `true` when the range selects no indices.
    pub(crate) fn is_empty(self) -> bool {
        self.count == 0
    }

    /// How many indices this range selects.
    pub(crate) fn count(self) -> usize {
        self.count
    }

    /// The first selected index.
    pub(crate) fn first(self) -> i128 {
        self.lo
    }

    /// The index one step after `index`. The progression already fits, so this stays on it.
    pub(crate) fn next_after(self, index: i128) -> i128 {
        index.checked_add(self.gap).unwrap_or(index)
    }

    /// Calls `visit` once for each selected index, in order.
    pub(crate) fn each(self, mut visit: impl FnMut(i128) -> Result<()>) -> Result<()> {
        let mut index = self.first();
        for position in 0..self.count() {
            if position != 0 {
                index = self.next_after(index);
            }
            visit(index)?;
        }
        Ok(())
    }
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
                ErrorKind::ExpectedDots,
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
        self.cursor.require_step().map(Some)
    }

    fn reject_trailing(&self) -> Result<()> {
        if self.cursor.is_empty() {
            return Ok(());
        }
        let span = self.cursor.error_span();
        Err(if self.style == BoundStyle::Pattern {
            Error::new(span, ErrorKind::OnePattern)
        } else {
            Error::new(span, ErrorKind::UnexpectedInRange)
        })
    }
}

fn add_offset(index: i128, offset: i128, span: Span) -> Result<i128> {
    index
        .checked_add(offset)
        .ok_or(Error::new(span, ErrorKind::IndexOverflow))
}

/// `start..end` or `start..=end`. The step stays beside the range.
enum Window {
    Exclusive(Range<i128>),
    Inclusive(RangeInclusive<i128>),
}

impl Window {
    fn start(&self) -> i128 {
        match self {
            Self::Exclusive(range) => range.start,
            Self::Inclusive(range) => *range.start(),
        }
    }

    fn end(&self) -> i128 {
        match self {
            Self::Exclusive(range) => range.end,
            Self::Inclusive(range) => *range.end(),
        }
    }

    fn is_inclusive(&self) -> bool {
        matches!(self, Self::Inclusive(_))
    }

    fn is_high_hole(&self, domain: &Domain) -> bool {
        let Self::Inclusive(range) = self else {
            return false;
        };
        let Some(just_past) = domain.hi.checked_add(1) else {
            return false;
        };
        *range.end() == domain.hi && *range.start() == just_past
    }

    fn reject_inverted(&self, span: Span) -> Result<()> {
        if self.start() > self.end() {
            return Err(Error::new(
                span,
                ErrorKind::InvalidRange {
                    start: self.start(),
                    end: self.end(),
                    inclusive: self.is_inclusive(),
                },
            ));
        }
        Ok(())
    }

    fn walk(
        &self,
        step: i128,
        domain: &Domain,
        visit: &mut impl FnMut(i128) -> Result<()>,
    ) -> Result<()> {
        match self.clip(step, domain) {
            Clip::Empty => Ok(()),
            Clip::Ready { mut current, stride, last } => {
                loop {
                    if current > last {
                        break;
                    }
                    visit(current)?;
                    let Some(next) = current.checked_add(stride) else {
                        break;
                    };
                    current = next;
                }
                Ok(())
            }
            Clip::Filtered => self.walk_filtered(step, domain, visit),
        }
    }

    /// The domain filter, used only when the clipped progression does not fit in `i128`.
    fn walk_filtered(
        &self,
        step: i128,
        domain: &Domain,
        visit: &mut impl FnMut(i128) -> Result<()>,
    ) -> Result<()> {
        let mut current = self.start();
        loop {
            if self.past_end(current) {
                break;
            }
            if domain.contains(current) {
                visit(current)?;
            }
            let Some(next) = current.checked_add(step) else {
                break;
            };
            if next <= current {
                break;
            }
            current = next;
        }
        Ok(())
    }

    /// Where this window meets `domain`. The ready plan needs no per-index test.
    fn clip(&self, step: i128, domain: &Domain) -> Clip {
        if step <= 0 || domain.gap <= 0 {
            return Clip::Empty;
        }
        let Some(end) = self.last_index() else {
            return Clip::Empty;
        };
        let start = self.start();
        let last = end.min(domain.hi);
        if start > last {
            return Clip::Empty;
        }
        let gap = domain.gap;
        let g = gcd(step, gap);
        let Some(offset) = start.checked_sub(domain.lo) else {
            return Clip::Filtered;
        };
        if offset.rem_euclid(g) != 0 {
            return Clip::Empty;
        }
        let Some(k) = steps_until_aligned(step, gap, offset) else {
            return Clip::Filtered;
        };
        let Some(stride) = (step / g).checked_mul(gap) else {
            return Clip::Filtered;
        };
        let Some(first) = k
            .checked_mul(step)
            .and_then(|shift| start.checked_add(shift))
        else {
            return Clip::Filtered;
        };
        if first > last {
            Clip::Empty
        } else {
            Clip::Ready { current: first, stride, last }
        }
    }

    fn last_index(&self) -> Option<i128> {
        let end = self.end();
        if self.is_inclusive() {
            Some(end)
        } else {
            end.checked_sub(1)
        }
    }

    fn past_end(&self, current: i128) -> bool {
        if self.is_inclusive() {
            current > self.end()
        } else {
            current >= self.end()
        }
    }

    /// How many indices this window yields at `step`. `None` means the count does not fit in `usize`.
    fn step_count(&self, step: i128) -> Option<usize> {
        if step <= 0 {
            return Some(0);
        }
        let span = if self.is_inclusive() {
            self.end().checked_sub(self.start())?
        } else {
            self.end().checked_sub(self.start())?.checked_sub(1)?
        };
        if span < 0 {
            return Some(0);
        }
        let steps = span / step;
        usize::try_from(steps.checked_add(1)?).ok()
    }
}

enum Clip {
    Empty,
    Ready {
        current: i128,
        stride: i128,
        last: i128,
    },
    Filtered,
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a.abs()
}

/// Smallest `k >= 0` such that `offset + k * step` is divisible by `gap`.
fn steps_until_aligned(step: i128, gap: i128, offset: i128) -> Option<i128> {
    let need = (-offset).rem_euclid(gap);
    if need == 0 {
        return Some(0);
    }
    let (g, coefficient) = egcd(step % gap, gap)?;
    if need % g != 0 {
        return None;
    }
    let period = gap / g;
    let factor = (need / g).rem_euclid(period);
    let k = coefficient.rem_euclid(period).checked_mul(factor)?;
    Some(k.rem_euclid(period))
}

/// `(g, x)` with `a * x + b * y = g` and `g > 0`.
fn egcd(mut a: i128, mut b: i128) -> Option<(i128, i128)> {
    let (mut x0, mut x1) = (1i128, 0i128);
    while b != 0 {
        let q = a.checked_div(b)?;
        let r = a.checked_rem(b)?;
        a = b;
        b = r;
        let next_x = x0.checked_sub(q.checked_mul(x1)?)?;
        x0 = x1;
        x1 = next_x;
    }
    if a < 0 {
        Some((a.checked_neg()?, x0.checked_neg()?))
    } else {
        Some((a, x0))
    }
}

/// A bound that was missing or invalid.
enum BoundError {
    Absent(Span),
    Invalid(Error),
}

impl From<Error> for BoundError {
    fn from(error: Error) -> Self {
        Self::Invalid(error)
    }
}

impl From<BoundError> for Error {
    fn from(error: BoundError) -> Self {
        match error {
            BoundError::Absent(span) => Self::new(span, ErrorKind::ExpectedBound),
            BoundError::Invalid(error) => error,
        }
    }
}

type BoundResult = std::result::Result<SpannedBound, BoundError>;

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
            return Err(Error::new(self.error_span(), ErrorKind::ExpectedDots));
        };
        if first.as_char() != '.' || first.spacing() != Spacing::Joint {
            return Err(Error::new(first.span(), ErrorKind::ExpectedDots));
        }
        let start_span = first.span();
        self.bump();
        let Some(second) = self.punct(0) else {
            return Err(Error::new(start_span, ErrorKind::ExpectedDots));
        };
        if second.as_char() != '.' {
            return Err(Error::new(second.span(), ErrorKind::ExpectedDots));
        }
        self.bump();
        // `..#` and `..::S` keep the second `.` joint because `#` and `:` are
        // punctuation. Only a following `=` makes the operator inclusive.
        if self.punct(0).is_some_and(|punct| punct.as_char() == '=') {
            self.bump();
            return Ok(true);
        }
        if self.punct(0).is_some_and(|punct| punct.as_char() == '.') {
            return Err(Error::new(start_span, ErrorKind::ExpectedDots));
        }
        Ok(false)
    }

    fn parse_pathsep(&mut self) -> Result<()> {
        if !self.starts_with_pathsep() {
            return Err(Error::new(self.error_span(), ErrorKind::ExpectedPathSep));
        }
        self.bump();
        self.bump();
        Ok(())
    }

    fn maybe_bound(&mut self, style: BoundStyle) -> Result<Option<SpannedBound>> {
        match self.parse_bound(style) {
            Ok(bound) => Ok(Some(bound)),
            Err(BoundError::Absent(_)) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn require_step(&mut self) -> Result<SpannedBound> {
        self.parse_bound(BoundStyle::Step)
            .map_err(|error| match error {
                BoundError::Absent(span) => Error::new(span, ErrorKind::ExpectedStep),
                BoundError::Invalid(error) => error,
            })
    }

    fn parse_bound(&mut self, style: BoundStyle) -> BoundResult {
        if let Some(hash) = self.punct(0).filter(|punct| punct.as_char() == '#') {
            let span = hash.span();
            self.bump();
            let offset = self.parse_hash_offset()?;
            return Ok(SpannedBound::here(offset, span));
        }
        if self.punct(0).is_some_and(|punct| punct.as_char() == '-') {
            let parsed = match (self.punct(0), self.tokens.get(self.index + 1)) {
                (Some(minus), Some(TokenTree::Literal(literal))) => {
                    Some((minus.span(), Radix::parse(literal)?))
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
                .ok_or(Error::new(span, ErrorKind::LiteralOverflow))?;
            return Ok(SpannedBound::int(signed, span));
        }
        let parsed = if let Some(TokenTree::Literal(literal)) = self.tokens.get(self.index) {
            Some((Radix::parse(literal)?, literal.span()))
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
        BoundError::Absent(self.error_span())
    }

    fn parse_hash_offset(&mut self) -> Result<i128> {
        let sign = match self.punct(0).map(Punct::as_char) {
            Some('+') => 1,
            Some('-') => -1,
            _ => return Ok(0),
        };
        let Some(sign_punct) = self.punct(0) else {
            return Err(Error::new(Span::call_site(), ErrorKind::MissingAfterSign));
        };
        let sign_span = sign_punct.span();
        self.bump();
        let magnitude = if let Some(TokenTree::Literal(literal)) = self.tokens.get(self.index) {
            Radix::parse(literal)?
        } else {
            return Err(Error::new(sign_span, ErrorKind::MissingAfterSign));
        };
        self.bump();
        let offset = match sign {
            1 => magnitude,
            _ => magnitude
                .checked_neg()
                .ok_or(Error::new(sign_span, ErrorKind::LiteralOverflow))?,
        };
        Ok(offset)
    }
}

/// Bases of an unsuffixed integer literal, and the prefix bytes that select them.
struct Radix;

impl Radix {
    const DECIMAL: u32 = 10;
    const HEX: u32 = 16;
    const OCTAL: u32 = 8;
    const BINARY: u32 = 2;

    const PREFIX: u8 = b'0';
    const HEX_MARK: u8 = b'x';
    const OCTAL_MARK: u8 = b'o';
    const BINARY_MARK: u8 = b'b';

    fn parse(literal: &Literal) -> Result<i128> {
        let text = literal.to_string();
        let span = literal.span();
        if text.as_bytes().contains(&b'_') {
            let stripped: String = text.chars().filter(|character| *character != '_').collect();
            Self::parse_digits(&stripped, span)
        } else {
            Self::parse_digits(&text, span)
        }
    }

    fn parse_digits(text: &str, span: Span) -> Result<i128> {
        let (radix, digits) = Self::split(text);
        if digits.is_empty() || !digits.bytes().all(|byte| RadixDigit::is(byte, radix)) {
            return Err(Error::new(span, ErrorKind::UnsuffixedLiteral));
        }
        i128::from_str_radix(digits, radix)
            .map_err(|_| Error::new(span, ErrorKind::LiteralOverflow))
    }

    fn split(text: &str) -> (u32, &str) {
        let bytes = text.as_bytes();
        let Some((&marker, rest)) = bytes.split_first() else {
            return (Self::DECIMAL, text);
        };
        if marker != Self::PREFIX {
            return (Self::DECIMAL, text);
        }
        let Some((&kind, digits)) = rest.split_first() else {
            return (Self::DECIMAL, text);
        };
        let radix = match kind.to_ascii_lowercase() {
            Self::HEX_MARK => Self::HEX,
            Self::OCTAL_MARK => Self::OCTAL,
            Self::BINARY_MARK => Self::BINARY,
            _ => return (Self::DECIMAL, text),
        };
        let prefix_len = bytes.len() - digits.len();
        (radix, &text[prefix_len..])
    }
}

/// Bytes that count as a digit of a `Radix` base.
struct RadixDigit;

impl RadixDigit {
    const ZERO: u8 = b'0';
    const NINE: u8 = b'9';
    const LOWER_A: u8 = b'a';
    const LOWER_Z: u8 = b'z';
    const UPPER_A: u8 = b'A';
    const UPPER_Z: u8 = b'Z';

    fn is(byte: u8, radix: u32) -> bool {
        let value = match byte {
            Self::ZERO..=Self::NINE => u32::from(byte - Self::ZERO),
            Self::LOWER_A..=Self::LOWER_Z | Self::UPPER_A..=Self::UPPER_Z => {
                u32::from(byte.to_ascii_lowercase() - Self::LOWER_A) + Radix::DECIMAL
            }
            _ => return false,
        };
        value < radix
    }
}

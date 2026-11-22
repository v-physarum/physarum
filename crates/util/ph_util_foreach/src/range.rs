//! Range syntax, domain checks, and index lists.
//!
//! Outer ranges and `Name[<pat>]` patterns share one parser. `syn` reads a range
//! literal as [`ExprRange`] and each integer as [`LitInt`]. The outer range must
//! already be finite. A pattern may use open ends, `#`, and the literal `-1`.
//!
//! Evaluation walks [`Range`] and [`RangeInclusive`] with [`Iterator::step_by`].
//! The domain is a second progression, so the stride passed to `step_by` is the
//! alignment of the pattern step with that domain. A step wider than [`usize`]
//! cannot be passed to `step_by`.
//!
//! [`ExprRange`]: syn::ExprRange
//! [`LitInt`]: syn::LitInt
//! [`Range`]: std::ops::Range
//! [`RangeInclusive`]: std::ops::RangeInclusive
//! [`Iterator::step_by`]: std::iter::Iterator::step_by
//! [`usize`]: usize

use std::ops::{Range, RangeInclusive};

use proc_macro2::{Ident, Literal, Spacing, Span, TokenStream, TokenTree};
use syn::{
    BinOp, Expr, ExprBinary, ExprPath, ExprUnary, Lit, LitInt, PathArguments, RangeLimits, UnOp,
};

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

impl BoundStyle {
    fn is_last_index(self, magnitude: i128) -> bool {
        self == Self::Pattern && magnitude == 1
    }
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
    fn new(built: RangeExprBuilder) -> Self {
        Self {
            start: built.start,
            end: built.end,
            end_inclusive: built.end_inclusive,
            step: built.step,
            span: built.span,
        }
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

    fn mentions_index(self) -> bool {
        let start = self.start.is_some_and(SpannedBound::mentions_index);
        let end = self.end.is_some_and(SpannedBound::mentions_index);
        start || end
    }

    /// Parses `tokens` as one range. `style` decides what `-1` and `#` mean.
    pub(crate) fn parse(tokens: &[TokenTree], style: BoundStyle) -> Result<Self> {
        if tokens.is_empty() {
            return Err(Error::new(Span::call_site(), ErrorKind::ExpectedRange));
        }
        match open_kind(tokens)? {
            Open::Range => parse_range_literal(tokens, style),
            Open::NotRange(span) => Err(Error::new(span, ErrorKind::ExpectedDots)),
        }
    }

    /// Parses a bracket pattern in one pass.
    ///
    /// `None` means the brackets do not open with a range, so they stay ordinary
    /// Rust. A broken bound such as `#+` is an error.
    pub(crate) fn parse_pattern(tokens: &[TokenTree]) -> Result<Option<Self>> {
        if tokens.is_empty() {
            return Ok(None);
        }
        match open_kind(tokens)? {
            Open::Range => parse_range_literal(tokens, BoundStyle::Pattern).map(Some),
            Open::NotRange(_) => Ok(None),
        }
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
        RangeExpr::new(self)
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
            Self::Range(range) => range.mentions_index(),
        }
    }

    /// The range this pattern selects. `Name[#]` is `0..=#`.
    pub(crate) fn to_range(self) -> RangeExpr {
        match self {
            Self::Shorthand { span } => RangeExpr::shorthand(span),
            Self::Range(range) => range,
        }
    }

    /// `Off[#]`: the shorthand pattern on the name `Off`.
    pub(crate) fn is_off(self, raw: &str) -> bool {
        raw == "Off" && matches!(self, Self::Shorthand { .. })
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
            Bound::Here { offset } => self.add_offset(offset, step.span)?,
            Bound::LastIndex => return Err(Error::new(step.span, ErrorKind::StepLastIndex)),
        };
        if value <= 0 {
            return Err(Error::new(step.span, ErrorKind::StepNotPositive));
        }
        Ok(value)
    }

    fn add_offset(self, offset: i128, span: Span) -> Result<i128> {
        self.index
            .checked_add(offset)
            .ok_or(Error::new(span, ErrorKind::IndexOverflow))
    }

    fn eval_bound(self, bound: SpannedBound) -> Result<i128> {
        match bound.bound {
            Bound::Int(value) => Ok(value),
            Bound::LastIndex => Ok(self.domain.hi),
            Bound::Here { offset } => self.add_offset(offset, bound.span),
        }
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

    /// The first selected index.
    pub(crate) fn first(self) -> i128 {
        self.lo
    }

    /// Calls `visit` once for each selected index, in order.
    ///
    /// `hi` is the last index of this progression, so the walk is
    /// `(lo..=hi).step_by(gap)`. An empty range is not that iterator: `lo..=lo`
    /// would yield one index.
    pub(crate) fn each(self, mut visit: impl FnMut(i128) -> Result<()>) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        let Ok(step) = usize::try_from(self.gap) else {
            return self.each_wide(visit);
        };
        for index in (self.lo..=self.hi).step_by(step) {
            visit(index)?;
        }
        Ok(())
    }

    /// `gap` does not fit in [`usize`], so [`Iterator::step_by`] cannot take it.
    fn each_wide(self, mut visit: impl FnMut(i128) -> Result<()>) -> Result<()> {
        let mut index = self.lo;
        for position in 0..self.count {
            if position != 0 {
                index = index.checked_add(self.gap).ok_or_else(Error::too_large)?;
            }
            visit(index)?;
        }
        Ok(())
    }
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
            Clip::Ready { current, stride, last } => {
                step_through(current, last, true, stride, |_| true, visit)
            }
            Clip::Filtered => {
                let start = self.start();
                let end = self.end();
                let inclusive = self.is_inclusive();
                step_through(
                    start,
                    end,
                    inclusive,
                    step,
                    |index| domain.contains(index),
                    visit,
                )
            }
        }
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
        let g = Self::gcd(step, gap);
        let Some(offset) = start.checked_sub(domain.lo) else {
            return Clip::Filtered;
        };
        if offset.rem_euclid(g) != 0 {
            return Clip::Empty;
        }
        let Some(k) = Self::steps_until_aligned(step, gap, offset) else {
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
        let (g, coefficient) = Self::egcd(step % gap, gap)?;
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

/// Whether the tokens open with a range literal.
enum Open {
    /// `..`, or a bound followed by `..` / `..=`.
    Range,
    /// Not a range. The span is where `..` was missing.
    NotRange(Span),
}

fn open_kind(tokens: &[TokenTree]) -> Result<Open> {
    if starts_with_range_op(tokens) {
        return Ok(Open::Range);
    }
    match bound_prefix(tokens)? {
        Some(len) if starts_with_range_op(&tokens[len..]) => Ok(Open::Range),
        Some(len) => Ok(Open::NotRange(token_span(&tokens[len..]))),
        None => Ok(Open::NotRange(token_span(tokens))),
    }
}

/// Placeholder written where the source has `#`, so `syn` can parse the range.
const HASH_IDENT: &str = "ph_foreach_index";
/// Stand-in for a user identifier that would otherwise look like [`HASH_IDENT`].
const USER_IDENT: &str = "ph_foreach_user";
/// Tokens in a one-token bound: `#` or a single integer literal.
const ONE_TOKEN: usize = 1;
/// Tokens in a `-N` bound.
const SIGNED_LITERAL: usize = 2;
/// Tokens in a `#±N` bound.
const HASH_OFFSET: usize = 3;

fn parse_range_literal(tokens: &[TokenTree], style: BoundStyle) -> Result<RangeExpr> {
    if let Some(span) = ellipsis_span(tokens)? {
        return Err(Error::new(span, ErrorKind::ExpectedDots));
    }
    let split = split_step(tokens);
    reject_hash_tails(split.range)?;
    if let Some(step) = split.step {
        reject_hash_tails(step)?;
    }
    let span = token_span(tokens);
    let expr = match syn_expr(split.range) {
        Ok(expr) => expr,
        Err(syn_span) => return Err(syn_failure(split.range, style, syn_span)),
    };
    let Expr::Range(range) = expr else {
        return Err(Error::new(span, ErrorKind::ExpectedDots));
    };
    if side_has_range(range.start.as_deref()) || side_has_range(range.end.as_deref()) {
        return Err(trailing(style, span));
    }
    let start = bound_from(range.start.as_deref(), style, trailing_kind(style))?;
    let end = bound_from(range.end.as_deref(), style, trailing_kind(style))?;
    let end_inclusive = match range.limits {
        RangeLimits::Closed(_) => true,
        RangeLimits::HalfOpen(_) => false,
    };
    let step = match split.step {
        None => None,
        Some(step_tokens) => Some(parse_step(step_tokens, style, split.missing)?),
    };
    Ok(RangeExpr::builder(span)
        .start(start)
        .end(end)
        .end_inclusive(end_inclusive)
        .step(step)
        .build())
}

fn ellipsis_span(tokens: &[TokenTree]) -> Result<Option<Span>> {
    let index = if starts_with_range_op(tokens) {
        0
    } else {
        match bound_prefix(tokens)? {
            Some(len) => len,
            None => return Ok(None),
        }
    };
    if is_joint_pair(tokens, index, '.', '.') && is_punct(tokens, index + 2, '.') {
        return Ok(Some(token_span(&tokens[index..])));
    }
    Ok(None)
}

fn parse_step(tokens: &[TokenTree], parent: BoundStyle, missing: Span) -> Result<SpannedBound> {
    if tokens.is_empty() {
        return Err(Error::new(missing, ErrorKind::ExpectedStep));
    }
    let expr = match syn_expr(tokens) {
        Ok(expr) => expr,
        Err(span) => {
            let kind = if opens_as_bound(tokens)? {
                trailing_kind(parent)
            } else {
                ErrorKind::ExpectedStep
            };
            return Err(Error::new(span, kind));
        }
    };
    if expr_has_range(&expr) {
        return Err(trailing(parent, expr_span(&expr)));
    }
    interpret(&expr, BoundStyle::Step, ErrorKind::ExpectedStep)
}

fn bound_from(
    expr: Option<&Expr>,
    style: BoundStyle,
    shape: ErrorKind,
) -> Result<Option<SpannedBound>> {
    match expr {
        None => Ok(None),
        Some(expr) => interpret(expr, style, shape).map(Some),
    }
}

fn interpret(expr: &Expr, style: BoundStyle, shape: ErrorKind) -> Result<SpannedBound> {
    match expr {
        Expr::Lit(literal) => lit_bound(&literal.lit, style, false, literal.lit.span()),
        Expr::Unary(unary) => unary_bound(unary, style, shape),
        Expr::Path(path) if is_hash_path(path) => Ok(SpannedBound::here(0, hash_span(path))),
        Expr::Binary(binary) => hash_offset(binary, shape),
        Expr::Group(group) => interpret(&group.expr, style, shape),
        other => Err(Error::new(expr_span(other), shape)),
    }
}

fn unary_bound(unary: &ExprUnary, style: BoundStyle, shape: ErrorKind) -> Result<SpannedBound> {
    let UnOp::Neg(minus) = &unary.op else {
        return Err(Error::new(expr_span(&unary.expr), shape));
    };
    match unary.expr.as_ref() {
        Expr::Lit(literal) => lit_bound(&literal.lit, style, true, minus.span),
        _ => Err(Error::new(minus.span, shape)),
    }
}

fn hash_offset(binary: &ExprBinary, shape: ErrorKind) -> Result<SpannedBound> {
    if !is_hash_expr(&binary.left) {
        return Err(Error::new(expr_span(&binary.left), shape));
    }
    let span = hash_expr_span(&binary.left);
    let Expr::Lit(literal) = binary.right.as_ref() else {
        return Err(Error::new(expr_span(&binary.right), shape));
    };
    let Lit::Int(int) = &literal.lit else {
        return Err(Error::new(literal.lit.span(), ErrorKind::UnsuffixedLiteral));
    };
    let magnitude = plain_int(int)?;
    let offset = match &binary.op {
        BinOp::Add(_) => magnitude,
        BinOp::Sub(_) => magnitude
            .checked_neg()
            .ok_or_else(|| Error::new(span, ErrorKind::LiteralOverflow))?,
        _ => return Err(Error::new(span, shape)),
    };
    Ok(SpannedBound::here(offset, span))
}

fn lit_bound(lit: &Lit, style: BoundStyle, negative: bool, span: Span) -> Result<SpannedBound> {
    let Lit::Int(int) = lit else {
        return Err(Error::new(span, ErrorKind::UnsuffixedLiteral));
    };
    let magnitude = plain_int(int)?;
    if !negative {
        return Ok(SpannedBound::int(magnitude, int.span()));
    }
    if style.is_last_index(magnitude) {
        return Ok(SpannedBound::last_index(span));
    }
    let signed = magnitude
        .checked_neg()
        .ok_or_else(|| Error::new(span, ErrorKind::LiteralOverflow))?;
    Ok(SpannedBound::int(signed, span))
}

fn plain_int(int: &LitInt) -> Result<i128> {
    if !int.suffix().is_empty() {
        return Err(Error::new(int.span(), ErrorKind::UnsuffixedLiteral));
    }
    int.base10_parse::<i128>()
        .map_err(|_| Error::new(int.span(), ErrorKind::LiteralOverflow))
}

fn is_hash_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Path(path) => is_hash_path(path),
        Expr::Group(group) => is_hash_expr(&group.expr),
        _ => false,
    }
}

fn is_hash_path(path: &ExprPath) -> bool {
    if !path.attrs.is_empty() || path.qself.is_some() || path.path.leading_colon.is_some() {
        return false;
    }
    let mut segments = path.path.segments.iter();
    let Some(segment) = segments.next() else {
        return false;
    };
    if segments.next().is_some() {
        return false;
    }
    segment.ident == HASH_IDENT && matches!(segment.arguments, PathArguments::None)
}

fn hash_span(path: &ExprPath) -> Span {
    path.path
        .segments
        .first()
        .map_or_else(Span::call_site, |segment| segment.ident.span())
}

fn hash_expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Path(path) => hash_span(path),
        Expr::Group(group) => hash_expr_span(&group.expr),
        other => expr_span(other),
    }
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Lit(literal) => literal.lit.span(),
        Expr::Path(path) => hash_span(path),
        Expr::Unary(unary) => match &unary.op {
            UnOp::Neg(token) => token.span,
            UnOp::Not(token) => token.span,
            UnOp::Deref(token) => token.span,
            _ => Span::call_site(),
        },
        Expr::Group(group) => expr_span(&group.expr),
        _ => Span::call_site(),
    }
}

fn side_has_range(expr: Option<&Expr>) -> bool {
    expr.is_some_and(expr_has_range)
}

fn expr_has_range(expr: &Expr) -> bool {
    match expr {
        Expr::Range(_) => true,
        Expr::Binary(binary) => expr_has_range(&binary.left) || expr_has_range(&binary.right),
        Expr::Unary(unary) => expr_has_range(&unary.expr),
        Expr::Paren(paren) => expr_has_range(&paren.expr),
        Expr::Group(group) => expr_has_range(&group.expr),
        _ => false,
    }
}

fn trailing(style: BoundStyle, span: Span) -> Error {
    Error::new(span, trailing_kind(style))
}

fn trailing_kind(style: BoundStyle) -> ErrorKind {
    if style == BoundStyle::Pattern {
        ErrorKind::OnePattern
    } else {
        ErrorKind::UnexpectedInRange
    }
}

fn syn_failure(tokens: &[TokenTree], style: BoundStyle, span: Span) -> Error {
    let kind = if contains_range_op(tokens) {
        trailing_kind(style)
    } else {
        ErrorKind::ExpectedDots
    };
    Error::new(span, kind)
}

struct StepSplit<'a> {
    range: &'a [TokenTree],
    step: Option<&'a [TokenTree]>,
    missing: Span,
}

fn split_step(tokens: &[TokenTree]) -> StepSplit<'_> {
    let mut index = 0;
    while index + 1 < tokens.len() {
        if is_joint_pair(tokens, index, ':', ':') {
            return StepSplit {
                range: &tokens[..index],
                step: Some(&tokens[index + 2..]),
                missing: token_span(&tokens[index..]),
            };
        }
        index += 1;
    }
    StepSplit { range: tokens, step: None, missing: Span::call_site() }
}

fn syn_expr(tokens: &[TokenTree]) -> std::result::Result<Expr, Span> {
    syn::parse2(tokens_for_syn(tokens)).map_err(|error| error.span())
}

fn tokens_for_syn(tokens: &[TokenTree]) -> TokenStream {
    let mut out = TokenStream::new();
    for token in tokens {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                let ident = Ident::new(HASH_IDENT, punct.span());
                out.extend([TokenTree::Ident(ident)]);
            }
            TokenTree::Ident(ident) if ident == HASH_IDENT || ident == USER_IDENT => {
                let renamed = Ident::new(USER_IDENT, ident.span());
                out.extend([TokenTree::Ident(renamed)]);
            }
            TokenTree::Literal(literal) => {
                out.extend([TokenTree::Literal(literal_for_syn(literal))]);
            }
            other => out.extend([other.clone()]),
        }
    }
    out
}

fn literal_for_syn(literal: &Literal) -> Literal {
    let text = literal.to_string();
    let mut chars = text.chars();
    if chars.next() != Some('0') {
        return literal.clone();
    }
    let Some(mark) = chars.next() else {
        return literal.clone();
    };
    if !matches!(mark, 'X' | 'O' | 'B') {
        return literal.clone();
    }
    let mut normalized = String::with_capacity(text.len());
    normalized.push('0');
    normalized.push(mark.to_ascii_lowercase());
    normalized.extend(chars);
    match normalized.parse::<Literal>() {
        Ok(mut parsed) => {
            parsed.set_span(literal.span());
            parsed
        }
        Err(_) => literal.clone(),
    }
}

fn bound_prefix(tokens: &[TokenTree]) -> Result<Option<usize>> {
    if is_punct(tokens, 0, '#') {
        if is_sign(tokens, 1) {
            let Some(TokenTree::Literal(literal)) = tokens.get(2) else {
                return Err(Error::new(
                    token_span(&tokens[1..]),
                    ErrorKind::MissingAfterSign,
                ));
            };
            let _ = checked_literal(literal)?;
            return Ok(Some(HASH_OFFSET));
        }
        return Ok(Some(ONE_TOKEN));
    }
    if is_punct(tokens, 0, '-') {
        let Some(TokenTree::Literal(literal)) = tokens.get(1) else {
            return Ok(None);
        };
        let _ = checked_literal(literal)?;
        return Ok(Some(SIGNED_LITERAL));
    }
    if let Some(TokenTree::Literal(literal)) = tokens.first() {
        let _ = checked_literal(literal)?;
        return Ok(Some(ONE_TOKEN));
    }
    Ok(None)
}

fn opens_as_bound(tokens: &[TokenTree]) -> Result<bool> {
    Ok(bound_prefix(tokens)?.is_some())
}

fn checked_literal(literal: &Literal) -> Result<i128> {
    let stream = TokenStream::from(TokenTree::Literal(literal_for_syn(literal)));
    let parsed = syn::parse2::<Lit>(stream)
        .map_err(|_| Error::new(literal.span(), ErrorKind::UnsuffixedLiteral))?;
    match parsed {
        Lit::Int(int) => plain_int(&int),
        _ => Err(Error::new(literal.span(), ErrorKind::UnsuffixedLiteral)),
    }
}

fn reject_hash_tails(tokens: &[TokenTree]) -> Result<()> {
    let mut index = 0;
    while index < tokens.len() {
        if is_punct(tokens, index, '#')
            && is_sign(tokens, index + 1)
            && !is_literal(tokens, index + 2)
        {
            return Err(Error::new(
                token_span(&tokens[index + 1..]),
                ErrorKind::MissingAfterSign,
            ));
        }
        index += 1;
    }
    Ok(())
}

fn step_through(
    start: i128,
    end: i128,
    inclusive: bool,
    step: i128,
    mut keep: impl FnMut(i128) -> bool,
    visit: &mut impl FnMut(i128) -> Result<()>,
) -> Result<()> {
    let Ok(stride) = usize::try_from(step) else {
        return step_wide(start, end, inclusive, step, keep, visit);
    };
    if inclusive {
        for index in (start..=end).step_by(stride) {
            if keep(index) {
                visit(index)?;
            }
        }
    } else {
        for index in (start..end).step_by(stride) {
            if keep(index) {
                visit(index)?;
            }
        }
    }
    Ok(())
}

fn step_wide(
    start: i128,
    end: i128,
    inclusive: bool,
    step: i128,
    mut keep: impl FnMut(i128) -> bool,
    visit: &mut impl FnMut(i128) -> Result<()>,
) -> Result<()> {
    let mut current = start;
    loop {
        let past = if inclusive {
            current > end
        } else {
            current >= end
        };
        if past {
            break;
        }
        if keep(current) {
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

fn starts_with_range_op(tokens: &[TokenTree]) -> bool {
    is_joint_pair(tokens, 0, '.', '.')
}

fn contains_range_op(tokens: &[TokenTree]) -> bool {
    (0..tokens.len()).any(|index| is_joint_pair(tokens, index, '.', '.'))
}

fn is_joint_pair(tokens: &[TokenTree], index: usize, first: char, second: char) -> bool {
    match (tokens.get(index), tokens.get(index + 1)) {
        (Some(TokenTree::Punct(left)), Some(TokenTree::Punct(right))) => {
            left.as_char() == first && left.spacing() == Spacing::Joint && right.as_char() == second
        }
        _ => false,
    }
}

fn is_punct(tokens: &[TokenTree], index: usize, mark: char) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == mark)
}

fn is_sign(tokens: &[TokenTree], index: usize) -> bool {
    is_punct(tokens, index, '+') || is_punct(tokens, index, '-')
}

fn is_literal(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Literal(_)))
}

fn token_span(tokens: &[TokenTree]) -> Span {
    tokens.first().map_or_else(Span::call_site, TokenTree::span)
}

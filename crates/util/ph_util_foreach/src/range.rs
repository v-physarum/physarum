//! Range syntax, domain checks, and index lists.
//!
//! Outer ranges and `Name[<pat>]` patterns share one parser. `syn` reads the
//! tokens as an [`Expr`]: a range ([`ExprRange`]) or `(range).step_by(step)`. Each
//! integer is a [`LitInt`]. The index is the loop variable. `end` is the last
//! domain index, and only as a pattern bound. `-1` is the integer negative one.
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

use proc_macro2::{Spacing, Span, TokenStream, TokenTree};
use syn::{
    BinOp, Expr, ExprMethodCall, ExprPath, ExprRange, ExprUnary, Lit, LitInt, PathArguments,
    RangeLimits, UnOp,
};

use crate::error::{Error, ErrorKind, Result};

/// How a bound is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundStyle {
    /// Outer range: integer literals only.
    Outer,
    /// Pattern bound: integer literals, the loop variable ± an integer, and `end`.
    Pattern,
    /// Step: a positive integer literal, or the loop variable ± an integer.
    Step,
}

/// A bound before it is evaluated.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Bound {
    /// An integer literal, already signed.
    Int(i128),
    /// The path `end` in a pattern: last index of the domain.
    LastIndex,
    /// The loop variable plus `offset` (`n - 1` is `Here { offset: -1 }`).
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

    /// Whether a bound mentions the loop variable.
    ///
    /// A step that mentions the variable does not count: the domain stays the
    /// flat outer set, and only the stride changes with the index.
    pub(crate) fn mentions_index(self) -> bool {
        let start = self.start.is_some_and(SpannedBound::mentions_index);
        let end = self.end.is_some_and(SpannedBound::mentions_index);
        start || end
    }

    /// Parses `tokens` as one outer range or `(range).step_by(step)`.
    ///
    /// `index` is the loop variable. Using it, or `end`, in the outer range is an error.
    pub(crate) fn parse(tokens: &[TokenTree], index: &str, style: BoundStyle) -> Result<Self> {
        if tokens.is_empty() {
            return Err(Error::new(Span::call_site(), ErrorKind::ExpectedRange));
        }
        let span = token_span(tokens);
        let expr =
            syn_expr(tokens).map_err(|error| Error::new(error.span(), ErrorKind::ExpectedDots))?;
        match opened(&expr) {
            Opened::Range(range) => range_from_range(range, index, style, span),
            Opened::Step(call) => range_from_method(call, index, style, span),
            Opened::Other => Err(Error::new(span, ErrorKind::ExpectedDots)),
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
        // `repeat(T, n + 1..=n)` when `n` is the last index: the window is empty
        // and the start sits one past the domain. That hole edge is allowed.
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

    fn build(self) -> RangeExpr {
        RangeExpr::new(self)
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
    /// Current index.
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

/// A bracket's expression, after parentheses are stripped.
enum Opened<'a> {
    Range(&'a ExprRange),
    Step(&'a ExprMethodCall),
    Other,
}

/// Parses one bracket as a pattern.
///
/// `None` means the brackets are ordinary Rust and stay as written. A lone
/// loop variable is not a range: `Name[n]` stays `Name[n]`.
pub(crate) fn classify_bracket(
    tokens: &[TokenTree],
    index: &str,
    span: Span,
) -> Result<Option<RangeExpr>> {
    if tokens.is_empty() {
        return Ok(None);
    }
    let expr = match syn_expr(tokens) {
        Ok(expr) => expr,
        Err(_) if looks_like_range(tokens) => {
            return Err(Error::new(span, ErrorKind::OnePattern));
        }
        Err(_) => return Ok(None),
    };
    match opened(&expr) {
        Opened::Range(range) => Ok(Some(range_from_range(
            range,
            index,
            BoundStyle::Pattern,
            span,
        )?)),
        Opened::Step(call) => Ok(Some(range_from_method(
            call,
            index,
            BoundStyle::Pattern,
            span,
        )?)),
        Opened::Other => Ok(None),
    }
}

fn opened(expr: &Expr) -> Opened<'_> {
    match expr {
        Expr::MethodCall(call) if call.method == "step_by" => Opened::Step(call),
        Expr::Paren(inner) => opened(&inner.expr),
        Expr::Group(inner) => opened(&inner.expr),
        Expr::Range(range) => Opened::Range(range),
        _ => Opened::Other,
    }
}

fn range_from_range(
    range: &ExprRange,
    index: &str,
    style: BoundStyle,
    span: Span,
) -> Result<RangeExpr> {
    if side_has_range(range.start.as_deref()) || side_has_range(range.end.as_deref()) {
        return Err(trailing(style, span));
    }
    let start = bound_from(range.start.as_deref(), index, style, trailing_kind(style))?;
    let end = bound_from(range.end.as_deref(), index, style, trailing_kind(style))?;
    let end_inclusive = match range.limits {
        RangeLimits::Closed(_) => true,
        RangeLimits::HalfOpen(_) => false,
    };
    Ok(RangeExpr::builder(span)
        .start(start)
        .end(end)
        .end_inclusive(end_inclusive)
        .build())
}

fn range_from_method(
    call: &ExprMethodCall,
    index: &str,
    style: BoundStyle,
    span: Span,
) -> Result<RangeExpr> {
    let mut args = call.args.iter();
    let Some(step) = args.next() else {
        return Err(Error::new(span, ErrorKind::ExpectedStep));
    };
    if args.next().is_some() {
        return Err(Error::new(span, ErrorKind::ExpectedStep));
    }
    let Opened::Range(range) = opened(&call.receiver) else {
        return Err(trailing(style, span));
    };
    let mut built = range_from_range(range, index, style, span)?;
    built.step = Some(interpret(
        step,
        index,
        BoundStyle::Step,
        ErrorKind::StepLastIndex,
    )?);
    Ok(built)
}

fn bound_from(
    expr: Option<&Expr>,
    index: &str,
    style: BoundStyle,
    shape: ErrorKind,
) -> Result<Option<SpannedBound>> {
    match expr {
        None => Ok(None),
        Some(expr) => interpret(expr, index, style, shape).map(Some),
    }
}

fn interpret(
    expr: &Expr,
    index: &str,
    style: BoundStyle,
    shape: ErrorKind,
) -> Result<SpannedBound> {
    match expr {
        Expr::Paren(inner) => interpret(&inner.expr, index, style, shape),
        Expr::Group(inner) => interpret(&inner.expr, index, style, shape),
        Expr::Lit(literal) => lit_bound(&literal.lit, false, literal.lit.span()),
        Expr::Unary(unary) => unary_bound(unary, shape),
        Expr::Path(path) => path_bound(path, index, style, shape),
        Expr::Binary(binary) => index_offset(binary, index, style, shape),
        other => Err(Error::new(expr_span(other), shape)),
    }
}

fn path_bound(
    path: &ExprPath,
    index: &str,
    style: BoundStyle,
    shape: ErrorKind,
) -> Result<SpannedBound> {
    let Some(name) = plain_ident(path) else {
        return Err(Error::new(path_span(path), shape));
    };
    let span = name.span();
    if name == index {
        return here_or_outer(span, style);
    }
    if name == "end" {
        return end_bound(span, style);
    }
    Err(Error::new(span, shape))
}

fn here_or_outer(span: Span, style: BoundStyle) -> Result<SpannedBound> {
    if style == BoundStyle::Outer {
        Err(Error::new(span, ErrorKind::OuterHash))
    } else {
        Ok(SpannedBound::here(0, span))
    }
}

fn end_bound(span: Span, style: BoundStyle) -> Result<SpannedBound> {
    match style {
        BoundStyle::Outer => Err(Error::new(span, ErrorKind::OuterHash)),
        BoundStyle::Pattern => Ok(SpannedBound::last_index(span)),
        BoundStyle::Step => Err(Error::new(span, ErrorKind::StepLastIndex)),
    }
}

fn index_offset(
    binary: &syn::ExprBinary,
    index: &str,
    style: BoundStyle,
    shape: ErrorKind,
) -> Result<SpannedBound> {
    if !is_index_expr(&binary.left, index) {
        return Err(Error::new(expr_span(&binary.left), shape));
    }
    if style == BoundStyle::Outer {
        return Err(Error::new(expr_span(&binary.left), ErrorKind::OuterHash));
    }
    let Expr::Lit(literal) = binary.right.as_ref() else {
        return Err(Error::new(
            expr_span(&binary.right),
            ErrorKind::UnsuffixedLiteral,
        ));
    };
    let Lit::Int(int) = &literal.lit else {
        return Err(Error::new(literal.lit.span(), ErrorKind::UnsuffixedLiteral));
    };
    let magnitude = plain_int(int)?;
    let span = expr_span(&binary.left);
    let offset = match &binary.op {
        BinOp::Add(_) => magnitude,
        BinOp::Sub(_) => magnitude
            .checked_neg()
            .ok_or_else(|| Error::new(span, ErrorKind::LiteralOverflow))?,
        _ => return Err(Error::new(span, shape)),
    };
    Ok(SpannedBound::here(offset, span))
}

fn is_index_expr(expr: &Expr, index: &str) -> bool {
    match expr {
        Expr::Paren(inner) => is_index_expr(&inner.expr, index),
        Expr::Group(inner) => is_index_expr(&inner.expr, index),
        Expr::Path(path) => plain_ident(path).is_some_and(|name| name == index),
        _ => false,
    }
}

fn unary_bound(unary: &ExprUnary, shape: ErrorKind) -> Result<SpannedBound> {
    let UnOp::Neg(minus) = &unary.op else {
        return Err(Error::new(expr_span(&unary.expr), shape));
    };
    match unary.expr.as_ref() {
        Expr::Lit(literal) => lit_bound(&literal.lit, true, minus.span),
        _ => Err(Error::new(minus.span, shape)),
    }
}

fn lit_bound(lit: &Lit, negative: bool, span: Span) -> Result<SpannedBound> {
    let Lit::Int(int) = lit else {
        return Err(Error::new(span, ErrorKind::UnsuffixedLiteral));
    };
    let magnitude = plain_int(int)?;
    if !negative {
        return Ok(SpannedBound::int(magnitude, int.span()));
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

fn plain_ident(path: &ExprPath) -> Option<&proc_macro2::Ident> {
    if !path.attrs.is_empty() || path.qself.is_some() || path.path.leading_colon.is_some() {
        return None;
    }
    let mut segments = path.path.segments.iter();
    let segment = segments.next()?;
    if segments.next().is_some() || !matches!(segment.arguments, PathArguments::None) {
        return None;
    }
    Some(&segment.ident)
}

fn path_span(path: &ExprPath) -> Span {
    path.path
        .segments
        .first()
        .map_or_else(Span::call_site, |segment| segment.ident.span())
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Lit(literal) => literal.lit.span(),
        Expr::Path(path) => path_span(path),
        Expr::Unary(unary) => match &unary.op {
            UnOp::Neg(token) => token.span,
            UnOp::Not(token) => token.span,
            UnOp::Deref(token) => token.span,
            _ => Span::call_site(),
        },
        Expr::Paren(inner) => expr_span(&inner.expr),
        Expr::Group(inner) => expr_span(&inner.expr),
        Expr::Binary(binary) => expr_span(&binary.left),
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
        Expr::Call(call) => call.args.iter().any(expr_has_range),
        Expr::MethodCall(call) => {
            expr_has_range(&call.receiver) || call.args.iter().any(expr_has_range)
        }
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

fn syn_expr(tokens: &[TokenTree]) -> std::result::Result<Expr, syn::Error> {
    let stream = TokenStream::from_iter(tokens.iter().cloned());
    syn::parse2(stream)
}

fn looks_like_range(tokens: &[TokenTree]) -> bool {
    let mut previous_dot = false;
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                let nested: Vec<TokenTree> = group.stream().into_iter().collect();
                if looks_like_range(&nested) {
                    return true;
                }
                previous_dot = false;
            }
            TokenTree::Punct(punct)
                if punct.as_char() == '.' && punct.spacing() == Spacing::Joint =>
            {
                previous_dot = true;
            }
            TokenTree::Punct(punct) if previous_dot && punct.as_char() == '.' => return true,
            _ => previous_dot = false,
        }
    }
    false
}

fn token_span(tokens: &[TokenTree]) -> Span {
    tokens.first().map_or_else(Span::call_site, TokenTree::span)
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

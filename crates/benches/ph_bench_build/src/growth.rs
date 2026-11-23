//! Scale series and the growth claim checked against measured times.
//!
//! A series is one case generator, the scales `N` it is rendered at, and a
//! claim: linear, or not quadratic. The exponent is the least-squares slope of
//! `ln(time)` on `ln(N)`. Two scales reduce to the log ratio
//! `ln(t2 / t1) / ln(n2 / n1)`.

use std::fmt;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use crate::error::{Error, ErrorKind, Result};

/// Exponent of exact linear growth, `time ∝ N`.
pub const LINEAR_EXPONENT: f64 = 1.0;

/// Exponent of exact quadratic growth, `time ∝ N²`.
pub const QUADRATIC_EXPONENT: f64 = 2.0;

/// How far an exponent may sit from 1 or from 2 and still count as that shape.
///
/// Wall-clock compiles jitter. [`LINEAR_LIMIT`] is [`LINEAR_EXPONENT`] plus
/// this value. [`QUADRATIC_LIMIT`] is [`QUADRATIC_EXPONENT`] minus this value.
///
/// On a four-times size step, linear still holds when the larger case takes at
/// most `4^1.25` (about 5.66) times as long. That step counts as quadratic once
/// it takes at least `4^1.75` (about 11.31) times as long. A pure quadratic
/// step takes 16 times as long.
pub const NOISE_ALLOWANCE: f64 = 0.25;

/// Largest exponent that still counts as linear.
///
/// Sublinear samples count as linear. Fixed compiler startup flattens a short
/// span, and that is still at most linear.
pub const LINEAR_LIMIT: f64 = LINEAR_EXPONENT + NOISE_ALLOWANCE;

/// Smallest exponent that counts as quadratic.
pub const QUADRATIC_LIMIT: f64 = QUADRATIC_EXPONENT - NOISE_ALLOWANCE;

/// Fewest scales that can show growth.
pub const MIN_SCALES: usize = 2;

/// Digits after the decimal in a `-Ztime-passes` second count.
const MILLIS_DIGITS: usize = 3;

/// Ceiling for [`RustcPass::TypeCheckCrate`] at scale 16.
///
/// Function items at that scale take about 8 ms. The closure form of the same
/// chain takes about 67 ms. [`Growth::Linear`] from scale 1 to scale 16 still
/// passes that closure chain: the large build must take more than 32 times as
/// long (`16` to the power [`LINEAR_LIMIT`]) before the claim fails.
pub const TYPE_CHECK_CRATE_LIMIT: Duration = Duration::from_millis(20);

/// Smallest scale. Zero has no logarithm.
pub const MIN_SCALE: u32 = 1;

/// Which measured builds a series belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Full debug build. The package is cleaned before each run.
    Debug,
    /// Incremental debug build. The package is not cleaned first.
    Incremental,
    /// Full release build. The package is cleaned before each run.
    Release,
}

impl fmt::Display for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Debug => "debug",
            Self::Incremental => "incremental",
            Self::Release => "release",
        };
        formatter.write_str(label)
    }
}

/// Growth a series must show.
///
/// See [`LINEAR_LIMIT`] and [`QUADRATIC_LIMIT`]. An exponent between those
/// limits is superlinear: [`Growth::Linear`] fails and [`Growth::NotQuadratic`]
/// passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Growth {
    /// Exponent at or below [`LINEAR_LIMIT`].
    Linear,
    /// Exponent below [`QUADRATIC_LIMIT`].
    NotQuadratic,
}

impl Growth {
    /// Fits `points` and checks that the fit meets this claim.
    ///
    /// # Errors
    ///
    /// Returns an error when `points` cannot be fit, or the fit misses this claim.
    pub fn check(self, points: &[Point]) -> Result<Fit> {
        judge("growth", self, points)
    }
}

/// Shape read off an exponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Character {
    /// Exponent at or below [`LINEAR_LIMIT`].
    Linear,
    /// Exponent above [`LINEAR_LIMIT`] and below [`QUADRATIC_LIMIT`].
    Superlinear,
    /// Exponent at or above [`QUADRATIC_LIMIT`].
    Quadratic,
}

impl Character {
    /// Classifies `exponent`.
    ///
    /// A non-finite exponent counts as quadratic, so both claims fail.
    #[must_use]
    pub fn from_exponent(exponent: f64) -> Self {
        if exponent <= LINEAR_LIMIT {
            Self::Linear
        } else if exponent < QUADRATIC_LIMIT {
            Self::Superlinear
        } else {
            Self::Quadratic
        }
    }
}

impl fmt::Display for Character {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Linear => "linear",
            Self::Superlinear => "superlinear",
            Self::Quadratic => "quadratic",
        };
        formatter.write_str(label)
    }
}

/// Item count `N` for one generated case.
///
/// [`Series::check`] and [`Growth::check`] reject a count below [`MIN_SCALE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scale(u32);

impl Scale {
    /// A scale of `count` items.
    #[must_use]
    pub const fn new(count: u32) -> Self {
        Self(count)
    }

    /// The item count.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0
    }
}

/// One measured point: scale `N` and its wall-clock seconds.
#[derive(Clone, Copy, Debug)]
pub struct Point {
    scale: Scale,
    seconds: f64,
}

impl Point {
    /// `seconds` is the wall-clock time of a case of size `scale`.
    #[must_use]
    pub const fn new(scale: Scale, seconds: f64) -> Self {
        Self { scale, seconds }
    }
}

/// One rustc pass named by `-Ztime-passes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustcPass {
    /// The `type_check_crate` pass.
    TypeCheckCrate,
}

impl RustcPass {
    /// The name rustc prints for this pass.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TypeCheckCrate => "type_check_crate",
        }
    }
}

/// Trait solver for a nightly pass measurement.
///
/// Stable hyperfine series keep [`Self::Default`]. [`Self::Next`] adds
/// `-Znext-solver=globally`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Solver {
    /// The compiler's default solver.
    #[default]
    Default,
    /// `-Znext-solver=globally`.
    Next,
}

impl Solver {
    pub(crate) fn rustc_args(self) -> &'static [&'static str] {
        match self {
            Self::Default => &["-Ztime-passes"],
            Self::Next => &["-Ztime-passes", "-Znext-solver=globally"],
        }
    }
}

/// Ceiling on one rustc pass at one scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassCeiling {
    pass: RustcPass,
    scale: Scale,
    limit: Duration,
}

impl PassCeiling {
    /// `pass` at `scale` must finish in `limit` or less.
    #[must_use]
    pub const fn new(pass: RustcPass, scale: Scale, limit: Duration) -> Self {
        Self { pass, scale, limit }
    }

    /// The pass this ceiling times.
    #[must_use]
    pub const fn pass(self) -> RustcPass {
        self.pass
    }

    /// The scale this ceiling applies to.
    #[must_use]
    pub const fn scale(self) -> Scale {
        self.scale
    }

    /// The longest allowed time.
    #[must_use]
    pub const fn limit(self) -> Duration {
        self.limit
    }

    /// Checks one reported pass time.
    ///
    /// `seconds` is the number rustc printed, already rounded to milliseconds.
    /// Zero is inside the ceiling. A non-finite or negative time is not.
    ///
    /// # Errors
    ///
    /// Returns an error when `seconds` is not a finite non-negative time, or
    /// when it is above [`Self::limit`].
    pub fn check_seconds(self, seconds: f64) -> Result<Duration> {
        let Some(elapsed_ms) = reported_millis(seconds) else {
            return Err(ErrorKind::PassTime {
                pass: self.pass.name().to_owned(),
                scale: self.scale.count(),
            }
            .into());
        };
        let limit_ms = self.limit.as_millis();
        if elapsed_ms > limit_ms {
            return Err(ErrorKind::PassSlow {
                pass: self.pass.name().to_owned(),
                scale: self.scale.count(),
                elapsed_ms,
                limit_ms,
            }
            .into());
        }
        Duration::try_from_secs_f64(seconds).map_err(|_| {
            ErrorKind::PassTime { pass: self.pass.name().to_owned(), scale: self.scale.count() }
                .into()
        })
    }
}

/// One measured pass under its ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassSample {
    scale: Scale,
    elapsed: Duration,
    limit: Duration,
}

impl PassSample {
    /// The scale that was timed.
    #[must_use]
    pub const fn scale(self) -> Scale {
        self.scale
    }

    /// The pass time rustc reported.
    #[must_use]
    pub const fn elapsed(self) -> Duration {
        self.elapsed
    }

    /// The ceiling that accepted this time.
    #[must_use]
    pub const fn limit(self) -> Duration {
        self.limit
    }
}

/// What [`Series::check`] accepted.
#[derive(Clone, Copy, Debug)]
pub enum Verdict {
    /// Log-log fit of hyperfine wall-clock builds.
    Growth(Fit),
    /// One nightly rustc pass under its ceiling.
    Pass(PassSample),
}

/// Milliseconds of a `-Ztime-passes` second count, printed to three decimals.
fn reported_millis(seconds: f64) -> Option<u128> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let text = format!("{seconds:.3}");
    let (whole, frac) = text.split_once('.')?;
    if frac.len() != MILLIS_DIGITS {
        return None;
    }
    let secs: u128 = whole.parse().ok()?;
    let millis: u128 = frac.parse().ok()?;
    secs.checked_mul(1000)?.checked_add(millis)
}

/// Log-log fit of `seconds ∝ N^exponent`.
#[derive(Clone, Copy, Debug)]
pub struct Fit {
    exponent: f64,
}

impl Fit {
    /// The fitted exponent.
    #[must_use]
    pub const fn exponent(self) -> f64 {
        self.exponent
    }

    /// The shape of this exponent under [`LINEAR_LIMIT`] and [`QUADRATIC_LIMIT`].
    #[must_use]
    pub fn character(self) -> Character {
        Character::from_exponent(self.exponent)
    }
}

/// One generator rendered at several scales, plus the growth those times must show.
///
/// ```no_run
/// use ph_bench_build::{Group, Growth, Series};
///
/// # fn main() {
/// Series::new("hand_structs")
///     .group(Group::Debug)
///     .scales([64, 1024, 8192])
///     .expect(Growth::Linear)
///     .render(|n| format!("pub struct Ix{};\n", n.count()))
///     .check()
///     .unwrap();
/// # }
/// ```
#[must_use]
#[derive(Debug)]
pub struct Series {
    name: String,
    group: Option<Group>,
    scales: Vec<Scale>,
    expect: Option<Growth>,
    ceiling: Option<PassCeiling>,
    solver: Solver,
    sources: Vec<String>,
    rendered: bool,
}

impl Series {
    /// Starts a series whose case names are `{name}_{n}`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            group: None,
            scales: Vec::new(),
            expect: None,
            ceiling: None,
            solver: Solver::Default,
            sources: Vec::new(),
            rendered: false,
        }
    }

    /// Debug, incremental, or release runs of this series.
    pub fn group(mut self, group: Group) -> Self {
        self.group = Some(group);
        self
    }

    /// Scales `N`, in the order the generator and [`Self::check_seconds`] use.
    ///
    /// The order is kept. A later fit sorts its own copy.
    pub fn scales(mut self, scales: impl IntoIterator<Item = u32>) -> Self {
        self.scales.clear();
        self.scales.extend(scales.into_iter().map(Scale::new));
        self.sources.clear();
        self.rendered = false;
        self
    }

    /// The growth claim checked after hyperfine measures the series.
    ///
    /// This is the stable path. A pass ceiling is a different claim.
    pub fn expect(mut self, growth: Growth) -> Self {
        self.expect = Some(growth);
        self
    }

    /// Checks one nightly rustc pass at one scale instead of a growth claim.
    ///
    /// [`Self::check`] compiles with `-Ztime-passes` and reads that pass on
    /// the user crate. Stable rustc has no `-Ztime-passes`.
    pub fn ceiling(mut self, ceiling: PassCeiling) -> Self {
        self.ceiling = Some(ceiling);
        self
    }

    /// Selects the trait solver for a pass ceiling.
    ///
    /// [`Solver::Default`] leaves the compiler default. [`Solver::Next`] passes
    /// `-Znext-solver=globally`. Hyperfine series ignore this.
    pub fn solver(mut self, solver: Solver) -> Self {
        self.solver = solver;
        self
    }

    /// Renders one source per scale. `generate` receives `N`.
    ///
    /// The source is stored on the series. [`Self::check`] writes it to the
    /// downstream crate. Call [`Self::group`], [`Self::scales`], and either
    /// [`Self::expect`] or [`Self::ceiling`] before [`Self::check`].
    pub fn render(mut self, mut generate: impl FnMut(Scale) -> String) -> Self {
        self.sources.clear();
        self.sources.reserve(self.scales.len());
        for scale in &self.scales {
            self.sources.push(generate(*scale));
        }
        self.rendered = true;
        self
    }

    /// Case kind, for example `flat_structs`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Scales in declaration order.
    #[must_use]
    pub fn scale_values(&self) -> &[Scale] {
        &self.scales
    }

    /// Source rendered for `scale`, when this series has that scale.
    #[must_use]
    pub fn source(&self, scale: Scale) -> Option<&str> {
        let index = self.scales.iter().position(|item| *item == scale)?;
        self.sources.get(index).map(String::as_str)
    }

    /// Builds each rendered source and checks the claim.
    ///
    /// [`Self::expect`] times stable hyperfine builds and checks the log-log
    /// exponent. [`Self::ceiling`] times one nightly rustc pass with
    /// `-Ztime-passes`. The workspace root is the ancestor of
    /// `CARGO_MANIFEST_DIR` whose manifest lists members. `use` paths in the
    /// rendered source that name a member become path dependencies. The user
    /// package is `{name}_user`. Output is `{target}/ph_bench_build/{name}-{group}`.
    ///
    /// # Errors
    ///
    /// Returns an error when the series is unfinished, both claims are set, a
    /// path cannot be resolved, a build fails, or the measured times miss the
    /// claim. A pass ceiling also fails when nightly rustc is missing.
    pub fn check(&self) -> Result<Verdict> {
        static BUILD: Mutex<()> = Mutex::new(());
        let _guard = BUILD.lock().unwrap_or_else(PoisonError::into_inner);
        if self.ceiling.is_some() && self.expect.is_some() {
            return Err(ErrorKind::SeriesBothClaims { series: self.name.clone() }.into());
        }
        if self.ceiling.is_some() {
            return self.check_ceiling();
        }
        self.check_growth()
    }

    fn check_growth(&self) -> Result<Verdict> {
        let (group, _) = self.ensure_growth()?;
        let home = crate::workspace::series_home(&self.name, group, &self.sources)?;
        let seconds = crate::engine::measure_series(&crate::engine::SeriesBuild {
            name: &self.name,
            group,
            scales: &self.scales,
            sources: &self.sources,
            home: &home,
        })?;
        let fit = self.check_seconds(&seconds)?;
        println!(
            "{group} {} exponent {:.3} {}",
            self.name,
            fit.exponent(),
            fit.character()
        );
        Ok(Verdict::Growth(fit))
    }

    fn check_ceiling(&self) -> Result<Verdict> {
        let group = self.ensure_rendered()?;
        let ceiling = self
            .ceiling
            .ok_or_else(|| Error::from(ErrorKind::SeriesExpect { series: self.name.clone() }))?;
        if !self.scales.contains(&ceiling.scale) {
            return Err(ErrorKind::PassScale {
                series: self.name.clone(),
                scale: ceiling.scale.count(),
            }
            .into());
        }
        let home = crate::workspace::series_home(&self.name, group, &self.sources)?;
        let seconds = crate::engine::measure_passes(&crate::engine::PassBuild {
            name: &self.name,
            group,
            scales: &self.scales,
            sources: &self.sources,
            home: &home,
            pass: ceiling.pass.name(),
            rustc_args: self.solver.rustc_args(),
        })?;
        let index = self.scales.iter().position(|scale| *scale == ceiling.scale);
        let Some(index) = index else {
            return Err(ErrorKind::PassScale {
                series: self.name.clone(),
                scale: ceiling.scale.count(),
            }
            .into());
        };
        let elapsed = ceiling.check_seconds(seconds[index])?;
        println!(
            "{group} {} {} scale {} {:.3}s limit {:.3}s",
            self.name,
            ceiling.pass.name(),
            ceiling.scale.count(),
            elapsed.as_secs_f64(),
            ceiling.limit.as_secs_f64()
        );
        Ok(Verdict::Pass(PassSample {
            scale: ceiling.scale,
            elapsed,
            limit: ceiling.limit,
        }))
    }

    /// Checks `seconds` against this series.
    ///
    /// Each time is paired with the scale at the same index in [`Self::scale_values`].
    ///
    /// # Errors
    ///
    /// Returns an error when the series was not rendered, the slice length
    /// differs from the scales, the times cannot be fit, or the fit misses the claim.
    pub fn check_seconds(&self, seconds: &[f64]) -> Result<Fit> {
        let (_, expect) = self.ensure_growth()?;
        if seconds.len() != self.scales.len() {
            return Err(ErrorKind::SeriesTimeCount {
                series: self.name.clone(),
                scales: self.scales.len(),
                times: seconds.len(),
            }
            .into());
        }
        let mut points = Vec::with_capacity(self.scales.len());
        for (scale, seconds) in self.scales.iter().copied().zip(seconds) {
            points.push(Point::new(scale, *seconds));
        }
        judge(&self.name, expect, &points)
    }

    fn prepare(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(ErrorKind::SeriesName.into());
        }
        if self.group.is_none() {
            return Err(ErrorKind::SeriesGroup { series: self.name.clone() }.into());
        }
        if self.scales.len() < MIN_SCALES {
            return Err(
                ErrorKind::SeriesTooFew { series: self.name.clone(), min: MIN_SCALES }.into(),
            );
        }
        for (index, scale) in self.scales.iter().enumerate() {
            let count = scale.count();
            if count < MIN_SCALE {
                return Err(
                    ErrorKind::SeriesScale { series: self.name.clone(), scale: count }.into(),
                );
            }
            if self.scales[..index].contains(scale) {
                return Err(
                    ErrorKind::SeriesDuplicate { series: self.name.clone(), scale: count }.into(),
                );
            }
        }
        Ok(())
    }

    fn ensure_rendered(&self) -> Result<Group> {
        self.prepare()?;
        if !self.rendered || self.sources.len() != self.scales.len() {
            return Err(ErrorKind::SeriesSources { series: self.name.clone() }.into());
        }
        for (scale, source) in self.scales.iter().zip(&self.sources) {
            if source.is_empty() {
                return Err(ErrorKind::SeriesSource {
                    series: self.name.clone(),
                    scale: scale.count(),
                }
                .into());
            }
        }
        self.group
            .ok_or_else(|| Error::from(ErrorKind::SeriesGroup { series: self.name.clone() }))
    }

    fn ensure_growth(&self) -> Result<(Group, Growth)> {
        let group = self.ensure_rendered()?;
        let expect = self
            .expect
            .ok_or_else(|| Error::from(ErrorKind::SeriesExpect { series: self.name.clone() }))?;
        Ok((group, expect))
    }
}

fn judge(series: &str, expect: Growth, points: &[Point]) -> Result<Fit> {
    let exponent = exponent(series, points)?;
    let fit = Fit { exponent };
    match (expect, fit.character()) {
        (Growth::Linear, Character::Linear)
        | (Growth::NotQuadratic, Character::Linear | Character::Superlinear) => Ok(fit),
        (Growth::Linear, _) => {
            Err(
                ErrorKind::NotLinear { series: series.to_owned(), exponent, limit: LINEAR_LIMIT }
                    .into(),
            )
        }
        (Growth::NotQuadratic, Character::Quadratic) => Err(ErrorKind::QuadraticGrowth {
            series: series.to_owned(),
            exponent,
            limit: QUADRATIC_LIMIT,
        }
        .into()),
    }
}

fn exponent(series: &str, points: &[Point]) -> Result<f64> {
    if points.len() < MIN_SCALES {
        return Err(ErrorKind::SeriesTooFew { series: series.to_owned(), min: MIN_SCALES }.into());
    }
    let mut samples = Vec::with_capacity(points.len());
    for point in points {
        let scale = point.scale.count();
        if scale < MIN_SCALE {
            return Err(ErrorKind::SeriesScale { series: series.to_owned(), scale }.into());
        }
        if !point.seconds.is_finite() || point.seconds <= 0.0 {
            return Err(ErrorKind::SeriesTime { series: series.to_owned(), scale }.into());
        }
        if samples.iter().any(|(seen, _)| *seen == scale) {
            return Err(ErrorKind::SeriesDuplicate { series: series.to_owned(), scale }.into());
        }
        samples.push((scale, point.seconds));
    }
    samples.sort_unstable_by_key(|(scale, _)| *scale);
    let mut logs = Vec::with_capacity(samples.len());
    for (scale, seconds) in samples {
        logs.push((f64::from(scale).ln(), seconds.ln()));
    }
    let slope = slope(&logs);
    if slope.is_finite() {
        Ok(slope)
    } else {
        Err(ErrorKind::SeriesLog { series: series.to_owned() }.into())
    }
}

/// Least-squares slope of `ln(time)` on `ln(N)`.
fn slope(logs: &[(f64, f64)]) -> f64 {
    let mut count = 0.0;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for &(x, y) in logs {
        count += 1.0;
        sum_x += x;
        sum_y += y;
    }
    let mean_x = sum_x / count;
    let mean_y = sum_y / count;
    let mut numer = 0.0;
    let mut denom = 0.0;
    for &(x, y) in logs {
        let dx = x - mean_x;
        numer += dx * (y - mean_y);
        denom += dx * dx;
    }
    numer / denom
}

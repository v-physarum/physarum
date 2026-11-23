//! Scale series and the growth claim checked against measured times.
//!
//! A series is one case generator, the scales `N` it is rendered at, and a
//! claim: linear, or not quadratic. The exponent is the least-squares slope of
//! `ln(time)` on `ln(N)`. Two scales reduce to the log ratio
//! `ln(t2 / t1) / ln(n2 / n1)`.

use std::fmt;
use std::sync::{Mutex, PoisonError};

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

    /// The claim checked after the series is measured.
    pub fn expect(mut self, growth: Growth) -> Self {
        self.expect = Some(growth);
        self
    }

    /// Renders one source per scale. `generate` receives `N`.
    ///
    /// The source is stored on the series. [`Self::check`] writes it to the
    /// downstream crate. Call [`Self::group`], [`Self::scales`], and
    /// [`Self::expect`] before [`Self::check`].
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

    /// Builds each rendered source and checks the measured times.
    ///
    /// The workspace root is the ancestor of `CARGO_MANIFEST_DIR` whose
    /// manifest lists members. `use` paths in the rendered source that name a
    /// member become path dependencies. The user package is `{name}_user`.
    /// Output is `{target}/ph_bench_build/{name}-{group}`.
    ///
    /// # Errors
    ///
    /// Returns an error when the series is unfinished, a path cannot be
    /// resolved, a build fails, or the measured times miss the claim.
    pub fn check(&self) -> Result<Fit> {
        static BUILD: Mutex<()> = Mutex::new(());
        let _guard = BUILD.lock().unwrap_or_else(PoisonError::into_inner);
        let (group, _) = self.ensure_ready()?;
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
        Ok(fit)
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
        let (_, expect) = self.ensure_ready()?;
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
        if self.expect.is_none() {
            return Err(ErrorKind::SeriesExpect { series: self.name.clone() }.into());
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

    fn ensure_ready(&self) -> Result<(Group, Growth)> {
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
        let group = self
            .group
            .ok_or_else(|| Error::from(ErrorKind::SeriesGroup { series: self.name.clone() }))?;
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

//! Readers for the JSON this bench consumes.
//!
//! `serde` and `serde_json` are built with default features off. Only `derive`
//! and `std` are enabled. Read failures are `thiserror` values.

use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ReadError {
    #[error("hyperfine json: {0}")]
    Hyperfine(serde_json::Error),
    #[error("hyperfine json has no time_wall_clock")]
    MissingWallClock,
    #[error("cargo json: {0}")]
    Cargo(serde_json::Error),
    #[error("runtime bench executable missing")]
    MissingExecutable,
}

pub(crate) struct WallClock {
    pub(crate) median: f64,
    pub(crate) stddev: f64,
}

impl WallClock {
    fn new(median: f64, stddev: f64) -> Self {
        Self { median, stddev }
    }

    /// Reads the wall-clock median and standard deviation from a hyperfine report.
    pub(crate) fn parse(json: &str) -> Result<Self, ReadError> {
        let report: HyperfineReport = serde_json::from_str(json).map_err(ReadError::Hyperfine)?;
        let Some(result) = report.results.first() else {
            return Err(ReadError::MissingWallClock);
        };
        let stat = &result.summary.time_wall_clock;
        Ok(Self::new(stat.median, stat.stddev))
    }
}

#[derive(Deserialize)]
struct HyperfineReport {
    results: Vec<HyperfineResult>,
}

#[derive(Deserialize)]
struct HyperfineResult {
    summary: HyperfineSummary,
}

#[derive(Deserialize)]
struct HyperfineSummary {
    time_wall_clock: HyperfineStat,
}

#[derive(Deserialize)]
struct HyperfineStat {
    median: f64,
    stddev: f64,
}

/// One line of cargo `--message-format=json`.
#[derive(Deserialize)]
pub(crate) struct CargoMessage {
    reason: String,
    #[serde(default)]
    executable: Option<String>,
    #[serde(default)]
    target: Option<CargoTarget>,
}

impl CargoMessage {
    /// The path of the `ph_util_foreach_runtime` artifact in a cargo JSON log.
    pub(crate) fn runtime_executable(log: &str) -> Result<PathBuf, ReadError> {
        for line in log.lines() {
            if line.is_empty() {
                continue;
            }
            let message: Self = serde_json::from_str(line).map_err(ReadError::Cargo)?;
            if message.reason != "compiler-artifact" {
                continue;
            }
            let Some(target) = message.target else {
                continue;
            };
            if target.name != "ph_util_foreach_runtime" {
                continue;
            }
            if let Some(path) = message.executable.filter(|path| !path.is_empty()) {
                return Ok(PathBuf::from(path));
            }
        }
        Err(ReadError::MissingExecutable)
    }
}

#[derive(Deserialize)]
struct CargoTarget {
    name: String,
}

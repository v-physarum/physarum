//! Readers for the JSON a bench consumes.
//!
//! `serde` and `serde_json` are built with default features off. Only `derive`
//! and `std` are enabled. Read failures are `thiserror` values.

use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

/// A JSON document this bench could not read.
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

/// Wall-clock median and standard deviation from one hyperfine result.
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
    /// The path of `artifact` in a cargo JSON log.
    pub(crate) fn executable(log: &str, artifact: &str) -> Result<PathBuf, ReadError> {
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
            if target.name != artifact {
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

#[cfg(test)]
mod tests {
    use super::{CargoMessage, WallClock};

    #[test]
    fn reads_hyperfine_wall_clock() {
        let json = r#"{
            "schema_version": 2,
            "primary_metric": "time_wall_clock",
            "results": [{
                "command": "true",
                "summary": {
                    "time_wall_clock": {
                        "median": 1.5,
                        "stddev": 0.25,
                        "mean": 1.5,
                        "unit": "second"
                    }
                }
            }]
        }"#;
        let clock = WallClock::parse(json).unwrap();
        assert_eq!(clock.median.to_bits(), 1.5_f64.to_bits());
        assert_eq!(clock.stddev.to_bits(), 0.25_f64.to_bits());
    }

    #[test]
    fn empty_hyperfine_results_are_an_error() {
        let Err(error) = WallClock::parse(r#"{"results":[]}"#) else {
            panic!("empty results parsed");
        };
        let error = error.to_string();
        assert!(error.contains("time_wall_clock"), "{error}");
    }

    #[test]
    fn reads_runtime_executable() {
        let log = r#"{"reason":"compiler-message","message":"note"}
{"reason":"compiler-artifact","executable":null,"target":{"name":"ph_util_foreach_runtime"}}
{"reason":"compiler-artifact","executable":"/tmp/runtime","target":{"name":"ph_util_foreach_runtime"}}
"#;
        let path = CargoMessage::executable(log, "ph_util_foreach_runtime").unwrap();
        assert_eq!(path.as_os_str(), "/tmp/runtime");
    }

    #[test]
    fn missing_runtime_executable_is_an_error() {
        let log = r#"{"reason":"compiler-artifact","executable":null,"target":{"name":"other"}}"#;
        let Err(error) = CargoMessage::executable(log, "ph_util_foreach_runtime") else {
            panic!("missing executable parsed");
        };
        let error = error.to_string();
        assert!(error.contains("executable"), "{error}");
    }
}

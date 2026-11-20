//! Readers for the JSON this bench consumes.
//!
//! `serde` and `serde_json` are built with default features off. Only `derive`
//! and `std` are enabled.

use std::path::PathBuf;

use serde::Deserialize;

pub(crate) struct WallClock {
    pub(crate) median: f64,
    pub(crate) stddev: f64,
}

pub(crate) fn wall_clock(json: &str) -> Result<WallClock, String> {
    let report: HyperfineReport =
        serde_json::from_str(json).map_err(|error| format!("hyperfine json: {error}"))?;
    let Some(result) = report.results.first() else {
        return Err("hyperfine json has no time_wall_clock".to_owned());
    };
    let stat = &result.summary.time_wall_clock;
    Ok(WallClock {
        median: stat.median,
        stddev: stat.stddev,
    })
}

pub(crate) fn runtime_executable(log: &str) -> Result<PathBuf, String> {
    for line in log.lines() {
        if line.is_empty() {
            continue;
        }
        let message: CargoMessage =
            serde_json::from_str(line).map_err(|error| format!("cargo json: {error}"))?;
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
    Err("runtime bench executable missing".to_owned())
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

#[derive(Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(default)]
    executable: Option<String>,
    #[serde(default)]
    target: Option<CargoTarget>,
}

#[derive(Deserialize)]
struct CargoTarget {
    name: String,
}

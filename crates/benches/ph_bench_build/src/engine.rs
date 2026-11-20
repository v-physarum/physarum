//! Hyperfine and cargo invocations for one [`crate::Plan`].

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::Plan;
use crate::error::{ErrorKind, Result};
use crate::json::{CargoMessage, WallClock};
use crate::manifest::UserManifest;

const TARGET_DIR: &str = "target";
const USER_DIR: &str = "user";
const LOGS_DIR: &str = "logs";
const SRC_DIR: &str = "src";
const CARGO_TOML: &str = "Cargo.toml";
const LIB_RS: &str = "lib.rs";
const REPORT_JSON: &str = "report.json";
const WARMUP_LOG: &str = "warmup.log";
const RUNTIME_TARGET_DIR: &str = "runtime-target";
const RUNTIME_BUILD_LOG: &str = "runtime_build.log";
const RUNTIME_BUILD_ERR: &str = "runtime_build.err";
const JSON_EXT: &str = ".json";
const HYPERFINE_LOG_EXT: &str = ".hyperfine.log";
const CARGO_LOG_EXT: &str = ".cargo.log";
const RUNTIME_RUN_PREFIX: &str = "runtime.run";
const LOG_EXT: &str = ".log";

struct Layout<'a> {
    root: PathBuf,
    out: PathBuf,
    user: PathBuf,
    logs: PathBuf,
    target: PathBuf,
    plan: &'a Plan,
}

struct Timing {
    median: f64,
    stddev: f64,
}

/// One cargo invocation timed by hyperfine.
#[derive(Clone, Copy)]
struct TimedCargo<'a> {
    label: &'a str,
    prepare: Option<&'a str>,
    command: &'a str,
    incremental: bool,
}

impl<'a> TimedCargo<'a> {
    fn new(label: &'a str, prepare: Option<&'a str>, command: &'a str, incremental: bool) -> Self {
        Self {
            label,
            prepare,
            command,
            incremental,
        }
    }

    fn time(self, layout: &Layout<'_>) -> Result<Timing> {
        let label = self.label;
        let json = layout.logs.join(format!("{label}{JSON_EXT}"));
        let log = layout.logs.join(format!("{label}{HYPERFINE_LOG_EXT}"));
        let cargo_log = layout.logs.join(format!("{label}{CARGO_LOG_EXT}"));
        let repeats = layout.plan.repeats.to_string();
        let mut command = Command::new("hyperfine");
        command.apply_measure_env(&layout.target, self.incremental);
        command.current_dir(&layout.user);
        command.args([
            "--style",
            "basic",
            "--runs",
            &repeats,
            "--metrics",
            "time_wall_clock",
            "--warmup",
            "1",
            "--export-json",
        ]);
        command.arg(&json);
        command.arg("--output").arg(&cargo_log);
        if let Some(prepare) = self.prepare {
            command.arg("--prepare").arg(prepare);
        }
        command.arg(self.command);
        command.run_combined(&log, self.label)?;
        let text = fs::read_to_string(&json)?;
        let clock = WallClock::parse(&text)?;
        Ok(Timing::new(clock.median, clock.stddev))
    }
}

impl Timing {
    fn new(median: f64, stddev: f64) -> Self {
        Self { median, stddev }
    }
}

struct Report {
    path: PathBuf,
    title: String,
    toolchain: String,
    package: String,
    repeats: u32,
    timer: String,
    samples: Vec<Sample>,
    runtime_seconds: Option<f64>,
}

#[derive(serde::Serialize)]
struct Sample {
    case: String,
    median_seconds: f64,
    stddev_seconds: f64,
}

impl Sample {
    fn new(case: &str, median: f64, stddev: f64) -> Self {
        Self {
            case: case.to_owned(),
            median_seconds: median,
            stddev_seconds: stddev,
        }
    }
}

#[derive(serde::Serialize)]
struct ReportDocument<'a> {
    title: &'a str,
    toolchain: &'a str,
    package: &'a str,
    note: &'a str,
    repeats: u32,
    timer: &'a str,
    samples: &'a [Sample],
    runtime_seconds: Option<f64>,
}

impl<'a> ReportDocument<'a> {
    fn new(report: &'a Report, note: &'a str) -> Self {
        Self {
            title: &report.title,
            toolchain: &report.toolchain,
            package: &report.package,
            note,
            repeats: report.repeats,
            timer: &report.timer,
            samples: &report.samples,
            runtime_seconds: report.runtime_seconds,
        }
    }
}

/// Creates the downstream crate and runs the selected scenarios.
pub(crate) fn execute(plan: &Plan) -> Result<()> {
    let layout = Layout::create(plan)?;
    let mut report = Report::create(&layout)?;
    layout.compile_dependency()?;
    report.measure_debug(&layout)?;
    report.measure_incremental(&layout)?;
    report.measure_release(&layout)?;
    report.measure_runtime(&layout)?;
    let text = fs::read_to_string(&report.path)?;
    println!("{text}");
    report.github_summary()
}

impl<'a> Layout<'a> {
    fn create(plan: &'a Plan) -> Result<Self> {
        let root = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .ok_or(ErrorKind::MissingManifestDir)?;
        // `cargo bench` passes `--bench` to a harness-free bench. A path argument
        // selects the output directory; flags are not a directory.
        let out = env::args()
            .skip(1)
            .find(|arg| !arg.starts_with('-'))
            .map_or_else(|| root.join(TARGET_DIR).join(&plan.output), PathBuf::from);
        if out.exists() {
            fs::remove_dir_all(&out)?;
        }
        let user = out.join(USER_DIR);
        let logs = out.join(LOGS_DIR);
        fs::create_dir_all(user.join(SRC_DIR))?;
        fs::create_dir_all(&logs)?;
        let manifest = UserManifest::new(
            &plan.package,
            plan.dependency.name(),
            plan.dependency.directory(),
        )
        .to_toml()?;
        fs::write(user.join(CARGO_TOML), manifest)?;
        let target = out.join(TARGET_DIR);
        Ok(Self {
            root,
            out,
            user,
            logs,
            target,
            plan,
        })
    }

    fn write_case(&self, case: &str) -> Result<()> {
        let source = (self.plan.render)(case)?;
        fs::write(self.user.join(SRC_DIR).join(LIB_RS), source)?;
        Ok(())
    }

    fn compile_dependency(&self) -> Result<()> {
        self.write_case(&self.plan.warmup)?;
        let mut command = Command::cargo(&self.user, &self.target, false);
        command.args(["build", "--offline"]);
        command.run_combined(&self.logs.join(WARMUP_LOG), "warmup")
    }
}

impl Report {
    fn create(layout: &Layout<'_>) -> Result<Self> {
        let report = Self {
            path: layout.out.join(REPORT_JSON),
            title: layout.plan.summary.clone(),
            toolchain: Command::captured("rustc", &["--version"])?,
            package: layout.plan.package.clone(),
            repeats: layout.plan.repeats,
            timer: Command::captured("hyperfine", &["--version"])?,
            samples: Vec::new(),
            runtime_seconds: None,
        };
        report.write()?;
        Ok(report)
    }

    fn note(&self) -> String {
        format!(
            "Proc-macro already built. Hyperfine median of {} runs: full debug, incremental debug, and full release. Release runtime is the bench's own timer.",
            self.repeats
        )
    }

    fn write(&self) -> Result<()> {
        let note = self.note();
        let text = serde_json::to_string_pretty(&ReportDocument::new(self, &note))?;
        fs::write(&self.path, text)?;
        Ok(())
    }

    fn sample(&mut self, name: &str, median: f64, stddev: f64) -> Result<()> {
        self.samples.push(Sample::new(name, median, stddev));
        self.write()
    }

    fn runtime(&mut self, median: f64) -> Result<()> {
        self.runtime_seconds = Some(median);
        self.write()
    }

    fn github_summary(&self) -> Result<()> {
        let Some(path) = env::var_os("GITHUB_STEP_SUMMARY") else {
            return Ok(());
        };
        let mut file = OpenOptions::new().append(true).open(path)?;
        let text = fs::read_to_string(&self.path)?;
        file.write_all(text.as_bytes())?;
        writeln!(file)?;
        Ok(())
    }

    fn measure_debug(&mut self, layout: &Layout<'_>) -> Result<()> {
        for case in &layout.plan.debug {
            layout.write_case(case)?;
            let label = format!("debug_full_{case}");
            let prepare = format!("cargo clean -p {}", layout.plan.package);
            let timing = TimedCargo::new(&label, Some(&prepare), "cargo build --offline", false)
                .time(layout)?;
            println!(
                "debug_full {case} median {:.3}s stddev {:.3}s",
                timing.median, timing.stddev
            );
            self.sample(case, timing.median, timing.stddev)?;
        }
        Ok(())
    }

    fn measure_incremental(&mut self, layout: &Layout<'_>) -> Result<()> {
        let Some(case) = layout.plan.incremental.as_deref() else {
            return Ok(());
        };
        layout.write_case(case)?;
        let label = format!("debug_incremental_{case}");
        let timing = TimedCargo::new(&label, None, "cargo build --offline", true).time(layout)?;
        println!(
            "debug_incremental {case} median {:.3}s stddev {:.3}s",
            timing.median, timing.stddev
        );
        self.sample(&label, timing.median, timing.stddev)
    }

    fn measure_release(&mut self, layout: &Layout<'_>) -> Result<()> {
        for case in &layout.plan.release {
            layout.write_case(case)?;
            let label = format!("release_full_{case}");
            let prepare = format!("cargo clean -p {} --release", layout.plan.package);
            let timing = TimedCargo::new(
                &label,
                Some(&prepare),
                "cargo build --release --offline",
                false,
            )
            .time(layout)?;
            println!(
                "release_full {case} median {:.3}s stddev {:.3}s",
                timing.median, timing.stddev
            );
            self.sample(&label, timing.median, timing.stddev)?;
        }
        Ok(())
    }

    fn measure_runtime(&mut self, layout: &Layout<'_>) -> Result<()> {
        let Some(bench) = layout.plan.runtime.as_deref() else {
            return Ok(());
        };
        let runtime_target = layout.out.join(RUNTIME_TARGET_DIR);
        let mut command = Command::cargo(&layout.root, &runtime_target, false);
        command.args([
            "build",
            "--release",
            "--bench",
            bench,
            "--offline",
            "--message-format=json",
        ]);
        let output = command.output().map_err(|source| ErrorKind::CommandStart {
            label: "release runtime bench".to_owned(),
            source,
        })?;
        fs::write(layout.logs.join(RUNTIME_BUILD_LOG), &output.stdout)?;
        fs::write(layout.logs.join(RUNTIME_BUILD_ERR), &output.stderr)?;
        if !output.status.success() {
            return Err(ErrorKind::RuntimeBuild {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
            .into());
        }
        let log = String::from_utf8(output.stdout).map_err(|source| ErrorKind::Utf8 {
            what: "runtime build log",
            source,
        })?;
        let executable = CargoMessage::executable(&log, bench)?;
        let mut samples = Vec::new();
        for run in 1..=layout.plan.repeats {
            let output =
                Command::new(&executable)
                    .output()
                    .map_err(|source| ErrorKind::CommandStart {
                        label: "runtime bench".to_owned(),
                        source,
                    })?;
            let stdout = String::from_utf8(output.stdout).map_err(|source| ErrorKind::Utf8 {
                what: "runtime output",
                source,
            })?;
            fs::write(
                layout
                    .logs
                    .join(format!("{RUNTIME_RUN_PREFIX}{run}{LOG_EXT}")),
                &stdout,
            )?;
            if !output.status.success() {
                return Err(ErrorKind::RuntimeRun { run }.into());
            }
            samples.push(stdout.runtime_seconds()?);
        }
        samples.sort_by(f64::total_cmp);
        let median = samples.median();
        println!("runtime_release {median:.6}s");
        self.runtime(median)
    }
}

trait MeasuredCommand {
    fn cargo(cwd: &Path, target_dir: &Path, incremental: bool) -> Self;
    fn apply_measure_env(&mut self, target_dir: &Path, incremental: bool);
    fn run_combined(&mut self, log: &Path, label: &str) -> Result<()>;
    fn captured(program: &'static str, args: &[&str]) -> Result<String>;
}

impl MeasuredCommand for Command {
    fn cargo(cwd: &Path, target_dir: &Path, incremental: bool) -> Self {
        let mut command = Self::new("cargo");
        command.current_dir(cwd);
        command.apply_measure_env(target_dir, incremental);
        command
    }

    fn apply_measure_env(&mut self, target_dir: &Path, incremental: bool) {
        self.env_remove("CARGO_PROFILE_DEV_DEBUG")
            .env("CARGO_INCREMENTAL", if incremental { "1" } else { "0" })
            .env("CARGO_TERM_COLOR", "never")
            .env("CARGO_TARGET_DIR", target_dir);
    }

    fn run_combined(&mut self, log: &Path, label: &str) -> Result<()> {
        let output = self.output().map_err(|source| ErrorKind::CommandStart {
            label: label.to_owned(),
            source,
        })?;
        let mut text = output.stdout;
        text.extend_from_slice(&output.stderr);
        fs::write(log, &text)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(ErrorKind::CommandFailed {
                label: label.to_owned(),
                output: String::from_utf8_lossy(&text).into_owned(),
            }
            .into())
        }
    }

    fn captured(program: &'static str, args: &[&str]) -> Result<String> {
        let output = Self::new(program)
            .args(args)
            .output()
            .map_err(|_| ErrorKind::ProgramMissing { program })?;
        output.stdout_text(program)
    }
}

trait CapturedOutput {
    fn stdout_text(&self, program: &'static str) -> Result<String>;
}

impl CapturedOutput for Output {
    fn stdout_text(&self, program: &'static str) -> Result<String> {
        if !self.status.success() {
            return Err(ErrorKind::ProgramFailed { program }.into());
        }
        let text = String::from_utf8(self.stdout.clone())
            .map_err(|source| ErrorKind::ProgramUtf8 { program, source })?;
        Ok(text.trim().to_owned())
    }
}

trait RuntimeStdout {
    fn runtime_seconds(&self) -> Result<f64>;
}

impl RuntimeStdout for str {
    fn runtime_seconds(&self) -> Result<f64> {
        for line in self.lines() {
            if let Some(rest) = line.strip_prefix("runtime_seconds ") {
                let seconds: f64 = rest.trim().parse().map_err(|_| ErrorKind::RuntimeSeconds)?;
                return Ok(seconds);
            }
        }
        Err(ErrorKind::MissingRuntimeSeconds.into())
    }
}

trait SampleMedian {
    fn median(&self) -> f64;
}

impl SampleMedian for [f64] {
    fn median(&self) -> f64 {
        let mid = self.len() / 2;
        if self.len().is_multiple_of(2) {
            f64::midpoint(self[mid - 1], self[mid])
        } else {
            self[mid]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sample;

    #[test]
    fn sample_is_json() {
        let text = serde_json::to_string(&Sample::new("flat_structs_64", 0.032, 0.002)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["case"], "flat_structs_64");
        assert_eq!(value["median_seconds"], 0.032);
        assert_eq!(value["stddev_seconds"], 0.002);
    }
}

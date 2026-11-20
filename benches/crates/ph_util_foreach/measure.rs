//! Times `impl_foreach!` in a downstream crate with hyperfine 2.
//!
//! The user-crate sources are the files in `benches/crates/ph_util_foreach/cases`.
//!
//! The proc-macro is compiled once and is not part of the case times.
//! Debug and release builds are full compiles. Incremental time is debug only.
//! Runtime time is the release bench.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

use thiserror::Error;

mod json;

const REPEATS: u32 = 3;
const DEBUG_CASES: &[&str] = &[
    "flat_structs_64",
    "flat_structs_1024",
    "flat_structs_8192",
    "hand_structs_64",
    "hand_structs_1024",
    "hand_structs_8192",
    "one_call_256",
    "many_calls_256",
    "fan_take_16",
    "fan_take_64",
    "fan_rows_64",
    "fan_rows_256",
];
const RELEASE_CASES: &[&str] = &[
    "flat_structs_1024",
    "hand_structs_1024",
    "flat_structs_8192",
    "fan_take_64",
    "fan_rows_256",
];

struct Layout {
    root: PathBuf,
    out: PathBuf,
    user: PathBuf,
    logs: PathBuf,
    target: PathBuf,
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
        Self { label, prepare, command, incremental }
    }

    fn time(self, layout: &Layout) -> Result<Timing> {
        let json = layout.logs.join(format!("{}.json", self.label));
        let log = layout.logs.join(format!("{}.hyperfine.log", self.label));
        let cargo_log = layout.logs.join(format!("{}.cargo.log", self.label));
        let repeats = REPEATS.to_string();
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
        let clock = json::WallClock::parse(&text)?;
        Ok(Timing::new(clock.median, clock.stddev))
    }
}

impl Timing {
    fn new(median: f64, stddev: f64) -> Self {
        Self { median, stddev }
    }
}

struct Report {
    file: File,
    rows: Vec<String>,
}

#[derive(Debug, Error)]
enum Failure {
    #[error(transparent)]
    Read(#[from] json::ReadError),
    #[error("{0}")]
    Message(String),
}

impl Failure {
    fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

type Result<T> = std::result::Result<T, Failure>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<()> {
    let layout = Layout::create()?;
    let mut report = Report::create(&layout)?;
    layout.compile_proc_macro()?;
    report.measure_debug(&layout)?;
    report.measure_incremental(&layout)?;
    report.measure_release(&layout)?;
    report.measure_runtime(&layout)?;
    let text = fs::read_to_string(layout.out.join("report.txt"))?;
    println!("{text}");
    report.github_summary()
}

impl Layout {
    fn create() -> Result<Self> {
        let root = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| {
                Failure::message(
                    "CARGO_MANIFEST_DIR is unset; run cargo bench --bench ph_util_foreach_measure",
                )
            })?;
        // `cargo bench` passes `--bench` to a harness-free bench. A path argument
        // selects the output directory; flags are not a directory.
        let out = env::args()
            .skip(1)
            .find(|arg| !arg.starts_with('-'))
            .map_or_else(|| root.join("target/ph-util-foreach-expand"), PathBuf::from);
        if out.exists() {
            fs::remove_dir_all(&out)?;
        }
        let user = out.join("user");
        let logs = out.join("logs");
        fs::create_dir_all(user.join("src"))?;
        fs::create_dir_all(&logs)?;
        let macro_dir = root.join("crates/util/ph_util_foreach");
        fs::write(user.join("Cargo.toml"), Self::user_manifest(&macro_dir))?;
        let target = out.join("target");
        Ok(Self { root, out, user, logs, target })
    }

    fn write_case(&self, case: &str) -> Result<()> {
        fs::write(self.user.join("src/lib.rs"), self.case_source(case)?)?;
        Ok(())
    }
}

impl Report {
    fn create(layout: &Layout) -> Result<Self> {
        let toolchain = Command::captured("rustc", &["--version"])?;
        let timer = Command::captured("hyperfine", &["--version"])?;
        let mut file = File::create(layout.out.join("report.txt"))?;
        writeln!(file, "toolchain {toolchain}")?;
        writeln!(file, "package ph_util_foreach_user")?;
        writeln!(file, "proc_macro compiled once before these cases")?;
        writeln!(file, "repeats {REPEATS}")?;
        writeln!(file, "timer {timer}")?;
        writeln!(file, "section=debug_full incremental_off")?;
        writeln!(file, "columns case median_seconds stddev_seconds")?;
        Ok(Self { file, rows: Vec::new() })
    }

    fn sample(&mut self, name: &str, median: f64, stddev: f64) -> Result<()> {
        let median = format!("{median:.3}");
        let stddev = format!("{stddev:.3}");
        writeln!(self.file, "{name} {median} {stddev}")?;
        self.file.flush()?;
        self.rows
            .push(format!("| `{name}` | {median} | {stddev} |"));
        Ok(())
    }

    fn runtime(&mut self, median: f64) -> Result<()> {
        writeln!(self.file, "runtime_release {median:.6}")?;
        self.file.flush()?;
        Ok(())
    }

    fn github_summary(&self) -> Result<()> {
        let Some(path) = env::var_os("GITHUB_STEP_SUMMARY") else {
            return Ok(());
        };
        let mut file = OpenOptions::new().append(true).open(path)?;
        writeln!(file, "## ph_util_foreach user compile time")?;
        writeln!(file)?;
        writeln!(
            file,
            "Proc-macro already built. Hyperfine median of {REPEATS} runs: full debug, incremental debug, and full release. Release runtime is the bench's own timer."
        )?;
        writeln!(file)?;
        writeln!(file, "| case | median (s) | stddev (s) |")?;
        writeln!(file, "| --- | ---: | ---: |")?;
        for row in &self.rows {
            writeln!(file, "{row}")?;
        }
        Ok(())
    }
}

impl Layout {
    fn compile_proc_macro(&self) -> Result<()> {
        self.write_case("flat_structs_1")?;
        let mut command = Command::cargo(&self.user, &self.target, false);
        command.args(["build", "--offline"]);
        command.run_combined(&self.logs.join("warmup.log"), "warmup")
    }

    fn user_manifest(macro_dir: &Path) -> String {
        format!(
            "[workspace]\n\n[package]\nname = \"ph_util_foreach_user\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\nph_util_foreach = {{ path = \"{}\" }}\n",
            macro_dir.display()
        )
    }

    fn case_source(&self, case: &str) -> Result<String> {
        let (kind, count) = case
            .rsplit_once('_')
            .ok_or_else(|| Failure::message(format!("unknown case {case}")))?;
        let count: u32 = count
            .parse()
            .map_err(|_| Failure::message(format!("unknown case {case}")))?;
        let cases = self.root.join("benches/crates/ph_util_foreach/cases");
        let len = count.to_string();
        match kind {
            "flat_structs" | "one_call" => {
                Self::fill(&cases.join("flat_structs.rs"), &[("CASE_LEN", &len)])
            }
            "hand_structs" => Self::repeat_item(
                &cases.join("hand_structs.rs"),
                &cases.join("hand_struct.rs"),
                count,
                None,
            ),
            "many_calls" => Self::repeat_item(
                &cases.join("many_calls.rs"),
                &cases.join("many_call.rs"),
                count,
                Some("NEXT"),
            ),
            "fan_take" => Self::fill(&cases.join("fan_take.rs"), &[("CASE_LEN", &len)]),
            "fan_rows" => Self::fill(&cases.join("fan_rows.rs"), &[("CASE_LEN", &len)]),
            _ => Err(Failure::message(format!("unknown case {case}"))),
        }
    }

    fn fill(path: &Path, pairs: &[(&str, &str)]) -> Result<String> {
        let mut text = fs::read_to_string(path)?;
        for (key, value) in pairs {
            text = text.replace(key, value);
        }
        Ok(text)
    }

    fn repeat_item(
        header: &Path,
        item: &Path,
        count: u32,
        next_key: Option<&str>,
    ) -> Result<String> {
        let mut source = fs::read_to_string(header)?;
        if !source.ends_with('\n') {
            source.push('\n');
        }
        if !source.ends_with("\n\n") {
            source.push('\n');
        }
        let item = fs::read_to_string(item)?;
        for index in 0..count {
            let mut line = item.replace("INDEX", &index.to_string());
            if let Some(next_key) = next_key {
                line = line.replace(next_key, &(index + 1).to_string());
            }
            source.push_str(&line);
        }
        Ok(source)
    }
}

impl Report {
    fn measure_debug(&mut self, layout: &Layout) -> Result<()> {
        for case in DEBUG_CASES {
            layout.write_case(case)?;
            let label = format!("debug_full_{case}");
            let timing = TimedCargo::new(
                &label,
                Some("cargo clean -p ph_util_foreach_user"),
                "cargo build --offline",
                false,
            )
            .time(layout)?;
            println!(
                "debug_full {case} median {:.3}s stddev {:.3}s",
                timing.median, timing.stddev
            );
            self.sample(case, timing.median, timing.stddev)?;
        }
        Ok(())
    }

    fn measure_incremental(&mut self, layout: &Layout) -> Result<()> {
        layout.write_case("flat_structs_1024")?;
        let timing = TimedCargo::new(
            "debug_incremental_flat_structs_1024",
            None,
            "cargo build --offline",
            true,
        )
        .time(layout)?;
        println!(
            "debug_incremental flat_structs_1024 median {:.3}s stddev {:.3}s",
            timing.median, timing.stddev
        );
        self.sample(
            "debug_incremental_flat_structs_1024",
            timing.median,
            timing.stddev,
        )
    }

    fn measure_release(&mut self, layout: &Layout) -> Result<()> {
        for case in RELEASE_CASES {
            layout.write_case(case)?;
            let label = format!("release_full_{case}");
            let timing = TimedCargo::new(
                &label,
                Some("cargo clean -p ph_util_foreach_user --release"),
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

    fn measure_runtime(&mut self, layout: &Layout) -> Result<()> {
        let runtime_target = layout.out.join("runtime-target");
        let mut command = Command::cargo(&layout.root, &runtime_target, false);
        command.args([
            "build",
            "--release",
            "--bench",
            "ph_util_foreach_runtime",
            "--offline",
            "--message-format=json",
        ]);
        let output = command.output().map_err(|error| {
            Failure::message(format!("release runtime bench failed to start: {error}"))
        })?;
        fs::write(layout.logs.join("runtime_build.log"), &output.stdout)?;
        fs::write(layout.logs.join("runtime_build.err"), &output.stderr)?;
        if !output.status.success() {
            return Err(Failure::message(format!(
                "release runtime bench failed to build\n{}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let log = String::from_utf8(output.stdout).map_err(|error| {
            Failure::message(format!("runtime build log is not utf-8: {error}"))
        })?;
        let executable = json::CargoMessage::runtime_executable(&log)?;
        let mut samples = Vec::new();
        for run in 1..=REPEATS {
            let output = Command::new(&executable).output().map_err(|error| {
                Failure::message(format!("runtime bench failed to start: {error}"))
            })?;
            let stdout = String::from_utf8(output.stdout).map_err(|error| {
                Failure::message(format!("runtime output is not utf-8: {error}"))
            })?;
            fs::write(layout.logs.join(format!("runtime.run{run}.log")), &stdout)?;
            if !output.status.success() {
                return Err(Failure::message(format!("runtime bench run {run} failed")));
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
    fn captured(program: &str, args: &[&str]) -> Result<String>;
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
        let output = self
            .output()
            .map_err(|error| Failure::message(format!("{label} failed to start: {error}")))?;
        let mut text = output.stdout;
        text.extend_from_slice(&output.stderr);
        fs::write(log, &text)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(Failure::message(format!(
                "{label} failed\n{}",
                String::from_utf8_lossy(&text)
            )))
        }
    }

    fn captured(program: &str, args: &[&str]) -> Result<String> {
        let output = Self::new(program)
            .args(args)
            .output()
            .map_err(|_| Failure::message(format!("{program} is required")))?;
        output.stdout_text(program)
    }
}

trait CapturedOutput {
    fn stdout_text(&self, program: &str) -> Result<String>;
}

impl CapturedOutput for Output {
    fn stdout_text(&self, program: &str) -> Result<String> {
        if !self.status.success() {
            return Err(Failure::message(format!("{program} failed")));
        }
        let text = String::from_utf8(self.stdout.clone())
            .map_err(|error| Failure::message(format!("{program} output is not utf-8: {error}")))?;
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
                return rest
                    .trim()
                    .parse()
                    .map_err(|_| Failure::message("runtime_seconds is not a number"));
            }
        }
        Err(Failure::message(
            "runtime bench did not print runtime_seconds",
        ))
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

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Failure::message(error.to_string())
    }
}

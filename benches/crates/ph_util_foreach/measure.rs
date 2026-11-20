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

fn fail(message: impl Into<String>) -> Failure {
    Failure::Message(message.into())
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
    compile_proc_macro(&layout)?;
    measure_debug(&layout, &mut report)?;
    measure_incremental(&layout, &mut report)?;
    measure_release(&layout, &mut report)?;
    measure_runtime(&layout, &mut report)?;
    let text = fs::read_to_string(layout.out.join("report.txt"))?;
    println!("{text}");
    report.github_summary()
}

impl Layout {
    fn create() -> Result<Self> {
        let root = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| {
                fail("CARGO_MANIFEST_DIR is unset; run cargo bench --bench ph_util_foreach_measure")
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
        fs::write(user.join("Cargo.toml"), user_manifest(&macro_dir))?;
        let target = out.join("target");
        Ok(Self {
            root,
            out,
            user,
            logs,
            target,
        })
    }

    fn write_case(&self, case: &str) -> Result<()> {
        fs::write(self.user.join("src/lib.rs"), case_source(&self.root, case)?)?;
        Ok(())
    }
}

impl Report {
    fn create(layout: &Layout) -> Result<Self> {
        let toolchain = command_stdout("rustc", &["--version"])?;
        let timer = command_stdout("hyperfine", &["--version"])?;
        let mut file = File::create(layout.out.join("report.txt"))?;
        writeln!(file, "toolchain {toolchain}")?;
        writeln!(file, "package ph_util_foreach_user")?;
        writeln!(file, "proc_macro compiled once before these cases")?;
        writeln!(file, "repeats {REPEATS}")?;
        writeln!(file, "timer {timer}")?;
        writeln!(file, "section=debug_full incremental_off")?;
        writeln!(file, "columns case median_seconds stddev_seconds")?;
        Ok(Self {
            file,
            rows: Vec::new(),
        })
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

fn compile_proc_macro(layout: &Layout) -> Result<()> {
    layout.write_case("flat_structs_1")?;
    let mut command = cargo(&layout.user, &layout.target, false);
    command.args(["build", "--offline"]);
    run_combined(&mut command, &layout.logs.join("warmup.log"), "warmup")
}

fn measure_debug(layout: &Layout, report: &mut Report) -> Result<()> {
    for case in DEBUG_CASES {
        layout.write_case(case)?;
        let timing = time_build(
            layout,
            &format!("debug_full_{case}"),
            Some("cargo clean -p ph_util_foreach_user"),
            "cargo build --offline",
            false,
        )?;
        println!(
            "debug_full {case} median {:.3}s stddev {:.3}s",
            timing.median, timing.stddev
        );
        report.sample(case, timing.median, timing.stddev)?;
    }
    Ok(())
}

fn measure_incremental(layout: &Layout, report: &mut Report) -> Result<()> {
    layout.write_case("flat_structs_1024")?;
    let timing = time_build(
        layout,
        "debug_incremental_flat_structs_1024",
        None,
        "cargo build --offline",
        true,
    )?;
    println!(
        "debug_incremental flat_structs_1024 median {:.3}s stddev {:.3}s",
        timing.median, timing.stddev
    );
    report.sample(
        "debug_incremental_flat_structs_1024",
        timing.median,
        timing.stddev,
    )
}

fn measure_release(layout: &Layout, report: &mut Report) -> Result<()> {
    for case in RELEASE_CASES {
        layout.write_case(case)?;
        let label = format!("release_full_{case}");
        let timing = time_build(
            layout,
            &label,
            Some("cargo clean -p ph_util_foreach_user --release"),
            "cargo build --release --offline",
            false,
        )?;
        println!(
            "release_full {case} median {:.3}s stddev {:.3}s",
            timing.median, timing.stddev
        );
        report.sample(&label, timing.median, timing.stddev)?;
    }
    Ok(())
}

fn measure_runtime(layout: &Layout, report: &mut Report) -> Result<()> {
    let runtime_target = layout.out.join("runtime-target");
    let mut command = cargo(&layout.root, &runtime_target, false);
    command.args([
        "build",
        "--release",
        "--bench",
        "ph_util_foreach_runtime",
        "--offline",
        "--message-format=json",
    ]);
    let output = command
        .output()
        .map_err(|error| fail(format!("release runtime bench failed to start: {error}")))?;
    fs::write(layout.logs.join("runtime_build.log"), &output.stdout)?;
    fs::write(layout.logs.join("runtime_build.err"), &output.stderr)?;
    if !output.status.success() {
        return Err(fail(format!(
            "release runtime bench failed to build\n{}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let log = String::from_utf8(output.stdout)
        .map_err(|error| fail(format!("runtime build log is not utf-8: {error}")))?;
    let executable = json::runtime_executable(&log)?;
    let mut samples = Vec::new();
    for run in 1..=REPEATS {
        let output = Command::new(&executable)
            .output()
            .map_err(|error| fail(format!("runtime bench failed to start: {error}")))?;
        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| fail(format!("runtime output is not utf-8: {error}")))?;
        fs::write(layout.logs.join(format!("runtime.run{run}.log")), &stdout)?;
        if !output.status.success() {
            return Err(fail(format!("runtime bench run {run} failed")));
        }
        samples.push(runtime_seconds(&stdout)?);
    }
    samples.sort_by(f64::total_cmp);
    let median = median(&samples);
    println!("runtime_release {median:.6}s");
    report.runtime(median)
}

fn time_build(
    layout: &Layout,
    label: &str,
    prepare: Option<&str>,
    cargo_command: &str,
    incremental: bool,
) -> Result<Timing> {
    let json = layout.logs.join(format!("{label}.json"));
    let log = layout.logs.join(format!("{label}.hyperfine.log"));
    let cargo_log = layout.logs.join(format!("{label}.cargo.log"));
    let repeats = REPEATS.to_string();
    let mut command = Command::new("hyperfine");
    apply_measure_env(&mut command, &layout.target, incremental);
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
    if let Some(prepare) = prepare {
        command.arg("--prepare").arg(prepare);
    }
    command.arg(cargo_command);
    run_combined(&mut command, &log, label)?;
    let text = fs::read_to_string(&json)?;
    let clock = json::wall_clock(&text)?;
    Ok(Timing::new(clock.median, clock.stddev))
}

fn cargo(cwd: &Path, target_dir: &Path, incremental: bool) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(cwd);
    apply_measure_env(&mut command, target_dir, incremental);
    command
}

fn apply_measure_env(command: &mut Command, target_dir: &Path, incremental: bool) {
    command
        .env_remove("CARGO_PROFILE_DEV_DEBUG")
        .env("CARGO_INCREMENTAL", if incremental { "1" } else { "0" })
        .env("CARGO_TERM_COLOR", "never")
        .env("CARGO_TARGET_DIR", target_dir);
}

fn run_combined(command: &mut Command, log: &Path, label: &str) -> Result<()> {
    let output = command
        .output()
        .map_err(|error| fail(format!("{label} failed to start: {error}")))?;
    let mut text = output.stdout;
    text.extend_from_slice(&output.stderr);
    fs::write(log, &text)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail(format!(
            "{label} failed\n{}",
            String::from_utf8_lossy(&text)
        )))
    }
}

fn command_stdout(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|_| fail(format!("{program} is required")))?;
    stdout_text(program, &output)
}

fn stdout_text(program: &str, output: &Output) -> Result<String> {
    if !output.status.success() {
        return Err(fail(format!("{program} failed")));
    }
    let text = String::from_utf8(output.stdout.clone())
        .map_err(|error| fail(format!("{program} output is not utf-8: {error}")))?;
    Ok(text.trim().to_owned())
}

fn runtime_seconds(stdout: &str) -> Result<f64> {
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("runtime_seconds ") {
            return rest
                .trim()
                .parse()
                .map_err(|_| fail("runtime_seconds is not a number"));
        }
    }
    Err(fail("runtime bench did not print runtime_seconds"))
}

fn median(sorted: &[f64]) -> f64 {
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        f64::midpoint(sorted[mid - 1], sorted[mid])
    } else {
        sorted[mid]
    }
}

fn user_manifest(macro_dir: &Path) -> String {
    format!(
        "[workspace]\n\n[package]\nname = \"ph_util_foreach_user\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\nph_util_foreach = {{ path = \"{}\" }}\n",
        macro_dir.display()
    )
}

fn case_source(root: &Path, case: &str) -> Result<String> {
    let (kind, count) = case
        .rsplit_once('_')
        .ok_or_else(|| fail(format!("unknown case {case}")))?;
    let count: u32 = count
        .parse()
        .map_err(|_| fail(format!("unknown case {case}")))?;
    let cases = root.join("benches/crates/ph_util_foreach/cases");
    let len = count.to_string();
    match kind {
        "flat_structs" | "one_call" => fill(&cases.join("flat_structs.rs"), &[("CASE_LEN", &len)]),
        "hand_structs" => repeat_item(
            &cases.join("hand_structs.rs"),
            &cases.join("hand_struct.rs"),
            count,
            None,
        ),
        "many_calls" => repeat_item(
            &cases.join("many_calls.rs"),
            &cases.join("many_call.rs"),
            count,
            Some("NEXT"),
        ),
        "fan_take" => fill(&cases.join("fan_take.rs"), &[("CASE_LEN", &len)]),
        "fan_rows" => fill(&cases.join("fan_rows.rs"), &[("CASE_LEN", &len)]),
        _ => Err(fail(format!("unknown case {case}"))),
    }
}

fn fill(path: &Path, pairs: &[(&str, &str)]) -> Result<String> {
    let mut text = fs::read_to_string(path)?;
    for (key, value) in pairs {
        text = text.replace(key, value);
    }
    Ok(text)
}

fn repeat_item(header: &Path, item: &Path, count: u32, next_key: Option<&str>) -> Result<String> {
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

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        fail(error.to_string())
    }
}

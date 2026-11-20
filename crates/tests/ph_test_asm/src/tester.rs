//! [`AsmTester`] checks one release probe.

use std::path::{Path, PathBuf};

use crate::emit;
use crate::scan;

/// Release assembly checks for one probe file.
///
/// ```no_run
/// use ph_test_asm::AsmTester;
///
/// AsmTester::new("probe.rs").function("add").hot_loop(|hot| {
///     hot.assert_simd();
///     hot.assert_no_bounds();
///     hot.assert_no_branch();
///     hot.assert_no_virtual_calls();
///     hot.on_target("x86_64-unknown-linux-gnu").assert_max_instructions(64);
/// });
/// ```
#[must_use]
#[derive(Debug)]
pub struct AsmTester {
    source: PathBuf,
}

impl AsmTester {
    /// `source` is a Rust probe. It is compiled as release on the first assert.
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
        }
    }

    /// Selects the symbol named `name`.
    pub fn function(self, name: impl Into<String>) -> FunctionScope {
        FunctionScope {
            source: self.source,
            name: name.into(),
        }
    }
}

/// One function inside an [`AsmTester`] probe.
#[must_use]
#[derive(Debug)]
pub struct FunctionScope {
    source: PathBuf,
    name: String,
}

impl FunctionScope {
    /// Runs `check` on the hot loop.
    ///
    /// The hot loop is the backedge region with the most instructions.
    /// A loop may contain one conditional branch, its latch.
    ///
    /// # Panics
    ///
    /// Panics when the probe fails to compile, the function is missing, or there is no loop.
    pub fn hot_loop(self, check: impl FnOnce(&Asserts)) {
        check(&load(&self.source, &self.name, ScopeKind::HotLoop, None));
    }

    /// Runs `check` on the whole function. The function may contain no conditional branch.
    ///
    /// # Panics
    ///
    /// Panics when the probe fails to compile or the function is missing.
    pub fn on_function(self, check: impl FnOnce(&Asserts)) {
        check(&load(&self.source, &self.name, ScopeKind::Function, None));
    }
}

/// Asserts for a function or its hot loop.
#[derive(Debug)]
pub struct Asserts {
    label: String,
    asm: String,
    scope_start: usize,
    scope_end: usize,
    signature: String,
    branch_allowance: u32,
    source: PathBuf,
    function: String,
    kind: ScopeKind,
}

impl Asserts {
    /// The scope contains a vector register.
    ///
    /// # Panics
    ///
    /// Panics when the scope is scalar.
    pub fn assert_simd(&self) {
        self.enforce(Rule::Simd);
    }

    /// The scope has no bounds-check panic.
    ///
    /// # Panics
    ///
    /// Panics when a bounds check is present.
    pub fn assert_no_bounds(&self) {
        self.enforce(Rule::NoBounds);
    }

    /// The scope has no extra conditional branch.
    ///
    /// A hot loop may keep its latch branch. A function may keep none.
    ///
    /// # Panics
    ///
    /// Panics when there are more conditional branches than the scope allows.
    pub fn assert_no_branch(&self) {
        self.enforce(Rule::NoBranch);
    }

    /// The scope has no indirect call.
    ///
    /// # Panics
    ///
    /// Panics when a callee is a register.
    pub fn assert_no_virtual_calls(&self) {
        self.enforce(Rule::NoVirtualCalls);
    }

    /// The scope has no conditional move.
    ///
    /// # Panics
    ///
    /// Panics when a conditional move is present.
    pub fn assert_no_select(&self) {
        self.enforce(Rule::NoSelect);
    }

    /// The scope has no integer or float division.
    ///
    /// # Panics
    ///
    /// Panics when a division instruction is present.
    pub fn assert_no_div(&self) {
        self.enforce(Rule::NoDiv);
    }

    /// The scope has no unwind resume.
    ///
    /// # Panics
    ///
    /// Panics when an unwind resume is present.
    pub fn assert_no_invoke(&self) {
        self.enforce(Rule::NoInvoke);
    }

    /// The scope does not call the Rust allocator.
    ///
    /// # Panics
    ///
    /// Panics when `__rust_alloc` or a related allocator symbol is present.
    pub fn assert_no_alloc(&self) {
        self.enforce(Rule::NoAlloc);
    }

    /// The scope has no atomic operation.
    ///
    /// # Panics
    ///
    /// Panics when an atomic instruction is present.
    pub fn assert_no_atomic(&self) {
        self.enforce(Rule::NoAtomic);
    }

    /// The scope has no `memcpy`, `memmove`, or `memset`.
    ///
    /// # Panics
    ///
    /// Panics when one of those symbols is present.
    pub fn assert_no_memcpy(&self) {
        self.enforce(Rule::NoMemcpy);
    }

    /// The scope has no panic or assert.
    ///
    /// # Panics
    ///
    /// Panics when a check is present.
    pub fn assert_no_checks(&self) {
        self.enforce(Rule::NoChecks);
    }

    /// Load instructions are at most `max`.
    ///
    /// # Panics
    ///
    /// Panics when the scope has more loads.
    pub fn assert_max_loads(&self, max: u32) {
        self.enforce(Rule::MaxLoads(max));
    }

    /// Store instructions are at most `max`.
    ///
    /// # Panics
    ///
    /// Panics when the scope has more stores.
    pub fn assert_max_stores(&self, max: u32) {
        self.enforce(Rule::MaxStores(max));
    }

    /// The scope is a counted loop: a backedge, an add or increment, and a compare.
    ///
    /// # Panics
    ///
    /// Panics when that pattern is missing.
    pub fn assert_counted_loop(&self) {
        self.enforce(Rule::CountedLoop);
    }

    /// The scope does not address the stack pointer.
    ///
    /// # Panics
    ///
    /// Panics when the scope names the stack pointer.
    pub fn assert_no_alloca_in_loop(&self) {
        self.enforce(Rule::NoAllocaInLoop);
    }

    /// Every pointer parameter is `noalias` in the LLVM IR signature.
    ///
    /// A signature with no pointer parameter passes.
    ///
    /// # Panics
    ///
    /// Panics when a pointer parameter lacks `noalias`.
    pub fn assert_noalias(&self) {
        self.enforce(Rule::NoAlias);
    }

    /// The scope has no gather instruction or gather callee.
    ///
    /// # Panics
    ///
    /// Panics when a callee name contains `gather`.
    pub fn assert_no_gather(&self) {
        self.enforce(Rule::NoGather);
    }

    /// The scope has no math library call.
    ///
    /// # Panics
    ///
    /// Panics when a callee is `sin` or another listed math symbol.
    pub fn assert_no_libm(&self) {
        self.enforce(Rule::NoLibm);
    }

    /// The scope has no fused multiply-add.
    ///
    /// # Panics
    ///
    /// Panics when a callee is `fma` or a fused multiply-add instruction.
    pub fn assert_no_fma(&self) {
        self.enforce(Rule::NoFma);
    }

    /// Recompiles the probe for `triple` and counts assembly instructions there.
    #[must_use]
    pub fn on_target<'a>(&'a self, triple: &'a str) -> TargetAssert<'a> {
        TargetAssert {
            asserts: self,
            triple,
        }
    }

    /// Runs every rule in `rules`.
    pub fn apply(&self, rules: &Rules) {
        rules.apply(self);
    }

    fn scope_text(&self) -> &str {
        &self.asm[self.scope_start..self.scope_end]
    }

    fn enforce(&self, rule: Rule) {
        let asm = self.scope_text();
        match rule {
            Rule::Simd => self.require(scan::has_simd(asm), "simd", "no vector register"),
            Rule::NoBounds => {
                self.forbid(
                    scan::has_bounds(asm),
                    "bounds check",
                    "bounds check is present",
                );
            }
            Rule::NoBranch => {
                let count = scan::conditional_branches(asm);
                if count > widen(self.branch_allowance) {
                    self.fail(
                        "conditional branch",
                        &format!(
                            "{count} conditional branches, allowance is {}",
                            self.branch_allowance
                        ),
                    );
                }
            }
            Rule::NoVirtualCalls => {
                self.forbid(
                    scan::has_indirect_call(asm),
                    "virtual call",
                    "indirect call",
                );
            }
            Rule::NoSelect => {
                self.forbid(
                    scan::has_conditional_move(asm),
                    "select",
                    "conditional move",
                );
            }
            Rule::NoDiv => self.forbid(scan::has_div(asm), "division", "division"),
            Rule::NoInvoke => self.forbid(scan::has_unwind(asm), "invoke", "unwind resume"),
            Rule::NoAlloc => self.forbid(scan::has_alloc(asm), "allocation", "allocator call"),
            Rule::NoAtomic => self.forbid(scan::has_atomic(asm), "atomic", "atomic operation"),
            Rule::NoMemcpy => self.forbid(
                scan::has_memcpy(asm),
                "memcpy",
                "memcpy, memmove, or memset",
            ),
            Rule::NoChecks => self.forbid(scan::has_panic(asm), "panic check", "panic or assert"),
            Rule::MaxLoads(max) => self.at_most(scan::count_loads(asm), max, "loads"),
            Rule::MaxStores(max) => self.at_most(scan::count_stores(asm), max, "stores"),
            Rule::CountedLoop => {
                self.require(
                    scan::is_counted_loop(asm),
                    "counted loop",
                    "no counted backedge",
                );
            }
            Rule::NoAllocaInLoop => self.forbid(
                scan::has_stack_address(asm),
                "alloca in loop",
                "stack pointer inside the scope",
            ),
            Rule::NoAlias => self.require(
                scan::params_are_noalias(&self.signature),
                "noalias",
                "pointer parameter is not noalias",
            ),
            Rule::NoGather => self.forbid(scan::has_gather(asm), "gather", "gather callee"),
            Rule::NoLibm => self.forbid(scan::has_libm(asm), "libm", "math call"),
            Rule::NoFma => self.forbid(scan::has_fma(asm), "fma", "fused multiply-add"),
        }
    }

    fn require(&self, passed: bool, expectation: &str, detail: &str) {
        if !passed {
            self.fail(expectation, detail);
        }
    }

    fn forbid(&self, failed: bool, expectation: &str, detail: &str) {
        if failed {
            self.fail(expectation, detail);
        }
    }

    fn at_most(&self, count: usize, max: u32, expectation: &str) {
        if count > widen(max) {
            self.fail(
                expectation,
                &format!("{count} {expectation}, limit is {max}"),
            );
        }
    }

    fn fail(&self, expectation: &str, detail: &str) -> ! {
        let snippet = first_lines(self.scope_text(), SNIPPET_LINES);
        panic!(
            "assertion failed: {expectation}\nscope: {}\n{detail}\n{snippet}",
            self.label
        );
    }
}

const SNIPPET_LINES: usize = 40;

fn first_lines(text: &str, max_lines: usize) -> String {
    let mut len = 0usize;
    let mut count = 0usize;
    for line in text.lines() {
        if count == max_lines {
            break;
        }
        if count > 0 {
            len += 1;
        }
        len += line.len();
        count += 1;
    }
    let mut snippet = String::with_capacity(len);
    count = 0;
    for line in text.lines() {
        if count == max_lines {
            break;
        }
        if count > 0 {
            snippet.push('\n');
        }
        snippet.push_str(line);
        count += 1;
    }
    snippet
}

/// Assembly instruction limit for one target triple.
#[derive(Debug)]
pub struct TargetAssert<'a> {
    asserts: &'a Asserts,
    triple: &'a str,
}

impl TargetAssert<'_> {
    /// Assembly instructions in this scope are at most `max`.
    ///
    /// # Panics
    ///
    /// Panics when the target probe has more instructions, or when `rustc` fails.
    pub fn assert_max_instructions(&self, max: u32) {
        let reloaded = load(
            &self.asserts.source,
            &self.asserts.function,
            self.asserts.kind,
            Some(self.triple),
        );
        let count = scan::asm_instructions(reloaded.scope_text());
        if count > widen(max) {
            reloaded.fail(
                "instructions",
                &format!("{count} assembly instructions, limit is {max}"),
            );
        }
    }
}

/// A set of asserts applied together.
#[must_use]
#[derive(Debug, Clone)]
pub struct Rules {
    items: Vec<Rule>,
}

impl Rules {
    /// An empty set.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Requires a vector operation.
    pub fn simd(self) -> Self {
        self.with(Rule::Simd)
    }

    /// Rejects bounds checks.
    pub fn no_bounds(self) -> Self {
        self.with(Rule::NoBounds)
    }

    /// Rejects extra conditional branches.
    pub fn no_branch(self) -> Self {
        self.with(Rule::NoBranch)
    }

    /// Rejects indirect calls.
    pub fn no_virtual_calls(self) -> Self {
        self.with(Rule::NoVirtualCalls)
    }

    /// Rejects conditional moves.
    pub fn no_select(self) -> Self {
        self.with(Rule::NoSelect)
    }

    /// Rejects division.
    pub fn no_div(self) -> Self {
        self.with(Rule::NoDiv)
    }

    /// Rejects unwind resumes.
    pub fn no_invoke(self) -> Self {
        self.with(Rule::NoInvoke)
    }

    /// Rejects allocator calls.
    pub fn no_alloc(self) -> Self {
        self.with(Rule::NoAlloc)
    }

    /// Rejects atomics.
    pub fn no_atomic(self) -> Self {
        self.with(Rule::NoAtomic)
    }

    /// Rejects memory copy symbols.
    pub fn no_memcpy(self) -> Self {
        self.with(Rule::NoMemcpy)
    }

    /// Rejects panics and asserts.
    pub fn no_checks(self) -> Self {
        self.with(Rule::NoChecks)
    }

    /// Caps loads.
    pub fn max_loads(self, max: u32) -> Self {
        self.with(Rule::MaxLoads(max))
    }

    /// Caps stores.
    pub fn max_stores(self, max: u32) -> Self {
        self.with(Rule::MaxStores(max))
    }

    /// Requires a counted loop.
    pub fn counted_loop(self) -> Self {
        self.with(Rule::CountedLoop)
    }

    /// Rejects stack-pointer addressing.
    pub fn no_alloca_in_loop(self) -> Self {
        self.with(Rule::NoAllocaInLoop)
    }

    /// Requires `noalias` on every pointer parameter.
    pub fn noalias(self) -> Self {
        self.with(Rule::NoAlias)
    }

    /// Rejects gather callees.
    pub fn no_gather(self) -> Self {
        self.with(Rule::NoGather)
    }

    /// Rejects math library calls.
    pub fn no_libm(self) -> Self {
        self.with(Rule::NoLibm)
    }

    /// Rejects fused multiply-add.
    pub fn no_fma(self) -> Self {
        self.with(Rule::NoFma)
    }

    /// Runs each rule on `asserts`.
    pub fn apply(&self, asserts: &Asserts) {
        for rule in &self.items {
            asserts.enforce(*rule);
        }
    }

    fn with(mut self, rule: Rule) -> Self {
        self.items.push(rule);
        self
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    Simd,
    NoBounds,
    NoBranch,
    NoVirtualCalls,
    NoSelect,
    NoDiv,
    NoInvoke,
    NoAlloc,
    NoAtomic,
    NoMemcpy,
    NoChecks,
    MaxLoads(u32),
    MaxStores(u32),
    CountedLoop,
    NoAllocaInLoop,
    NoAlias,
    NoGather,
    NoLibm,
    NoFma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Function,
    HotLoop,
}

fn widen(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn load(source: &Path, name: &str, kind: ScopeKind, target: Option<&str>) -> Asserts {
    let assembly = emit::compile(source, target).unwrap_or_else(|error| panic!("{error}"));
    let function = assembly
        .into_function(name)
        .unwrap_or_else(|error| panic!("{error}"));
    let (scope_start, scope_end) = match kind {
        ScopeKind::Function => (0, function.asm().len()),
        ScopeKind::HotLoop => function
            .hot_bounds()
            .unwrap_or_else(|error| panic!("{error}")),
    };
    let branch_allowance = match kind {
        ScopeKind::Function => 0,
        ScopeKind::HotLoop => 1,
    };
    let label = scope_label(kind, function.name());
    let crate::assembly::Function {
        name: function_name,
        asm,
        signature,
        hot: _,
    } = function;
    Asserts {
        label,
        asm,
        scope_start,
        scope_end,
        signature,
        branch_allowance,
        source: source.to_owned(),
        function: function_name,
        kind,
    }
}

fn scope_label(kind: ScopeKind, name: &str) -> String {
    let prefix = match kind {
        ScopeKind::Function => "function `",
        ScopeKind::HotLoop => "hot loop of `",
    };
    let mut label = String::with_capacity(prefix.len() + name.len() + 1);
    label.push_str(prefix);
    label.push_str(name);
    label.push('`');
    label
}

//! [`AsmTester`] checks one release probe.

use std::path::{Path, PathBuf};

use crate::emit;
use crate::ir::{Check, CheckKind};
use crate::scan;

/// Release asm and LLVM IR checks for one probe file.
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

    /// Selects the `define` named `name`.
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
    /// The hot loop is the innermost natural loop with the most instructions.
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
    ir: String,
    asm: String,
    signature: String,
    checks: Vec<Check>,
    branch_allowance: u32,
    loop_bodies: Vec<String>,
    source: PathBuf,
    function: String,
    kind: ScopeKind,
}

impl Asserts {
    /// The scope contains a vector type or vector instruction.
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

    /// The scope has no `select`.
    ///
    /// # Panics
    ///
    /// Panics when a `select` is present.
    pub fn assert_no_select(&self) {
        self.enforce(Rule::NoSelect);
    }

    /// The scope has no integer or float division.
    ///
    /// # Panics
    ///
    /// Panics when a div or rem opcode is present.
    pub fn assert_no_div(&self) {
        self.enforce(Rule::NoDiv);
    }

    /// The scope has no `invoke`.
    ///
    /// # Panics
    ///
    /// Panics when an `invoke` is present.
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
    /// Panics when an atomic opcode is present.
    pub fn assert_no_atomic(&self) {
        self.enforce(Rule::NoAtomic);
    }

    /// The scope has no `memcpy`, `memmove`, or `memset`.
    ///
    /// # Panics
    ///
    /// Panics when one of those intrinsics is present.
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

    /// `phi` instructions are at most `max`.
    ///
    /// # Panics
    ///
    /// Panics when the scope has more phis.
    pub fn assert_max_phis(&self, max: u32) {
        self.enforce(Rule::MaxPhis(max));
    }

    /// Basic blocks are at most `max`.
    ///
    /// # Panics
    ///
    /// Panics when the scope has more blocks.
    pub fn assert_max_basic_blocks(&self, max: u32) {
        self.enforce(Rule::MaxBasicBlocks(max));
    }

    /// The scope is a counted loop: an integer phi with a constant step and a compare.
    ///
    /// # Panics
    ///
    /// Panics when that pattern is missing.
    pub fn assert_counted_loop(&self) {
        self.enforce(Rule::CountedLoop);
    }

    /// No loop in the scope allocates stack memory per iteration.
    ///
    /// That is an `alloca`, `llvm.stacksave`, or `llvm.lifetime.start` inside a loop.
    ///
    /// # Panics
    ///
    /// Panics when a loop body has one of those operations.
    pub fn assert_no_alloca_in_loop(&self) {
        self.enforce(Rule::NoAllocaInLoop);
    }

    /// Every pointer parameter is `noalias`.
    ///
    /// A signature with no pointer parameter passes.
    ///
    /// # Panics
    ///
    /// Panics when a pointer parameter lacks `noalias`.
    pub fn assert_noalias(&self) {
        self.enforce(Rule::NoAlias);
    }

    /// The scope has no gather intrinsic or gather callee.
    ///
    /// # Panics
    ///
    /// Panics when a callee name contains `gather`.
    pub fn assert_no_gather(&self) {
        self.enforce(Rule::NoGather);
    }

    /// The scope has no `libm` intrinsic.
    ///
    /// # Panics
    ///
    /// Panics when a callee is `llvm.sin` or another listed math intrinsic.
    pub fn assert_no_libm(&self) {
        self.enforce(Rule::NoLibm);
    }

    /// The scope has no fused multiply-add.
    ///
    /// # Panics
    ///
    /// Panics when a callee is `llvm.fma` or `llvm.fmuladd`.
    pub fn assert_no_fma(&self) {
        self.enforce(Rule::NoFma);
    }

    /// The scope does not mention `i128`.
    ///
    /// # Panics
    ///
    /// Panics when the scope uses `i128`.
    pub fn assert_no_i128(&self) {
        self.enforce(Rule::NoI128);
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

    fn enforce(&self, rule: Rule) {
        match rule {
            Rule::Simd => self.require(
                scan::has_simd(&self.ir),
                "simd",
                "no vector type or vector instruction",
            ),
            Rule::NoBounds => self.forbid(self.bounds(), "bounds check", "bounds check is present"),
            Rule::NoBranch => {
                let count = scan::conditional_branches(&self.ir);
                self.forbid(
                    count > widen(self.branch_allowance),
                    "conditional branch",
                    &format!(
                        "{count} conditional branches, allowance is {}",
                        self.branch_allowance
                    ),
                );
            }
            Rule::NoVirtualCalls => self.forbid(
                scan::has_indirect_call(&self.ir),
                "virtual call",
                "indirect call",
            ),
            Rule::NoSelect => self.forbid(scan::has_opcode(&self.ir, "select"), "select", "select"),
            Rule::NoDiv => self.forbid(scan::has_div(&self.ir), "division", "div or rem"),
            Rule::NoInvoke => self.forbid(scan::has_opcode(&self.ir, "invoke"), "invoke", "invoke"),
            Rule::NoAlloc => self.forbid(scan::has_alloc(&self.ir), "allocation", "allocator call"),
            Rule::NoAtomic => self.forbid(scan::has_atomic(&self.ir), "atomic", "atomic operation"),
            Rule::NoMemcpy => self.forbid(
                scan::has_memcpy(&self.ir),
                "memcpy",
                "memcpy, memmove, or memset",
            ),
            Rule::NoChecks => {
                self.forbid(!self.checks.is_empty(), "panic check", "panic or assert");
            }
            Rule::MaxLoads(max) => self.at_most(scan::count_opcode(&self.ir, "load"), max, "loads"),
            Rule::MaxStores(max) => {
                self.at_most(scan::count_opcode(&self.ir, "store"), max, "stores");
            }
            Rule::MaxPhis(max) => self.at_most(scan::count_opcode(&self.ir, "phi"), max, "phis"),
            Rule::MaxBasicBlocks(max) => {
                self.at_most(scan::basic_blocks(&self.ir), max, "basic blocks");
            }
            Rule::CountedLoop => {
                self.require(
                    scan::is_counted_loop(&self.ir),
                    "counted loop",
                    "no integer induction",
                );
            }
            Rule::NoAllocaInLoop => self.forbid(
                self.loop_bodies
                    .iter()
                    .any(|body| scan::has_alloca_in_loop(body)),
                "alloca in loop",
                "stack allocation inside a loop",
            ),
            Rule::NoAlias => self.require(
                scan::params_are_noalias(&self.signature),
                "noalias",
                "pointer parameter is not noalias",
            ),
            Rule::NoGather => self.forbid(scan::has_gather(&self.ir), "gather", "gather callee"),
            Rule::NoLibm => self.forbid(scan::has_libm(&self.ir), "libm", "libm intrinsic"),
            Rule::NoFma => self.forbid(scan::has_fma(&self.ir), "fma", "fma intrinsic"),
            Rule::NoI128 => self.forbid(scan::has_i128(&self.ir), "i128", "i128"),
        }
    }

    fn bounds(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.kind() == CheckKind::Bounds)
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
        let snippet = self.ir.lines().take(40).collect::<Vec<_>>().join("\n");
        panic!(
            "assertion failed: {expectation}\nscope: {}\n{detail}\n{snippet}",
            self.label
        );
    }
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
        let count = scan::asm_instructions(&reloaded.asm);
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

    /// Rejects `select`.
    pub fn no_select(self) -> Self {
        self.with(Rule::NoSelect)
    }

    /// Rejects division.
    pub fn no_div(self) -> Self {
        self.with(Rule::NoDiv)
    }

    /// Rejects `invoke`.
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

    /// Rejects memory intrinsics.
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

    /// Caps phis.
    pub fn max_phis(self, max: u32) -> Self {
        self.with(Rule::MaxPhis(max))
    }

    /// Caps basic blocks.
    pub fn max_basic_blocks(self, max: u32) -> Self {
        self.with(Rule::MaxBasicBlocks(max))
    }

    /// Requires a counted loop.
    pub fn counted_loop(self) -> Self {
        self.with(Rule::CountedLoop)
    }

    /// Rejects stack allocation inside a loop.
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

    /// Rejects `libm` intrinsics.
    pub fn no_libm(self) -> Self {
        self.with(Rule::NoLibm)
    }

    /// Rejects fused multiply-add.
    pub fn no_fma(self) -> Self {
        self.with(Rule::NoFma)
    }

    /// Rejects `i128`.
    pub fn no_i128(self) -> Self {
        self.with(Rule::NoI128)
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
    MaxPhis(u32),
    MaxBasicBlocks(u32),
    CountedLoop,
    NoAllocaInLoop,
    NoAlias,
    NoGather,
    NoLibm,
    NoFma,
    NoI128,
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
        .function(name)
        .unwrap_or_else(|error| panic!("{error}"));
    let label_name = function.name().to_owned();
    let signature = function.signature().to_owned();
    match kind {
        ScopeKind::Function => Asserts {
            label: format!("function `{label_name}`"),
            ir: function.ir().to_owned(),
            asm: function.asm().to_owned(),
            signature,
            checks: function.checks().to_vec(),
            branch_allowance: 0,
            loop_bodies: function
                .loops()
                .iter()
                .map(|item| item.ir().to_owned())
                .collect(),
            source: source.to_owned(),
            function: name.to_owned(),
            kind,
        },
        ScopeKind::HotLoop => {
            let hot = function
                .hot_loop()
                .unwrap_or_else(|error| panic!("{error}"));
            Asserts {
                label: format!("hot loop of `{label_name}`"),
                ir: hot.ir().to_owned(),
                asm: hot.asm().to_owned(),
                signature,
                checks: hot.checks().to_vec(),
                branch_allowance: 1,
                loop_bodies: vec![hot.ir().to_owned()],
                source: source.to_owned(),
                function: name.to_owned(),
                kind,
            }
        }
    }
}

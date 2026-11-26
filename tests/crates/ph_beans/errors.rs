//! Emit-bit failures by error code.
//!
//! Each probe is compiled against the built `ph_beans` library. The code table
//! is `crates/ph_beans/docs/errors.md`.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use ph_test_errors::ErrorTester;

mod library;

use library::{CRATE, library};

/// Beans and events every probe starts from. `Up` holds slot 0 of `Events`.
const PRELUDE: &str = "
#![allow(unused)]
use ph_beans::{At, BeanInfo, Bit, EmitBit, EventInfo, EventReg, Ix, Marker, Query, SlotOf};
struct App; impl BeanInfo for App { type Provides=(); type Injects=(); type Beans=(); type State=ph_beans::NoState; }
struct Up; impl BeanInfo for Up { type Provides=(); type Injects=(); type Beans=(); type State=ph_beans::NoState; }
struct Low; impl BeanInfo for Low { type Provides=(); type Injects=(); type Beans=(); type State=ph_beans::NoState; }
impl EventInfo for Up { type Parents=(); }
impl EventInfo for Low { type Parents=Up; }
struct Events;
struct Elsewhere;
impl SlotOf<Events> for Bit<0> { type Owner = Up; }
impl EmitBit for Up { type Reg = Events; type Slot = Bit<0>; }
";

/// `App` takes its events from `Events`.
const APP_REG: &str = "impl EventReg for App { type Reg = Events; }";

const FILTER_PATH: &str = "fn main() { let _ = App::query().filter(Up::ON[Low::ON]); }";

#[test]
fn an_event_without_a_bit_is_filter_error_4() {
    fail(&format!("{APP_REG}\n{FILTER_PATH}"), "FilterError<4>");
}

#[test]
fn a_module_without_a_registry_is_filter_error_4() {
    fail(
        "fn main() { let _ = App::query().filter(Up::ON); }",
        "FilterError<4>",
    );
}

#[test]
fn an_event_of_another_registry_is_filter_error_4() {
    fail(
        "impl EventReg for App { type Reg = Elsewhere; }
         fn main() { let _ = App::query().filter(Up::ON); }",
        "FilterError<4>",
    );
}

#[test]
fn a_path_over_two_registries_is_filter_error_4() {
    let body = format!(
        "{APP_REG}
         impl SlotOf<Elsewhere> for Bit<0> {{ type Owner = Low; }}
         impl EmitBit for Low {{ type Reg = Elsewhere; type Slot = Bit<0>; }}
         {FILTER_PATH}"
    );
    fail(&body, "FilterError<4>");
}

#[test]
fn a_slot_with_no_owner_is_filter_error_5() {
    fail(
        "impl EmitBit for Low { type Reg = Events; type Slot = Bit<1>; }
         fn main() {}",
        "FilterError<5>",
    );
}

// The compiler reports a taken slot as a type mismatch on `SlotOf::Owner`.
#[test]
fn a_slot_of_another_event_is_a_type_mismatch() {
    let source = format!(
        "{PRELUDE}
         impl EmitBit for Low {{ type Reg = Events; type Slot = Bit<0>; }}
         fn main() {{}}"
    );
    ErrorTester::source(source)
        .extern_crate(CRATE, library())
        .assert_code("E0271");
}

// A provide names its access: `Get<Dep>` or `Mut<Dep>`, never a bare bean.
#[test]
fn a_provide_without_a_marker_is_pipeline_error_5() {
    fail(
        "#[derive(Default)] struct Dep; impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
         struct Host; impl BeanInfo for Host { type Provides=(Dep,); type Injects=(); type Beans=(Dep,); type State=u8; }
         fn boot(_: ()) -> u8 { 0 }
         fn main() { let _ = Host::query().init_state(boot); }",
        "PipelineError<5>",
    );
}

#[test]
fn a_provided_state_without_default_is_pipeline_error_3() {
    fail(
        "struct Raw;
         struct Dep; impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=Raw; }
         struct Host; impl BeanInfo for Host { type Provides=(ph_beans::Mut<Dep>,); type Injects=(); type Beans=(ph_beans::Mut<Dep>,); type State=u8; }
         fn boot(_: ()) -> u8 { 0 }
         fn main() { let _ = Host::query().init_state(boot); }",
        "PipelineError<3>",
    );
}

#[test]
fn mut_of_a_shared_inject_is_pipeline_error_4() {
    fail(
        "struct Dep; impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
         struct Host; impl BeanInfo for Host { type Provides=(); type Injects=ph_beans::Get<Dep>; type Beans=(Dep,); type State=u8; }
         impl At<Dep> for Host { type Ix = Ix<0>; }
         fn boot(dep: &mut u8) -> u8 { *dep }
         fn main() { let _ = Host::query().inject_resources(Dep::MUT).init_state(boot); }",
        "PipelineError<4>",
    );
}

// `App` does not provide `Up`, and the state of `Up` holds no buffer.
#[test]
fn a_declared_emit_without_an_event_buffer_is_filter_error_7() {
    let body = format!(
        "{APP_REG}
         use ph_beans::Tick;
         impl SlotOf<Events> for Bit<1> {{ type Owner = Low; }}
         impl EmitBit for Low {{ type Reg = Events; type Slot = Bit<1>; }}
         fn main() {{
             let tick = App::query().emit(Up::ON[Low::ON]).build();
             let _ = Tick::<App>::init(&tick, ());
         }}"
    );
    fail(&body, "FilterError<7>");
}

// A bean provided under `Get` is made by `init` and never changes.
#[test]
fn mut_of_a_shared_provide_is_pipeline_error_4() {
    fail(
        "#[derive(Default)] struct Dep; impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
         struct Host; impl BeanInfo for Host { type Provides=(ph_beans::Get<Dep>,); type Injects=(); type Beans=(ph_beans::Get<Dep>,); type State=u8; }
         impl At<Dep> for Host { type Ix = Ix<0>; }
         fn boot(dep: &mut u8) -> u8 { *dep }
         fn main() { let _ = Host::query().inject_resources(*Dep::MUT).init_state(boot); }",
        "PipelineError<4>",
    );
}

// The status of a claim list names the bean and the code.
#[test]
fn a_bean_claimed_twice_is_claim_error_1() {
    fail(
        "#[derive(Default)] struct Dep; impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
         struct Host; impl BeanInfo for Host { type Provides=(ph_beans::Mut<Dep>,); type Injects=(); type Beans=(ph_beans::Mut<Dep>,); type State=u8; }
         impl At<Dep> for Host { type Ix = Ix<0>; }
         fn main() { let _ = Host::query().inject_resources(*Dep::T & *Dep::T); }",
        "ClaimError<Dep, 1>",
    );
}

#[test]
fn a_bean_claimed_exclusive_twice_is_claim_error_1() {
    fail(
        "#[derive(Default)] struct Dep; impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
         struct Host; impl BeanInfo for Host { type Provides=(ph_beans::Mut<Dep>,); type Injects=(); type Beans=(ph_beans::Mut<Dep>,); type State=u8; }
         impl At<Dep> for Host { type Ix = Ix<0>; }
         fn main() { let _ = Host::query().inject_resources(*Dep::MUT & Dep::MUT); }",
        "ClaimError<Dep, 1>",
    );
}

#[test]
fn a_bean_claimed_shared_and_exclusive_is_claim_error_2() {
    fail(
        "#[derive(Default)] struct Dep; impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
         struct Host; impl BeanInfo for Host { type Provides=(ph_beans::Mut<Dep>,); type Injects=(); type Beans=(ph_beans::Mut<Dep>,); type State=u8; }
         impl At<Dep> for Host { type Ix = Ix<0>; }
         fn main() { let _ = Host::query().inject_resources(*Dep::T & *Dep::MUT); }",
        "ClaimError<Dep, 2>",
    );
}

// A bean injected as `Dep::T as View` is lent shared: it cannot be changed.
#[test]
fn mut_of_a_shared_dyn_inject_is_pipeline_error_4() {
    fail(
        "use ph_beans::{Dyn, DynObject, Get};
         trait View {} struct Dep; impl View for Dep {}
         impl BeanInfo for Dep { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
         impl<'a> DynObject<'a> for dyn View { type Obj = dyn View + 'a; fn shared(&'a self) -> &'a Self::Obj { self } fn exclusive(&'a mut self) -> &'a mut Self::Obj { self } }
         struct Host; impl BeanInfo for Host { type Provides=(); type Injects=Dyn<Get<Dep>, dyn View>; type Beans=Dyn<Get<Dep>, dyn View>; type State=u8; }
         impl At<Dep> for Host { type Ix = Ix<0>; }
         fn boot(_: &mut dyn View) -> u8 { 0 }
         fn main() { let _ = Host::query().inject_resources(Dep::MUT).init_state(boot); }",
        "PipelineError<4>",
    );
}

/// Children for the `.forward` and `.init_state` probes. `Leaf` has no
/// pipeline. `Kid` has one and changes nothing of its own. `Grip` has one
/// and provides a bean under `Mut`.
const CHILDREN: &str = "
use ph_beans::{Get, Mut, Setup, Tick};
fn boot(_: ()) -> u8 { 0 }
#[derive(Default)] struct Leaf; impl BeanInfo for Leaf { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
#[derive(Default)] struct Kid; impl BeanInfo for Kid { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
#[derive(Default)] struct Grip; impl BeanInfo for Grip { type Provides=(Mut<Leaf>,); type Injects=(); type Beans=(Mut<Leaf>,); type State=u8; }
impl At<Leaf> for Grip { type Ix = Ix<0>; }
impl Setup for Kid { fn setup(self) -> impl Tick<Self> { Kid::query().init_state(boot).build() } }
impl Setup for Grip { fn setup(self) -> impl Tick<Self> { Grip::query().init_state(boot).build() } }
";

/// A `.forward` that does not compile: `Host` provides `child` under the
/// marker type `provided` and names it with the marker `forward`.
fn forward_fails(child: &str, provided: &str, forward: &str, code: &str) {
    let body = format!(
        "{CHILDREN}
         struct Host; impl BeanInfo for Host {{ type Provides=({provided}<{child}>,); type Injects=(); type Beans=({provided}<{child}>,); type State=u8; }}
         impl At<{child}> for Host {{ type Ix = Ix<0>; }}
         fn main() {{
             let tick = Host::query().init_state(boot).forward({child}::{forward}).build();
             let _ = Tick::<Host>::init(&tick, ());
         }}"
    );
    fail(&body, code);
}

// A forwarded bean runs its own pipeline: it needs `Setup`.
#[test]
fn a_forwarded_bean_without_a_pipeline_is_pipeline_error_6() {
    forward_fails("Leaf", "Mut", "MUT", "PipelineError<6>");
}

// The marker of a `.forward` is the marker of the provide.
#[test]
fn a_forward_under_t_of_a_mut_provide_is_pipeline_error_8() {
    forward_fails("Kid", "Mut", "T", "PipelineError<8>");
}

#[test]
fn a_forward_under_mut_of_a_get_provide_is_pipeline_error_8() {
    forward_fails("Kid", "Get", "MUT", "PipelineError<8>");
}

// A child under `Get` changes nothing of its own. A provide under `Mut` is
// something it can change.
#[test]
fn a_child_with_a_mut_provide_forwarded_under_t_is_pipeline_error_7() {
    forward_fails("Grip", "Get", "T", "PipelineError<7>");
}

/// A pipeline of `module` that does not compile because of its init step:
/// `steps` is the chain after `query()`. `App` has no state, `Kid` a `u8`.
fn init_fails(module: &str, steps: &str, code: &str) {
    fail(
        &format!("{CHILDREN}\nfn main() {{ let _ = {module}::query(){steps}; }}"),
        code,
    );
}

// A module without state names `NoState`. Nothing outside the library makes
// that value, so it takes no init function.
#[test]
fn init_state_of_a_module_without_state_is_pipeline_error_9() {
    init_fails("App", ".init_state(|_| ())", "PipelineError<9>");
}

#[test]
fn a_default_init_of_a_module_without_state_is_rejected() {
    init_fails(
        "App",
        ".init_state(|_| Default::default())",
        "NoState: Default",
    );
}

// An init function returns the state of its module, nothing else.
#[test]
fn an_init_that_returns_another_type_is_pipeline_error_9() {
    init_fails("Kid", ".init_state(|_| 0u16)", "PipelineError<9>");
}

// A module with a state is not built without its init function.
#[test]
fn a_module_with_state_built_without_init_is_need_init() {
    init_fails("Kid", ".build()", "NeedInit");
}

/// A module that provides the descriptor of `described` and claims it
/// with the marker `claim`.
fn info_fails(described: &str, claim: &str, code: &str) {
    let body = format!(
        "{CHILDREN}
         use ph_beans::{{Info, NoState}};
         fn reads(_: &Info<{described}>) {{}}
         struct Host; impl BeanInfo for Host {{ type Provides=(Get<Info<{described}>>,); type Injects=(); type Beans=(Get<Info<{described}>>,); type State=NoState; }}
         impl At<Info<{described}>> for Host {{ type Ix = Ix<0>; }}
         fn main() {{
             let tick = Host::query().inject_resources(Info::<{described}>::{claim}).map(reads).build();
             let _ = Tick::<Host>::init(&tick, ());
         }}"
    );
    fail(&body, code);
}

// A descriptor is held under `Get`: it is made by `init` and never changes.
#[test]
fn mut_of_a_descriptor_is_pipeline_error_4() {
    info_fails("Kid", "MUT", "PipelineError<4>");
}

// The descriptor is built from the pipeline of the bean. `Leaf` has none.
#[test]
fn a_descriptor_of_a_bean_without_a_pipeline_is_pipeline_error_3() {
    info_fails("Leaf", "T", "PipelineError<3>");
}

#[test]
fn two_owners_of_one_slot_conflict() {
    let source = format!(
        "{PRELUDE}
         impl SlotOf<Events> for Bit<0> {{ type Owner = Low; }}
         fn main() {{}}"
    );
    ErrorTester::source(source)
        .extern_crate(CRATE, library())
        .assert_code("E0119");
}

fn fail(body: &str, description: &str) {
    let source = format!("{PRELUDE}\n{body}\n");
    ErrorTester::source(source)
        .extern_crate(CRATE, library())
        .assert_description(description);
}

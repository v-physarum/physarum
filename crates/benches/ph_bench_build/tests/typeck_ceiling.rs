//! Nightly `type_check_crate` ceiling for a function-item map chain.
//!
//! Stable rustc has no `-Ztime-passes`, so the build is ignored there.
//! [`TYPE_CHECK_CRATE_LIMIT`] is 20 ms at scale 16. From scale 1 to 16,
//! [`Growth::Linear`] still passes a jump of about 13× (5 ms to 67 ms).

use std::time::Duration;

use ph_bench_build::{
    Group, Growth, PassCeiling, RustcPass, Scale, Series, Solver, TYPE_CHECK_CRATE_LIMIT,
};

const USE_LINE: &str = "use ph_beans::{BeanInfo, Query, Update, NoState};\n\n";
const BEAN: &str = "\
struct App;

impl BeanInfo for App {
    type Provides = ();
    type Injects = ();
    type Beans = ();
    type State = NoState;
}
";
const CHAIN_START: &str = "\
pub fn chain() {
    let mut built = App::query()
";
const CHAIN_END: &str = "\
        .build();
    Update::<App>::update(&mut built, ());
}
";
const SYSTEM_PREFIX: &str = "fn system_";
const SYSTEM_SUFFIX: &str = "() {}\n";
const MAP_PREFIX: &str = "        .map(system_";
const MAP_SUFFIX: &str = ")\n";
/// Decimal digits of a `u32`.
const DIGIT_CAP: usize = 10;
/// Bytes reserved for one generated system and its `.map` line.
const STEP_CAP: usize = SYSTEM_PREFIX.len()
    + DIGIT_CAP
    + SYSTEM_SUFFIX.len()
    + MAP_PREFIX.len()
    + DIGIT_CAP
    + MAP_SUFFIX.len();

#[test]
fn the_type_check_ceiling_is_twenty_milliseconds() {
    assert_eq!(TYPE_CHECK_CRATE_LIMIT, Duration::from_millis(20));
    let ceiling = PassCeiling::new(
        RustcPass::TypeCheckCrate,
        Scale::new(16),
        TYPE_CHECK_CRATE_LIMIT,
    );
    assert_eq!(ceiling.pass().name(), "type_check_crate");
    ceiling.check_seconds(0.008).unwrap();
    ceiling.check_seconds(0.020).unwrap();
    let slow = ceiling.check_seconds(0.067).unwrap_err().to_string();
    assert!(slow.contains("67"), "{slow}");
    assert!(slow.contains("20"), "{slow}");
    assert!(ceiling.check_seconds(f64::NAN).is_err());
}

#[test]
fn a_growth_claim_and_a_ceiling_cannot_both_be_set() {
    let error = Series::new("bean_map")
        .group(Group::Debug)
        .scales([1, 16])
        .expect(Growth::Linear)
        .ceiling(PassCeiling::new(
            RustcPass::TypeCheckCrate,
            Scale::new(16),
            TYPE_CHECK_CRATE_LIMIT,
        ))
        .render(|_| "fn system() {}\n".to_owned())
        .check()
        .unwrap_err()
        .to_string();
    assert!(error.contains("both"), "{error}");
}

#[test]
fn bean_map_source_is_a_function_item_chain() {
    let source = bean_map(Scale::new(2));
    assert!(source.starts_with(USE_LINE));
    assert!(source.contains("fn system_0() {}"));
    assert!(source.contains("fn system_1() {}"));
    assert!(source.contains(".map(system_0)"));
    assert!(source.contains(".map(system_1)"));
    assert!(source.contains(".build()"));
    assert!(source.contains("Update::<App>::update"));
    assert!(!source.contains('|'), "{source}");
}

#[test]
#[ignore = "nightly rustc -Ztime-passes"]
fn bean_map_type_check_stays_under_the_ceiling() {
    Series::new("bean_map")
        .group(Group::Debug)
        .scales([1, 4, 8, 16])
        .ceiling(PassCeiling::new(
            RustcPass::TypeCheckCrate,
            Scale::new(16),
            TYPE_CHECK_CRATE_LIMIT,
        ))
        .solver(Solver::Next)
        .render(bean_map)
        .check()
        .unwrap();
}

fn bean_map(scale: Scale) -> String {
    let count = scale.count();
    let mut source = String::with_capacity(
        USE_LINE.len()
            + BEAN.len()
            + CHAIN_START.len()
            + CHAIN_END.len()
            + STEP_CAP * count as usize,
    );
    source.push_str(USE_LINE);
    source.push_str(BEAN);
    for index in 0..count {
        source.push_str(SYSTEM_PREFIX);
        push_u32(&mut source, index);
        source.push_str(SYSTEM_SUFFIX);
    }
    source.push_str(CHAIN_START);
    for index in 0..count {
        source.push_str(MAP_PREFIX);
        push_u32(&mut source, index);
        source.push_str(MAP_SUFFIX);
    }
    source.push_str(CHAIN_END);
    source
}

fn push_u32(out: &mut String, mut value: u32) {
    let mut bytes = [0u8; DIGIT_CAP];
    let mut at = bytes.len();
    loop {
        at -= 1;
        bytes[at] = b"0123456789"[(value % 10) as usize];
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for byte in &bytes[at..] {
        out.push(char::from(*byte));
    }
}

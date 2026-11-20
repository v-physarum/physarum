//! JSON readers for the compile-time bench.

#[path = "../../../benches/crates/ph_util_foreach/json.rs"]
mod json;

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
    let clock = json::wall_clock(json).unwrap();
    assert_eq!(clock.median.to_bits(), 1.5_f64.to_bits());
    assert_eq!(clock.stddev.to_bits(), 0.25_f64.to_bits());
}

#[test]
fn empty_hyperfine_results_are_an_error() {
    let Err(error) = json::wall_clock(r#"{"results":[]}"#) else {
        panic!("empty results parsed");
    };
    assert!(error.contains("time_wall_clock"), "{error}");
}

#[test]
fn reads_runtime_executable() {
    let log = r#"{"reason":"compiler-message","message":"note"}
{"reason":"compiler-artifact","executable":null,"target":{"name":"ph_util_foreach_runtime"}}
{"reason":"compiler-artifact","executable":"/tmp/runtime","target":{"name":"ph_util_foreach_runtime"}}
"#;
    let path = json::runtime_executable(log).unwrap();
    assert_eq!(path.as_os_str(), "/tmp/runtime");
}

#[test]
fn missing_runtime_executable_is_an_error() {
    let log = r#"{"reason":"compiler-artifact","executable":null,"target":{"name":"other"}}"#;
    let Err(error) = json::runtime_executable(log) else {
        panic!("missing executable parsed");
    };
    assert!(error.contains("executable"), "{error}");
}

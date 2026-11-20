use proc_macro2::TokenStream;

fn assert_expands(input: &str, expected: &str) {
    let actual = expand_tokens(input);
    if expected.is_empty() {
        assert!(actual.is_empty(), "actual: {actual}");
        return;
    }
    let expected = normalize(expected);
    assert_eq!(actual, expected, "actual: {actual}");
}

fn assert_err(input: &str, needle: &str) {
    let input = syn::parse_str::<TokenStream>(input).unwrap_or_else(|error| {
        panic!("input tokens: {error}");
    });
    let message = crate::body::expand(input)
        .expect_err("expected an error")
        .to_string();
    assert!(
        message.contains(needle),
        "error `{message}` does not contain `{needle}`"
    );
}

fn expand_tokens(input: &str) -> String {
    let input = syn::parse_str::<TokenStream>(input).unwrap_or_else(|error| {
        panic!("input tokens: {error}");
    });
    crate::body::expand(input)
        .unwrap_or_else(|error| panic!("expand: {error}"))
        .to_string()
}

fn normalize(input: &str) -> String {
    syn::parse_str::<TokenStream>(input)
        .unwrap_or_else(|error| panic!("expected tokens: {error}"))
        .to_string()
}

fn assert_name_pattern(outer: &str, pattern: &str, times: usize, ids: &[i128]) {
    let list = ids
        .iter()
        .map(|index| format!("Name{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let one = if list.is_empty() {
        "K ;".to_owned()
    } else {
        format!("K {list};")
    };
    let expected = vec![one; times].join(" ");
    assert_expands(&format!("{outer} {{ K Name{pattern}; }}"), &expected);
}

fn range_i128(start: i128, end: i128) -> Vec<i128> {
    (start..end).collect()
}

#[test]
fn flat_structs_and_stepped_outer_ranges() {
    assert_expands(
        "0..4 { pub struct Ix#; }",
        "pub struct Ix0; pub struct Ix1; pub struct Ix2; pub struct Ix3;",
    );
    assert_expands(
        "0..=3 { pub struct Ix#; }",
        "pub struct Ix0; pub struct Ix1; pub struct Ix2; pub struct Ix3;",
    );
    assert_expands(
        "0..10::2 { pub struct Ix#; }",
        "pub struct Ix0; pub struct Ix2; pub struct Ix4; pub struct Ix6; pub struct Ix8;",
    );
    assert_expands(
        "0..=9::2 { pub struct Ix#; }",
        "pub struct Ix0; pub struct Ix2; pub struct Ix4; pub struct Ix6; pub struct Ix8;",
    );
    assert_expands(
        "1..10::2 { pub struct Ix#; }",
        "pub struct Ix1; pub struct Ix3; pub struct Ix5; pub struct Ix7; pub struct Ix9;",
    );
    assert_expands(
        "2..=5 { pub struct Ix#; }",
        "pub struct Ix2; pub struct Ix3; pub struct Ix4; pub struct Ix5;",
    );
    assert_expands("3..=3 { struct Ix#; }", "struct Ix3;");
    assert_expands("10..=12::2 { struct Ix#; }", "struct Ix10; struct Ix12;");
    assert_expands("0..1::2 { struct Ix#; }", "struct Ix0;");
    assert_expands("5..5 { K; }", "");
}

#[test]
fn pattern_forms_and_steps() {
    assert_name_pattern("0..10", "[0..10]", 10, &range_i128(0, 10));
    assert_name_pattern("0..=10", "[0..=10]", 11, &range_i128(0, 11));
    assert_name_pattern("0..10", "[0..]", 10, &range_i128(0, 10));
    assert_name_pattern("0..10", "[..10]", 10, &range_i128(0, 10));
    assert_name_pattern("0..=10", "[..=10]", 11, &range_i128(0, 11));
    assert_name_pattern("0..4", "[..]", 4, &range_i128(0, 4));
    assert_name_pattern("0..10", "[0..10::2]", 10, &[0, 2, 4, 6, 8]);
    assert_name_pattern("0..=9", "[0..=9::2]", 10, &[0, 2, 4, 6, 8]);
    assert_name_pattern("0..10", "[..::2]", 10, &[0, 2, 4, 6, 8]);
    assert_name_pattern("0..=10", "[0..::2]", 11, &[0, 2, 4, 6, 8, 10]);
    assert_name_pattern("0..=10", "[..10::2]", 11, &[0, 2, 4, 6, 8]);
    assert_name_pattern("0..=10", "[..=9::2]", 11, &[0, 2, 4, 6, 8]);
    assert_name_pattern("0..=10", "[2..=5]", 11, &[2, 3, 4, 5]);
    assert_name_pattern("0..=10", "[1..::2]", 11, &[1, 3, 5, 7, 9]);
    assert_name_pattern("0..=4", "[-1..=-1]", 5, &[4]);
    assert_name_pattern("0..=4", "[0..=-1]", 5, &range_i128(0, 5));
    assert_name_pattern("0..=4", "[-1..]", 5, &[4]);
}

#[test]
fn flat_domain_is_the_full_outer_set() {
    assert_expands(
        "0..3 { X Name[..]; }",
        "X Name0, Name1, Name2; X Name0, Name1, Name2; X Name0, Name1, Name2;",
    );
    assert_expands(
        "0..3 { Ix# Name[..]; }",
        "Ix0 Name0, Name1, Name2; Ix1 Name0, Name1, Name2; Ix2 Name0, Name1, Name2;",
    );
}

#[test]
fn fan_domain_is_the_prefix_through_the_outer_index() {
    assert_expands(
        "0..3 { X Name[..]; Y Name[..=#]; }",
        "X Name0; Y Name0; X Name0, Name1; Y Name0, Name1; X Name0, Name1, Name2; Y Name0, Name1, Name2;",
    );
    assert_expands(
        "0..3 { K Name[#]; }",
        "K Name0; K Name0, Name1; K Name0, Name1, Name2;",
    );
    // Outer step does not thin the fan prefix.
    assert_expands(
        "0..6::2 { K Name[..=#]; }",
        "K Name0; K Name0, Name1, Name2; K Name0, Name1, Name2, Name3, Name4;",
    );
    assert_expands(
        "0..6::2 { K Name[..]; }",
        "K Name0, Name2, Name4; K Name0, Name2, Name4; K Name0, Name2, Name4;",
    );
}

#[test]
fn gapped_domain_filters_members_and_rejects_a_far_end() {
    let evens = "K Name0, Name2, Name4, Name6, Name8;";
    assert_expands("0..10::2 { K Name[0..10]; }", &[evens; 5].join(" "));
    let middle = "K Name2, Name4, Name6;";
    assert_expands("0..10::2 { K Name[1..=7]; }", &[middle; 5].join(" "));
    assert_err("0..10::2 { K Name[0..12]; }", "outside the domain");
}

#[test]
fn placeholders_concat_tuples_off_and_hash_offsets() {
    assert_expands("0..2 { K Off[#]; }", "K Off; K Off;");
    assert_expands("0..2 { K Off[0..2]; }", "K Off0, Off1; K Off0, Off1;");
    assert_expands(
        "0..2 { K Name[0..0] + Name[0..2]; }",
        "K Name0, Name1; K Name0, Name1;",
    );
    assert_expands(
        "0..2 { K Name[0..2] + 1; }",
        "K Name0, Name1 + 1; K Name0, Name1 + 1;",
    );
    assert_expands(
        "0..3 { K (T#, U#)[0..3]; }",
        &["K (T0, U0), (T1, U1), (T2, U2);"; 3].join(" "),
    );
    assert_expands(
        "0..3 { K (T#)[..=#]; }",
        "K (T0); K (T0), (T1); K (T0), (T1), (T2);",
    );
    assert_expands(
        "1..3 { K Name[#-1..=#]; }",
        "K Name0, Name1; K Name1, Name2;",
    );
    assert_expands("1..3 { K Name[0..=#::#+1]; }", "K Name0; K Name0;");
    assert_expands("0..2 { K Name[#+1..=#]; }", "K ; K ;");
    assert_expands("0..4 { K Name[4..=3]; }", "K ; K ; K ; K ;");
}

#[test]
fn ordinary_tokens_stay_in_place() {
    assert_expands(
        "0..2 { #[allow(dead_code)] struct Ix#; }",
        "#[allow(dead_code)] struct Ix0; #[allow(dead_code)] struct Ix1;",
    );
    assert_expands(
        "0..1 { fn f(a: i32, b: i32) -> i32 { a + b } }",
        "fn f(a: i32, b: i32) -> i32 { a + b }",
    );
    assert_expands("0..1 { let x = values[i]; }", "let x = values[i];");
    assert_expands("0..1 { let x = Name[]; }", "let x = Name[];");
    assert_expands(
        "0..2 { let x = arr[(#)]; }",
        "let x = arr[(0)]; let x = arr[(1)];",
    );
}

#[test]
fn impl_take_fan_example() {
    assert_expands(
        "0..4 { impl_take!(args: (T[..=#]), ix: Ix#, out: T#, rest: T[..#] + Off[#] + T[#+1..=#],); }",
        "impl_take!(args: (T0), ix: Ix0, out: T0, rest: Off,);
         impl_take!(args: (T0, T1), ix: Ix1, out: T1, rest: T0, Off,);
         impl_take!(args: (T0, T1, T2), ix: Ix2, out: T2, rest: T0, T1, Off,);
         impl_take!(args: (T0, T1, T2, T3), ix: Ix3, out: T3, rest: T0, T1, T2, Off,);",
    );
}

#[test]
fn negative_index_literal_and_paste() {
    let actual = expand_tokens("-1..1 { const N: i128 = #; }");
    assert!(
        actual.contains("-1") || actual.contains("- 1"),
        "expected a negative literal in `{actual}`"
    );
    assert!(actual.contains('0'), "expected zero in `{actual}`");
    assert_err("-1..0 { struct Ix#; }", "cannot paste a negative index");
    assert_err(
        "-1..0 { K Name[#]; }",
        "fan mode domain `0..=index` is empty",
    );
}

#[test]
fn outer_range_errors() {
    for input in [
        "0.. { K; }",
        "..4 { K; }",
        "..=4 { K; }",
        ".. { K; }",
        "0..::2 { K; }",
        "..4::2 { K; }",
        "..=4::2 { K; }",
        "..::2 { K; }",
    ] {
        assert_err(input, "open outer range");
    }
    assert_err("5..3 { K; }", "invalid range");
    assert_err("5..=3 { K; }", "invalid range");
    assert_err("5..4 { K; }", "invalid range");
    assert_err("0..4::0 { K; }", "step must be a positive integer");
    assert_err("0..4::-1 { K; }", "step must be a positive integer");
    assert_err("0..# { K; }", "outer range cannot use `#`");
    assert_err("0..4::# { K; }", "outer range cannot use `#`");
    assert_err("0..4", "expected a `{ ... }` body");
    assert_err("{ K; }", "expected a range");
    assert_err("0..2 { K; } extra", "unexpected token after the body");
}

#[test]
fn pattern_errors() {
    assert_err("0..4 { K Name[0..10]; }", "outside the domain");
    assert_err("0..=10 { K Name[4..=3]; }", "invalid range");
    assert_err("0..4 { A[0..#, Off]; }", "exactly one range pattern");
    assert_err("0..4 { Name[0..1..2]; }", "exactly one range pattern");
    assert_err("0..4 { Name[0..2::]; }", "expected a step");
    assert_err(
        "0..4 { K Name[0..4::0]; }",
        "step must be a positive integer",
    );
    assert_err(
        "1..2 { K Name[0..=#::#-1]; }",
        "step must be a positive integer",
    );
    assert_err("0..2 { K Name[#-1..=#]; }", "outside the domain");
    assert_err("0..4 { K Name[#+]; }", "expected an integer literal");
}

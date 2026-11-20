#![allow(missing_docs, clippy::must_use_candidate)]

#[allow(dead_code)]
mod generated {
    use ph_util_foreach::impl_foreach;

    impl_foreach!(0..=10 {
        pub fn excl #() -> &'static str {
            stringify!(Name[0..10])
        }
        pub fn incl #() -> &'static str {
            stringify!(Name[0..=10])
        }
        pub fn from #() -> &'static str {
            stringify!(Name[0..])
        }
        pub fn to #() -> &'static str {
            stringify!(Name[..10])
        }
        pub fn to_incl #() -> &'static str {
            stringify!(Name[..=10])
        }
        pub fn full #() -> &'static str {
            stringify!(Name[..])
        }
        pub fn step_excl #() -> &'static str {
            stringify!(Name[0..10::2])
        }
        pub fn step_incl #() -> &'static str {
            stringify!(Name[0..=9::2])
        }
        pub fn step_full #() -> &'static str {
            stringify!(Name[..::2])
        }
        pub fn step_from #() -> &'static str {
            stringify!(Name[0..::2])
        }
        pub fn step_to #() -> &'static str {
            stringify!(Name[..10::2])
        }
        pub fn step_to_incl #() -> &'static str {
            stringify!(Name[..=9::2])
        }
        pub fn window #() -> &'static str {
            stringify!(Name[2..=5])
        }
        pub fn from_one_step #() -> &'static str {
            stringify!(Name[1..::2])
        }
        pub fn last #() -> &'static str {
            stringify!(Name[-1..=-1])
        }
    });
}

fn idents(value: &str) -> Vec<&str> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect()
}

fn expect(ids: &[i128]) -> Vec<String> {
    ids.iter().map(|index| format!("Name{index}")).collect()
}

fn assert_names(value: &str, ids: &[i128]) {
    let got = idents(value);
    let want = expect(ids);
    assert_eq!(got, want.iter().map(String::as_str).collect::<Vec<_>>());
}

fn through(end_inclusive: i128) -> Vec<i128> {
    (0..=end_inclusive).collect()
}

#[test]
fn every_range_form_and_step() {
    assert_names(generated::excl0(), &through(9));
    assert_names(generated::incl0(), &through(10));
    assert_names(generated::from0(), &through(10));
    assert_names(generated::to0(), &through(9));
    assert_names(generated::to_incl0(), &through(10));
    assert_names(generated::full0(), &through(10));
    assert_names(generated::step_excl0(), &[0, 2, 4, 6, 8]);
    assert_names(generated::step_incl0(), &[0, 2, 4, 6, 8]);
    assert_names(generated::step_full0(), &[0, 2, 4, 6, 8, 10]);
    assert_names(generated::step_from0(), &[0, 2, 4, 6, 8, 10]);
    assert_names(generated::step_to0(), &[0, 2, 4, 6, 8]);
    assert_names(generated::step_to_incl0(), &[0, 2, 4, 6, 8]);
    assert_names(generated::window0(), &[2, 3, 4, 5]);
    assert_names(generated::from_one_step0(), &[1, 3, 5, 7, 9]);
    assert_names(generated::last0(), &[10]);
}

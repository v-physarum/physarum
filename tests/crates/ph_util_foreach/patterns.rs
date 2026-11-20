#![allow(missing_docs, clippy::must_use_candidate)]

#[allow(dead_code)]
mod generated {
    use ph_util_foreach::impl_foreach;

    impl_foreach!(range = 0..=10, body = {
        pub fn excl + #() -> &'static str {
            stringify!(Name[0..10])
        }
        pub fn incl + #() -> &'static str {
            stringify!(Name[0..=10])
        }
        pub fn from + #() -> &'static str {
            stringify!(Name[0..])
        }
        pub fn to + #() -> &'static str {
            stringify!(Name[..10])
        }
        pub fn to_incl + #() -> &'static str {
            stringify!(Name[..=10])
        }
        pub fn full + #() -> &'static str {
            stringify!(Name[..])
        }
        pub fn step_excl + #() -> &'static str {
            stringify!(Name[0..10::2])
        }
        pub fn step_incl + #() -> &'static str {
            stringify!(Name[0..=9::2])
        }
        pub fn step_full + #() -> &'static str {
            stringify!(Name[..::2])
        }
        pub fn step_from + #() -> &'static str {
            stringify!(Name[0..::2])
        }
        pub fn step_to + #() -> &'static str {
            stringify!(Name[..10::2])
        }
        pub fn step_to_incl + #() -> &'static str {
            stringify!(Name[..=9::2])
        }
        pub fn window + #() -> &'static str {
            stringify!(Name[2..=5])
        }
        pub fn from_one_step + #() -> &'static str {
            stringify!(Name[1..::2])
        }
        pub fn last + #() -> &'static str {
            stringify!(Name[-1..=-1])
        }
    });
}

/// One identifier in expanded text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ident<'a>(&'a str);

impl<'a> Ident<'a> {
    fn new(text: &'a str) -> Self {
        Self(text)
    }
}

/// Text produced by one expanded item.
#[derive(Clone, Copy)]
struct Expanded<'a>(&'a str);

impl<'a> Expanded<'a> {
    fn new(text: &'a str) -> Self {
        Self(text)
    }

    fn idents(self) -> Vec<Ident<'a>> {
        self.0
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|part| !part.is_empty())
            .map(Ident::new)
            .collect()
    }
}

/// The pasted spelling `Name{index}`.
#[derive(Debug)]
struct Spelled(String);

impl PartialEq<Spelled> for Ident<'_> {
    fn eq(&self, other: &Spelled) -> bool {
        self.0 == other.0
    }
}

/// The indexes a pattern is expected to paste.
struct Indexes(Vec<i128>);

impl Indexes {
    fn through(end_inclusive: i128) -> Self {
        Self((0..=end_inclusive).collect())
    }

    fn list(ids: &[i128]) -> Self {
        Self(ids.to_vec())
    }

    fn names(&self) -> Vec<Spelled> {
        self.0
            .iter()
            .map(|index| Spelled(format!("Name{index}")))
            .collect()
    }
}

fn assert_names(value: Expanded<'_>, indexes: &Indexes) {
    assert_eq!(value.idents(), indexes.names());
}

#[test]
fn every_range_form_and_step() {
    assert_names(Expanded::new(generated::excl0()), &Indexes::through(9));
    assert_names(Expanded::new(generated::incl0()), &Indexes::through(10));
    assert_names(Expanded::new(generated::from0()), &Indexes::through(10));
    assert_names(Expanded::new(generated::to0()), &Indexes::through(9));
    assert_names(Expanded::new(generated::to_incl0()), &Indexes::through(10));
    assert_names(Expanded::new(generated::full0()), &Indexes::through(10));
    assert_names(
        Expanded::new(generated::step_excl0()),
        &Indexes::list(&[0, 2, 4, 6, 8]),
    );
    assert_names(
        Expanded::new(generated::step_incl0()),
        &Indexes::list(&[0, 2, 4, 6, 8]),
    );
    assert_names(
        Expanded::new(generated::step_full0()),
        &Indexes::list(&[0, 2, 4, 6, 8, 10]),
    );
    assert_names(
        Expanded::new(generated::step_from0()),
        &Indexes::list(&[0, 2, 4, 6, 8, 10]),
    );
    assert_names(
        Expanded::new(generated::step_to0()),
        &Indexes::list(&[0, 2, 4, 6, 8]),
    );
    assert_names(
        Expanded::new(generated::step_to_incl0()),
        &Indexes::list(&[0, 2, 4, 6, 8]),
    );
    assert_names(
        Expanded::new(generated::window0()),
        &Indexes::list(&[2, 3, 4, 5]),
    );
    assert_names(
        Expanded::new(generated::from_one_step0()),
        &Indexes::list(&[1, 3, 5, 7, 9]),
    );
    assert_names(Expanded::new(generated::last0()), &Indexes::list(&[10]));
}

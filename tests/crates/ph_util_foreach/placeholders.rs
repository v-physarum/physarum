#![allow(missing_docs, clippy::must_use_candidate)]

use ph_util_foreach::impl_foreach;

impl_foreach!(range = 0..3, body = {
    #[allow(unused_parens)]
    pub fn at + #() -> i32 {
        let values = [10, 20, 30];
        values[(#)]
    }
});

impl_foreach!(range = -1..0, body = {
    pub fn neg() -> i128 {
        #
    }
});

impl_foreach!(
    range = 0..1,
    body = {
        pub fn add(left: i32, right: i32) -> i32 {
            left + right
        }
    }
);

impl_foreach!(range = 0..2, body = {
    pub struct T + #;
});

impl_foreach!(range = 0..2, body = {
    pub fn take + #(_: ((T + #, T + #)[0..2])) {}
});

#[test]
fn hash_inside_ordinary_brackets_is_the_index_literal() {
    assert_eq!(at0(), 10);
    assert_eq!(at1(), 20);
    assert_eq!(at2(), 30);
}

#[test]
fn plus_between_ordinary_expressions_is_addition() {
    assert_eq!(add(2, 3), 5);
}

#[test]
fn tuple_rows_repeat_the_row_template() {
    take0(((T0, T0), (T1, T1)));
    take1(((T0, T0), (T1, T1)));
}

#[test]
fn negative_index_is_an_integer_literal() {
    assert_eq!(neg(), -1);
}

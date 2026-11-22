#![allow(missing_docs, non_upper_case_globals, clippy::must_use_candidate)]

use ph_util_foreach::impl_foreach;

fn takes_index(index: i128) -> i128 {
    index
}

impl_foreach!(
    range = 0..4,
    body = {
        $id = Ix #;
        $n = #;
        pub const $id: i128 = $n;
        pub fn passed + #() -> i128 {
            takes_index($id) + takes_index($n)
        }
    }
);

impl_foreach!(range = -5..-4, body = {
    $n = #;
    pub fn neg_index() -> i128 {
        takes_index($n)
    }
});

impl_foreach!(range = 0..2, body = {
    $id = Qx + #;
    pub const $id: i128 = #;
});

impl_foreach!(range = 0..2, body = {
    $id = Rx #;
    pub const $id: i128 = #;
});

impl_foreach!(range = 0..6::2, body = {
    $id = Sx #;
    pub const $id: i128 = #;
});

impl_foreach!(0..2 {
    $id = Zx #;
    pub const $id: i128 = #;
});

impl_foreach!(range = 0..1, body = {
    $id = Ax #;
    pub mod hide {
        $id = Bx #;
        pub const $id: i128 = 1;
    }
    pub const $id: i128 = 2;
});

impl_foreach!(range = 0..1, body = {
    $id = Cx #;
    pub const $id: i128 = 4;
    pub fn seen() -> i128 {
        let inner = {
            $id = Dy #;
            $n = #;
            $n
        };
        inner + $id
    }
});

impl_foreach!(range = 0..1, body = {
    $r#type = Vx #;
    pub const $type: i128 = 5;
});

impl_foreach!(range = 0..3, body = {
    $n = #;
    pub fn captured + #() -> i128 {
        let values = [($n)[0..2]];
        takes_index(values[0]) + takes_index(values[1])
    }
});

impl_foreach!(range = 0..3, body = {
    $id = Fx #;
    $n = #;
    pub mod fan_bind + # {
        pub const INDEX: i128 = $n;
        pub const TEXT: &str = stringify!(Name[..=#]);
    }
    pub const $id: i128 = fan_bind + #::INDEX;
});

impl_foreach!(
    range = 0..1,
    body = {
        macro_rules! echo_ident {
            ($name:ident) => {
                stringify!($name)
            };
        }
        pub fn echoed() -> &'static str {
            echo_ident!(kept)
        }
    }
);

#[test]
fn binding_is_a_function_argument() {
    assert_eq!(passed0(), 0);
    assert_eq!(passed1(), 2);
    assert_eq!(passed2(), 4);
    assert_eq!(passed3(), 6);
    assert_eq!((Ix0, Ix1, Ix2, Ix3), (0, 1, 2, 3));
}

#[test]
fn literal_binding_keeps_a_negative_index() {
    assert_eq!(neg_index(), -5);
}

#[test]
fn paste_forms_bind_the_same_identifier() {
    assert_eq!((Qx0, Qx1), (0, 1));
    assert_eq!((Rx0, Rx1), (0, 1));
    assert_eq!((Sx0, Sx2, Sx4), (0, 2, 4));
    assert_eq!((Zx0, Zx1), (0, 1));
}

#[test]
fn binding_ends_at_the_group_and_is_visible_inside_it() {
    assert_eq!(hide::Bx0, 1);
    assert_eq!(Ax0, 2);
    assert_eq!(seen(), 4);
    assert_eq!(Cx0, 4);
    assert_eq!(Vx0, 5);
}

#[test]
fn binding_keeps_the_index_from_the_binding_site() {
    assert_eq!(captured0(), 0);
    assert_eq!(captured1(), 2);
    assert_eq!(captured2(), 4);
}

#[test]
fn binding_survives_fan_mode() {
    assert_eq!(fan_bind0::INDEX, 0);
    assert_eq!(fan_bind1::INDEX, 1);
    assert_eq!(fan_bind2::INDEX, 2);
    assert_eq!((Fx0, Fx1, Fx2), (0, 1, 2));
    assert_eq!(fan_bind0::TEXT, "Name0");
    assert_eq!(fan_bind2::TEXT, "Name0, Name1, Name2");
}

#[test]
fn unbound_dollar_stays_in_a_nested_macro() {
    assert_eq!(echoed(), "kept");
}

#![allow(missing_docs, dead_code, clippy::must_use_candidate)]

use ph_util_foreach::impl_foreach;

fn words(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for character in input.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            current.push(character);
        } else {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            if !character.is_whitespace() {
                out.push(character.to_string());
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn assert_rows(rows: &[&str], expected: &str) {
    let actual = rows.join(" ");
    assert_eq!(words(&actual), words(expected), "actual: {actual}");
}

impl_foreach!(0..=3 {
    pub struct Incl + #;
});

impl_foreach!(1..10::2 {
    pub struct Odd + #;
});

impl_foreach!(2..=5 {
    pub struct Win + #;
});

impl_foreach!(3..=3 {
    pub struct One + #;
});

impl_foreach!(10..=12::2 {
    pub struct Ten + #;
});

impl_foreach!(0..1::2 {
    pub struct Half + #;
});

#[allow(unused_macros)]
macro_rules! fail_if_emitted {
    () => {
        compile_error!("empty range emitted a copy");
    };
}

impl_foreach!(5..5 {
    fail_if_emitted!();
});

impl_foreach!(-1..0 {
    pub const NEG: i128 = #;
});

impl_foreach!(0..1 {
    pub const ZERO: i128 = #;
    pub fn add(left: i32, right: i32) -> i32 {
        left + right
    }
});

impl_foreach!(0..2 {
    #[allow(dead_code)]
    pub struct Attr + #;
});

impl_foreach!(0..10 {
    pub mod excl + # {
        pub const TEXT: &str = stringify!(Name[0..10]);
    }
    pub mod from + # {
        pub const TEXT: &str = stringify!(Name[0..]);
    }
    pub mod to + # {
        pub const TEXT: &str = stringify!(Name[..10]);
    }
    pub mod full + # {
        pub const TEXT: &str = stringify!(Name[..]);
    }
    pub mod step_excl + # {
        pub const TEXT: &str = stringify!(Name[0..10::2]);
    }
    pub mod step_full + # {
        pub const TEXT: &str = stringify!(Name[..::2]);
    }
});

impl_foreach!(0..=10 {
    pub mod incl + # {
        pub const TEXT: &str = stringify!(Name[0..=10]);
    }
    pub mod to_incl + # {
        pub const TEXT: &str = stringify!(Name[..=10]);
    }
    pub mod from_step + # {
        pub const TEXT: &str = stringify!(Name[0..::2]);
    }
    pub mod to_step + # {
        pub const TEXT: &str = stringify!(Name[..10::2]);
    }
    pub mod window + # {
        pub const TEXT: &str = stringify!(Name[2..=5]);
    }
    pub mod from_one + # {
        pub const TEXT: &str = stringify!(Name[1..::2]);
    }
    pub mod last + # {
        pub const TEXT: &str = stringify!(Name[-1..=-1]);
    }
    pub mod through_last + # {
        pub const TEXT: &str = stringify!(Name[0..=-1]);
    }
});

impl_foreach!(0..=9 {
    pub mod step_incl + # {
        pub const TEXT: &str = stringify!(Name[0..=9::2]);
    }
});

impl_foreach!(0..=4 {
    pub mod last_only + # {
        pub const TEXT: &str = stringify!(Name[-1..=-1]);
    }
    pub mod last_tail + # {
        pub const TEXT: &str = stringify!(Name[-1..]);
    }
});

impl_foreach!(0..3 {
    pub mod flat_full + # {
        pub const TEXT: &str = stringify!(X Name[..];);
    }
    pub mod flat_paste + # {
        pub const TEXT: &str = stringify!(Ix + # Name[..];);
    }
});

impl_foreach!(0..3 {
    pub mod fan_prefix + # {
        pub const TEXT: &str = stringify!(X Name[..]; Y Name[..=#];);
    }
    pub mod fan_short + # {
        pub const TEXT: &str = stringify!(K Name[#];);
    }
    pub mod fan_row + # {
        pub const TEXT: &str = stringify!(K (T + #)[..=#];);
    }
});

impl_foreach!(0..6::2 {
    pub mod fan_step + # {
        pub const TEXT: &str = stringify!(K Name[..=#];);
    }
});

impl_foreach!(0..6::2 {
    pub mod flat_step + # {
        pub const TEXT: &str = stringify!(K Name[..];);
    }
});

impl_foreach!(0..10::2 {
    pub mod gap_all + # {
        pub const TEXT: &str = stringify!(K Name[0..10];);
    }
    pub mod gap_mid + # {
        pub const TEXT: &str = stringify!(K Name[1..=7];);
    }
});

impl_foreach!(0..2 {
    pub mod off_seq + # {
        pub const TEXT: &str = stringify!(K Off[0..2];);
    }
    pub mod empty_plus + # {
        pub const TEXT: &str = stringify!(K Name[0..0] + Name[0..2];);
    }
    pub mod plus_literal + # {
        pub const TEXT: &str = stringify!(K Name[0..2] + 1;);
    }
    pub mod paste_plus + # {
        pub const TEXT: &str = stringify!(let x = Ix + # + 1;);
    }
    pub mod index_brackets + # {
        pub const TEXT: &str = stringify!(let x = arr[(#)];);
    }
});

impl_foreach!(0..2 {
    pub mod off_bare + # {
        pub const TEXT: &str = stringify!(K Off[#];);
    }
    pub mod hole + # {
        pub const TEXT: &str = stringify!(K Name[#+1..=#];);
    }
});

impl_foreach!(0..3 {
    pub mod rows + # {
        pub const TEXT: &str = stringify!(K (T + #, U + #)[0..3];);
    }
});

impl_foreach!(1..3 {
    pub mod offset + # {
        pub const TEXT: &str = stringify!(K Name[#-1..=#];);
    }
    pub mod step_hash + # {
        pub const TEXT: &str = stringify!(K Name[0..=#::#+1];);
    }
});

impl_foreach!(0..4 {
    pub mod hole_edge + # {
        pub const TEXT: &str = stringify!(K Name[4..=3];);
    }
});

impl_foreach!(0..1 {
    pub mod ordinary + # {
        pub const TEXT: &str = stringify!(let x = values[i]; let y = Name[];);
    }
    pub mod attribute + # {
        pub const TEXT: &str = stringify!(#[allow(dead_code)] struct Ix + #;);
    }
});

#[test]
fn outer_ranges_paste_the_selected_indexes() {
    let _ = (Incl0, Incl1, Incl2, Incl3);
    let _ = (Odd1, Odd3, Odd5, Odd7, Odd9);
    let _ = (Win2, Win3, Win4, Win5);
    let _ = One3;
    let _ = (Ten10, Ten12);
    let _ = Half0;
    assert_eq!(NEG, -1);
    assert_eq!(ZERO, 0);
    assert_eq!(add(2, 3), 5);
    let _ = (Attr0, Attr1);
}

#[test]
fn pattern_forms_match_the_clipped_domain() {
    let names: [&str; 10] = [
        "Name0", "Name1", "Name2", "Name3", "Name4", "Name5", "Name6", "Name7", "Name8", "Name9",
    ];
    assert_rows(&[excl0::TEXT], &names.join(", "));
    assert_rows(
        &[incl0::TEXT],
        "Name0, Name1, Name2, Name3, Name4, Name5, Name6, Name7, Name8, Name9, Name10",
    );
    assert_rows(&[from0::TEXT], &names.join(", "));
    assert_rows(&[to0::TEXT], &names.join(", "));
    assert_rows(
        &[to_incl0::TEXT],
        "Name0, Name1, Name2, Name3, Name4, Name5, Name6, Name7, Name8, Name9, Name10",
    );
    assert_rows(&[full0::TEXT], &names.join(", "));
    assert_rows(&[step_excl0::TEXT], "Name0, Name2, Name4, Name6, Name8");
    assert_rows(&[step_incl0::TEXT], "Name0, Name2, Name4, Name6, Name8");
    assert_rows(&[step_full0::TEXT], "Name0, Name2, Name4, Name6, Name8");
    assert_rows(
        &[from_step0::TEXT],
        "Name0, Name2, Name4, Name6, Name8, Name10",
    );
    assert_rows(&[to_step0::TEXT], "Name0, Name2, Name4, Name6, Name8");
    assert_rows(&[window0::TEXT], "Name2, Name3, Name4, Name5");
    assert_rows(&[from_one0::TEXT], "Name1, Name3, Name5, Name7, Name9");
    assert_rows(&[last0::TEXT], "Name10");
    assert_rows(
        &[through_last0::TEXT],
        "Name0, Name1, Name2, Name3, Name4, Name5, Name6, Name7, Name8, Name9, Name10",
    );
    assert_rows(&[last_only0::TEXT], "Name4");
    assert_rows(&[last_tail0::TEXT], "Name4");
    let _ = excl9::TEXT;
    let _ = incl10::TEXT;
}

#[test]
fn flat_domain_repeats_the_full_outer_set() {
    let full = "X Name0, Name1, Name2;";
    assert_rows(
        &[flat_full0::TEXT, flat_full1::TEXT, flat_full2::TEXT],
        &format!("{full} {full} {full}"),
    );
    assert_rows(
        &[flat_paste0::TEXT, flat_paste1::TEXT, flat_paste2::TEXT],
        "Ix0 Name0, Name1, Name2; Ix1 Name0, Name1, Name2; Ix2 Name0, Name1, Name2;",
    );
}

#[test]
fn fan_domain_is_the_prefix_through_the_outer_index() {
    assert_rows(
        &[fan_prefix0::TEXT, fan_prefix1::TEXT, fan_prefix2::TEXT],
        "X Name0; Y Name0; X Name0, Name1; Y Name0, Name1; X Name0, Name1, Name2; Y Name0, Name1, Name2;",
    );
    assert_rows(
        &[fan_short0::TEXT, fan_short1::TEXT, fan_short2::TEXT],
        "K Name0; K Name0, Name1; K Name0, Name1, Name2;",
    );
    assert_rows(
        &[fan_step0::TEXT, fan_step2::TEXT, fan_step4::TEXT],
        "K Name0; K Name0, Name1, Name2; K Name0, Name1, Name2, Name3, Name4;",
    );
    let evens = "K Name0, Name2, Name4;";
    assert_rows(
        &[flat_step0::TEXT, flat_step2::TEXT, flat_step4::TEXT],
        &format!("{evens} {evens} {evens}"),
    );
}

#[test]
fn gapped_domain_keeps_only_outer_indexes() {
    let evens = "K Name0, Name2, Name4, Name6, Name8;";
    assert_rows(
        &[
            gap_all0::TEXT,
            gap_all2::TEXT,
            gap_all4::TEXT,
            gap_all6::TEXT,
            gap_all8::TEXT,
        ],
        &format!("{evens} {evens} {evens} {evens} {evens}"),
    );
    let middle = "K Name2, Name4, Name6;";
    assert_rows(
        &[
            gap_mid0::TEXT,
            gap_mid2::TEXT,
            gap_mid4::TEXT,
            gap_mid6::TEXT,
            gap_mid8::TEXT,
        ],
        &format!("{middle} {middle} {middle} {middle} {middle}"),
    );
}

#[test]
fn placeholders_concat_tuples_and_offsets() {
    assert_rows(&[off_bare0::TEXT, off_bare1::TEXT], "K Off; K Off;");
    assert_rows(
        &[off_seq0::TEXT, off_seq1::TEXT],
        "K Off0, Off1; K Off0, Off1;",
    );
    assert_rows(
        &[empty_plus0::TEXT, empty_plus1::TEXT],
        "K Name0, Name1; K Name0, Name1;",
    );
    assert_rows(
        &[plus_literal0::TEXT, plus_literal1::TEXT],
        "K Name0, Name1 + 1; K Name0, Name1 + 1;",
    );
    let row = "K (T0, U0), (T1, U1), (T2, U2);";
    assert_rows(
        &[rows0::TEXT, rows1::TEXT, rows2::TEXT],
        &format!("{row} {row} {row}"),
    );
    assert_rows(
        &[fan_row0::TEXT, fan_row1::TEXT, fan_row2::TEXT],
        "K (T0); K (T0), (T1); K (T0), (T1), (T2);",
    );
    assert_rows(
        &[offset1::TEXT, offset2::TEXT],
        "K Name0, Name1; K Name1, Name2;",
    );
    assert_rows(&[step_hash1::TEXT, step_hash2::TEXT], "K Name0; K Name0;");
    assert_rows(&[hole0::TEXT, hole1::TEXT], "K ; K ;");
    assert_rows(
        &[
            hole_edge0::TEXT,
            hole_edge1::TEXT,
            hole_edge2::TEXT,
            hole_edge3::TEXT,
        ],
        "K ; K ; K ; K ;",
    );
}

#[test]
fn paste_and_ordinary_tokens() {
    assert_rows(
        &[paste_plus0::TEXT, paste_plus1::TEXT],
        "let x = Ix0 + 1; let x = Ix1 + 1;",
    );
    assert_rows(
        &[index_brackets0::TEXT, index_brackets1::TEXT],
        "let x = arr[(0)]; let x = arr[(1)];",
    );
    assert_rows(&[ordinary0::TEXT], "let x = values[i]; let y = Name[];");
    assert_rows(&[attribute0::TEXT], "#[allow(dead_code)] struct Ix0;");
}

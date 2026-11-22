#![allow(missing_docs, dead_code, clippy::must_use_candidate)]

use ph_util_foreach::impl_foreach;

/// One token of expanded text: an identifier run or a single punctuation mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Word<'a>(&'a str);

impl<'a> Word<'a> {
    fn new(text: &'a str) -> Self {
        Self(text)
    }
}

/// Words parsed from expanded text. The source string is not kept.
struct Text<'a>(Vec<Word<'a>>);

impl<'a> Text<'a> {
    fn new(text: &'a str) -> Self {
        Self(Self::split(text))
    }

    fn words(&self) -> Vec<&Word<'a>> {
        self.0.iter().collect()
    }

    fn split(text: &'a str) -> Vec<Word<'a>> {
        let mut out = Vec::new();
        let mut start = None;
        for (index, character) in text.char_indices() {
            if character.is_ascii_alphanumeric() || character == '_' {
                if start.is_none() {
                    start = Some(index);
                }
            } else {
                if let Some(from) = start.take() {
                    out.push(Word::new(&text[from..index]));
                }
                if !character.is_whitespace() {
                    let end = index + character.len_utf8();
                    out.push(Word::new(&text[index..end]));
                }
            }
        }
        if let Some(from) = start {
            out.push(Word::new(&text[from..]));
        }
        out
    }
}

fn assert_rows(rows: &[Text<'_>], expected: &Text<'_>) {
    let actual = rows.iter().flat_map(Text::words).collect::<Vec<_>>();
    assert_eq!(actual, expected.words());
}

impl_foreach!(n in 0..=3 {
    pub struct paste(Incl, n);
});

impl_foreach!(n in 0x10..0x12 {
    pub struct paste(Hex, n);
});

impl_foreach!(n in 0o10..0o12 {
    pub struct paste(Oct, n);
});

impl_foreach!(n in 0b1000..0b1010 {
    pub struct paste(Bin, n);
});

impl_foreach!(n in 0x1_0..0x1_2 {
    pub struct paste(HexSep, n);
});

impl_foreach!(n in (1..10).step_by(2) {
    pub struct paste(Odd, n);
});

impl_foreach!(n in 2..=5 {
    pub struct paste(Win, n);
});

impl_foreach!(n in 3..=3 {
    pub struct paste(One, n);
});

impl_foreach!(n in (10..=12).step_by(2) {
    pub struct paste(Ten, n);
});

impl_foreach!(n in (0..1).step_by(2) {
    pub struct paste(Half, n);
});

#[allow(unused_macros)]
macro_rules! fail_if_emitted {
    () => {
        compile_error!("empty range emitted a copy");
    };
}

impl_foreach!(n in 5..5 {
    fail_if_emitted!();
});

impl_foreach!(n in -1..0 {
    pub const NEG: i128 = n;
});

impl_foreach!(n in 0..1 {
    pub const ZERO: i128 = n;
    pub fn add(left: i32, right: i32) -> i32 {
        left + right
    }
});

impl_foreach!(n in 0..2 {
    #[allow(dead_code)]
    pub struct paste(Attr, n);
});

impl_foreach!(n in 0..10 {
    pub mod paste(excl, n) {
        pub const TEXT: &str = stringify!(Name[0..10]);
    }
    pub mod paste(from, n) {
        pub const TEXT: &str = stringify!(Name[0..]);
    }
    pub mod paste(to, n) {
        pub const TEXT: &str = stringify!(Name[..10]);
    }
    pub mod paste(full, n) {
        pub const TEXT: &str = stringify!(Name[..]);
    }
    pub mod paste(step_excl, n) {
        pub const TEXT: &str = stringify!(Name[(0..10).step_by(2)]);
    }
    pub mod paste(step_full, n) {
        pub const TEXT: &str = stringify!(Name[(..).step_by(2)]);
    }
});

impl_foreach!(n in 0..=10 {
    pub mod paste(incl, n) {
        pub const TEXT: &str = stringify!(Name[0..=10]);
    }
    pub mod paste(to_incl, n) {
        pub const TEXT: &str = stringify!(Name[..=10]);
    }
    pub mod paste(from_step, n) {
        pub const TEXT: &str = stringify!(Name[(0..).step_by(2)]);
    }
    pub mod paste(to_step, n) {
        pub const TEXT: &str = stringify!(Name[(..10).step_by(2)]);
    }
    pub mod paste(window, n) {
        pub const TEXT: &str = stringify!(Name[2..=5]);
    }
    pub mod paste(from_one, n) {
        pub const TEXT: &str = stringify!(Name[(1..).step_by(2)]);
    }
    pub mod paste(last, n) {
        pub const TEXT: &str = stringify!(Name[end..=end]);
    }
    pub mod paste(through_last, n) {
        pub const TEXT: &str = stringify!(Name[0..=end]);
    }
});

impl_foreach!(n in 0..=9 {
    pub mod paste(step_incl, n) {
        pub const TEXT: &str = stringify!(Name[(0..=9).step_by(2)]);
    }
});

impl_foreach!(n in 0..=4 {
    pub mod paste(last_only, n) {
        pub const TEXT: &str = stringify!(Name[end..=end]);
    }
    pub mod paste(last_tail, n) {
        pub const TEXT: &str = stringify!(Name[end..]);
    }
});

impl_foreach!(n in 0..3 {
    pub mod paste(flat_full, n) {
        pub const TEXT: &str = stringify!(X + Name[..];);
    }
    pub mod paste(flat_paste, n) {
        pub const TEXT: &str = stringify!(paste(Ix, n) Name[..];);
    }
});

impl_foreach!(n in 0..3 {
    pub mod paste(fan_prefix, n) {
        pub const TEXT: &str = stringify!(X + Name[..]; Y + Name[..=n];);
    }
    pub mod paste(fan_short, n) {
        pub const TEXT: &str = stringify!(K + Name[n];);
    }
    pub mod paste(fan_row, n) {
        pub const TEXT: &str = stringify!(K + (paste(T, n))[..=n];);
    }
});

impl_foreach!(n in (0..6).step_by(2) {
    pub mod paste(fan_step, n) {
        pub const TEXT: &str = stringify!(K + Name[..=n];);
    }
});

impl_foreach!(n in (0..6).step_by(2) {
    pub mod paste(flat_step, n) {
        pub const TEXT: &str = stringify!(K + Name[..];);
    }
});

impl_foreach!(n in (0..10).step_by(2) {
    pub mod paste(gap_all, n) {
        pub const TEXT: &str = stringify!(K + Name[0..10];);
    }
    pub mod paste(gap_mid, n) {
        pub const TEXT: &str = stringify!(K + Name[1..=7];);
    }
});

impl_foreach!(n in 0..2 {
    pub mod paste(off_seq, n) {
        pub const TEXT: &str = stringify!(K + Off[0..2];);
    }
    pub mod paste(empty_plus, n) {
        pub const TEXT: &str = stringify!(K + Name[0..0] + Name[0..2];);
    }
    pub mod paste(plus_literal, n) {
        pub const TEXT: &str = stringify!(K + Name[0..2] + 1;);
    }
    pub mod paste(paste_plus, n) {
        pub const TEXT: &str = stringify!(let x = paste(Ix, n) + 1;);
    }
    pub mod paste(index_brackets, n) {
        pub const TEXT: &str = stringify!(let x = arr[(n)];);
    }
});

impl_foreach!(n in 0..2 {
    pub mod paste(off_bare, n) {
        pub const TEXT: &str = stringify!(K + Off[n];);
    }
    pub mod paste(hole, n) {
        pub const TEXT: &str = stringify!(K + Name[n + 1..=n];);
    }
});

impl_foreach!(n in 0..3 {
    pub mod paste(rows, n) {
        pub const TEXT: &str = stringify!(K + (paste(T, n), paste(U, n))[0..3];);
    }
});

impl_foreach!(n in 1..3 {
    pub mod paste(offset, n) {
        pub const TEXT: &str = stringify!(K + Name[n - 1..=n];);
    }
    pub mod paste(step_hash, n) {
        pub const TEXT: &str = stringify!(K + Name[(0..=n).step_by(n + 1)];);
    }
});

impl_foreach!(n in 0..4 {
    pub mod paste(hole_edge, n) {
        pub const TEXT: &str = stringify!(K + Name[4..=3];);
    }
});

impl_foreach!(n in 0..1 {
    pub mod paste(ordinary, n) {
        pub const TEXT: &str = stringify!(let x = values[i]; let y = Name[];);
    }
    pub mod paste(attribute, n) {
        pub const TEXT: &str = stringify!(#[allow(dead_code)] struct paste(Ix, n););
    }
});

#[test]
fn outer_ranges_paste_the_selected_indexes() {
    let _ = (Incl0, Incl1, Incl2, Incl3);
    let _ = (Hex16, Hex17);
    let _ = (Oct8, Oct9, Bin8, Bin9, HexSep16, HexSep17);
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
    assert_rows(&[Text::new(excl0::TEXT)], &Text::new(&names.join(", ")));
    assert_rows(
        &[Text::new(incl0::TEXT)],
        &Text::new("Name0, Name1, Name2, Name3, Name4, Name5, Name6, Name7, Name8, Name9, Name10"),
    );
    assert_rows(&[Text::new(from0::TEXT)], &Text::new(&names.join(", ")));
    assert_rows(&[Text::new(to0::TEXT)], &Text::new(&names.join(", ")));
    assert_rows(
        &[Text::new(to_incl0::TEXT)],
        &Text::new("Name0, Name1, Name2, Name3, Name4, Name5, Name6, Name7, Name8, Name9, Name10"),
    );
    assert_rows(&[Text::new(full0::TEXT)], &Text::new(&names.join(", ")));
    assert_rows(
        &[Text::new(step_excl0::TEXT)],
        &Text::new("Name0, Name2, Name4, Name6, Name8"),
    );
    assert_rows(
        &[Text::new(step_incl0::TEXT)],
        &Text::new("Name0, Name2, Name4, Name6, Name8"),
    );
    assert_rows(
        &[Text::new(step_full0::TEXT)],
        &Text::new("Name0, Name2, Name4, Name6, Name8"),
    );
    assert_rows(
        &[Text::new(from_step0::TEXT)],
        &Text::new("Name0, Name2, Name4, Name6, Name8, Name10"),
    );
    assert_rows(
        &[Text::new(to_step0::TEXT)],
        &Text::new("Name0, Name2, Name4, Name6, Name8"),
    );
    assert_rows(
        &[Text::new(window0::TEXT)],
        &Text::new("Name2, Name3, Name4, Name5"),
    );
    assert_rows(
        &[Text::new(from_one0::TEXT)],
        &Text::new("Name1, Name3, Name5, Name7, Name9"),
    );
    assert_rows(&[Text::new(last0::TEXT)], &Text::new("Name10"));
    assert_rows(
        &[Text::new(through_last0::TEXT)],
        &Text::new("Name0, Name1, Name2, Name3, Name4, Name5, Name6, Name7, Name8, Name9, Name10"),
    );
    assert_rows(&[Text::new(last_only0::TEXT)], &Text::new("Name4"));
    assert_rows(&[Text::new(last_tail0::TEXT)], &Text::new("Name4"));
    let _ = excl9::TEXT;
    let _ = incl10::TEXT;
}

#[test]
fn flat_domain_repeats_the_full_outer_set() {
    let full = "X, Name0, Name1, Name2;";
    assert_rows(
        &[
            Text::new(flat_full0::TEXT),
            Text::new(flat_full1::TEXT),
            Text::new(flat_full2::TEXT),
        ],
        &Text::new(&format!("{full} {full} {full}")),
    );
    assert_rows(
        &[
            Text::new(flat_paste0::TEXT),
            Text::new(flat_paste1::TEXT),
            Text::new(flat_paste2::TEXT),
        ],
        &Text::new("Ix0 Name0, Name1, Name2; Ix1 Name0, Name1, Name2; Ix2 Name0, Name1, Name2;"),
    );
}

#[test]
fn fan_domain_is_the_prefix_through_the_outer_index() {
    assert_rows(
        &[
            Text::new(fan_prefix0::TEXT),
            Text::new(fan_prefix1::TEXT),
            Text::new(fan_prefix2::TEXT),
        ],
        &Text::new(
            "X, Name0; Y, Name0; X, Name0, Name1; Y, Name0, Name1; X, Name0, Name1, Name2; Y, Name0, Name1, Name2;",
        ),
    );
    assert_rows(
        &[
            Text::new(fan_short0::TEXT),
            Text::new(fan_short1::TEXT),
            Text::new(fan_short2::TEXT),
        ],
        &Text::new("K, Name0; K, Name0, Name1; K, Name0, Name1, Name2;"),
    );
    assert_rows(
        &[
            Text::new(fan_step0::TEXT),
            Text::new(fan_step2::TEXT),
            Text::new(fan_step4::TEXT),
        ],
        &Text::new("K, Name0; K, Name0, Name1, Name2; K, Name0, Name1, Name2, Name3, Name4;"),
    );
    let evens = "K, Name0, Name2, Name4;";
    assert_rows(
        &[
            Text::new(flat_step0::TEXT),
            Text::new(flat_step2::TEXT),
            Text::new(flat_step4::TEXT),
        ],
        &Text::new(&format!("{evens} {evens} {evens}")),
    );
}

#[test]
fn gapped_domain_keeps_only_outer_indexes() {
    let evens = "K, Name0, Name2, Name4, Name6, Name8;";
    assert_rows(
        &[
            Text::new(gap_all0::TEXT),
            Text::new(gap_all2::TEXT),
            Text::new(gap_all4::TEXT),
            Text::new(gap_all6::TEXT),
            Text::new(gap_all8::TEXT),
        ],
        &Text::new(&format!("{evens} {evens} {evens} {evens} {evens}")),
    );
    let middle = "K, Name2, Name4, Name6;";
    assert_rows(
        &[
            Text::new(gap_mid0::TEXT),
            Text::new(gap_mid2::TEXT),
            Text::new(gap_mid4::TEXT),
            Text::new(gap_mid6::TEXT),
            Text::new(gap_mid8::TEXT),
        ],
        &Text::new(&format!("{middle} {middle} {middle} {middle} {middle}")),
    );
}

#[test]
fn placeholders_concat_tuples_and_offsets() {
    assert_rows(
        &[Text::new(off_bare0::TEXT), Text::new(off_bare1::TEXT)],
        &Text::new("K, Off; K, Off;"),
    );
    assert_rows(
        &[Text::new(off_seq0::TEXT), Text::new(off_seq1::TEXT)],
        &Text::new("K, Off0, Off1; K, Off0, Off1;"),
    );
    assert_rows(
        &[Text::new(empty_plus0::TEXT), Text::new(empty_plus1::TEXT)],
        &Text::new("K, Name0, Name1; K, Name0, Name1;"),
    );
    assert_rows(
        &[
            Text::new(plus_literal0::TEXT),
            Text::new(plus_literal1::TEXT),
        ],
        &Text::new("K, Name0, Name1 + 1; K, Name0, Name1 + 1;"),
    );
    let row = "K, (T0, U0), (T1, U1), (T2, U2);";
    assert_rows(
        &[
            Text::new(rows0::TEXT),
            Text::new(rows1::TEXT),
            Text::new(rows2::TEXT),
        ],
        &Text::new(&format!("{row} {row} {row}")),
    );
    assert_rows(
        &[
            Text::new(fan_row0::TEXT),
            Text::new(fan_row1::TEXT),
            Text::new(fan_row2::TEXT),
        ],
        &Text::new("K, (T0); K, (T0), (T1); K, (T0), (T1), (T2);"),
    );
    assert_rows(
        &[Text::new(offset1::TEXT), Text::new(offset2::TEXT)],
        &Text::new("K, Name0, Name1; K, Name1, Name2;"),
    );
    assert_rows(
        &[Text::new(step_hash1::TEXT), Text::new(step_hash2::TEXT)],
        &Text::new("K, Name0; K, Name0;"),
    );
    assert_rows(
        &[Text::new(hole0::TEXT), Text::new(hole1::TEXT)],
        &Text::new("K ; K ;"),
    );
    assert_rows(
        &[
            Text::new(hole_edge0::TEXT),
            Text::new(hole_edge1::TEXT),
            Text::new(hole_edge2::TEXT),
            Text::new(hole_edge3::TEXT),
        ],
        &Text::new("K ; K ; K ; K ;"),
    );
}

#[test]
fn paste_and_ordinary_tokens() {
    assert_rows(
        &[Text::new(paste_plus0::TEXT), Text::new(paste_plus1::TEXT)],
        &Text::new("let x = Ix0 + 1; let x = Ix1 + 1;"),
    );
    assert_rows(
        &[
            Text::new(index_brackets0::TEXT),
            Text::new(index_brackets1::TEXT),
        ],
        &Text::new("let x = arr[(0)]; let x = arr[(1)];"),
    );
    assert_rows(
        &[Text::new(ordinary0::TEXT)],
        &Text::new("let x = values[i]; let y = Name[];"),
    );
    assert_rows(
        &[Text::new(attribute0::TEXT)],
        &Text::new("#[allow(dead_code)] struct Ix0;"),
    );
}

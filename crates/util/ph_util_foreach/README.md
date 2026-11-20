# ph_util_foreach

`impl_foreach` copies a token-tree body once for each index of a finite range. The copy can build structs, impls, and the invocations of other macros. This crate does not define those other macros.

The macro is implemented on the stable `proc_macro` API and has no crate dependencies.

## Invocation

```rust
use ph_util_foreach::impl_foreach;

impl_foreach!(0..4, {
    pub struct Ix + #;
});
```

The call is `impl_foreach!(range, { body })`. The body is an arbitrary token tree. A semicolon ends the invocation when it is an item. Commas inside the body separate fields of a nested macro.

`0..4` above expands to:

```rust
pub struct Ix0;
pub struct Ix1;
pub struct Ix2;
pub struct Ix3;
```

## Ranges

Outer ranges and patterns inside `[]` use the same forms. Bounds are integer literals or `#` expressions. Nothing is hard-coded to a sample such as `0..4`.

| Syntax | Rust range | Indices |
| --- | --- | --- |
| `A..B` | `Range` | `A` inclusive, `B` exclusive |
| `A..=B` | `RangeInclusive` | `A` and `B` inclusive |
| `A..` | `RangeFrom` | from `A` through the end of the domain |
| `..B` | `RangeTo` | from the start of the domain up to `B`, exclusive |
| `..=B` | `RangeToInclusive` | from the start of the domain through `B`, inclusive |
| `..` | `RangeFull` | the whole domain |
| `<range>::S` | step | the same range, every `S`-th index |

`::S` applies to every form, including bounds that use `#`: `0..#::2`, `..=#::2`, `0..10::2`, `..::2`.

The step counts in arithmetic from the start of the clipped range. `0..10::2` is `0, 2, 4, 6, 8`. `0..=9::2` is the same list. `S` is a positive integer literal, or `#±N` when that value is a positive compile-time constant (`::2`, `::#`, `::#+1`). `S <= 0` is a compile error, whether `S` is written as `0`, a negative literal, or a `#±N` expression that evaluates to `0` or below.

An open end is clipped to the current domain, and then `::S` is applied.

### Outer range

`impl_foreach!(<Range>, { /* body */ })` uses the forms in the table. The set has to be finite with no parent domain:

| Outer range | Result |
| --- | --- |
| `A..B` | allowed, for example `impl_foreach!(0..4, { /* body */ })` |
| `A..=B` | allowed, for example `impl_foreach!(0..=3, { /* body */ })` |
| `A..B::S` and `A..=B::S` | allowed, for example `impl_foreach!(0..10::2, { /* body */ })` |
| `A..`, `..B`, `..=B`, `..` | rejected, including when written with `::S` |

Inside `Name[<pat>]`, open forms stay valid. The step domain closes them.

### Domain

- Flat mode: the index set of the outer range after its `::S` step.
- Fan mode: for each outer index, the prefix `0..=outer` (every integer, including indexes the outer step skipped).

A bound outside that domain is a compile error. An empty slice on the hole edge is allowed: `(last + 1)..=last`. That is the window `T[#+1..=#]` produces when `#` is the last index of the domain.

An inclusive bound must lie in `first..=last`. An exclusive end may be at most one outer step past the last index, so `Name[0..10]` is valid on the domain produced by `0..10` or `0..10::2`.

On a stepped flat domain the range is generated arithmetically and then intersected with the index set. `Name[1..=7]` on `0..10::2` is `Name2, Name4, Name6`.

### Bounds in a pattern

A pattern bound is one of:

- an integer literal (`0`, `10`, `-2`);
- `#`, the current index;
- `#±N`, written `#+1` or `#-1`;
- the literal `-1`, which means the last index of the domain.

`-1` is that sentinel only when it is written as the pattern bound. `#-1` is arithmetic on the current index. In an outer range, `-1` is the integer negative one, because there is no domain yet.

Exactly one pattern is allowed in one pair of brackets. `A[0..#, Off]` is a compile error.

## Placeholders

| Form | Expansion |
| --- | --- |
| `#` | the current index as an integer literal |
| `Name + #` | the identifier `Name` with the decimal digits of `#` appended |
| `Name[#]` | `Name[0..=#]` |
| `Name[<pat>]` | `Name` plus those digits for each index in `<pat>`, separated by commas |
| `Off[#]` | the identifier `Off`, with no digits |
| `X + Y` | the two expansions concatenated; an empty piece adds no comma |
| `(A, B, …)[<pat>]` | one parenthesized row per index, separated by commas |

`Name + #` is the only identifier paste. `Ix + #` expands to `Ix0`, `Ix1`, and so on. The index must be non-negative; pasting a negative index is a compile error. An identifier immediately followed by `#` (`Ix #`) is a compile error.

`X + Y` concatenates sequence expansions such as `Name[<pat>]` and `(…)[<pat>]`. A bare identifier is a one-element piece, so `X + Name[..]` expands to `X, Name0, Name1, ...`. Writing the pieces next to each other (`X Name[..]`) is a compile error. `Ix + #` is paste: the left side is a bare identifier, the right side is `#`, and the expansion is one identifier.

Inside a tuple row, `#` is the row index. `(T + #, U + #)[0..3]` expands to `(T0, U0), (T1, U1), (T2, U2)`.

`#[...]` and `#![...]` stay attributes. Brackets that do not open with a range stay as written, and `#` inside them is still the current index, so `arr[(#)]` becomes `arr[(0)]`. An identifier followed by a real range is the placeholder, so `Name[0..2]` is `Name0, Name1`.

`Off[0..2]` is the ordinary sequence `Off0, Off1`. Only `Off[#]` is the bare hole marker.

## Flat and fan

Flat mode emits one copy per outer index. `#` is that index. The domain is the full outer set. `pub struct Ix + #;` is flat.

```rust
impl_foreach!(0..10::2, {
    pub struct Ix + #;
});
```

expands to `Ix0`, `Ix2`, `Ix4`, `Ix6`, and `Ix8`.

Fan mode is used when the body contains a prefix or hole form: `Name[#]`, `Off[#]`, or any range bound that mentions `#` (`T[..=#]`, `T[..#]`, `T[#+1..=#]`, and the same shapes with other names). A step of `#` by itself stays in flat mode. Fan mode still emits one copy per outer index. `#` is that index, the hole is that index, and the domain is `0..=#`.

```rust
impl_foreach!(0..4, {
    impl_take!(
        args: (T[..=#]),
        ix: Ix + #,
        out: T + #,
        rest: T[..#] + Off[#] + T[#+1..=#],
    );
});
```

`impl_take!` is a local `macro_rules!` helper written by the caller. The four copies are:

| `#` | `args` | `ix` | `out` | `rest` |
| --- | --- | --- | --- | --- |
| 0 | `(T0)` | `Ix0` | `T0` | `Off` |
| 1 | `(T0, T1)` | `Ix1` | `T1` | `T0, Off` |
| 2 | `(T0, T1, T2)` | `Ix2` | `T2` | `T0, T1, Off` |
| 3 | `(T0, T1, T2, T3)` | `Ix3` | `T3` | `T0, T1, T2, Off` |

## Compile-time checks

The macro emits `compile_error!` when:

- the outer range is open;
- the step is less than or equal to zero;
- `A..B` or `A..=B` has a start after the end, other than an empty hole-edge slice;
- a bound lies outside the domain;
- the placeholder syntax cannot be parsed (`A[0..#, Off]`, a second range in the same brackets, `::` with no step, or an identifier immediately followed by `#`).

An empty hole-edge slice expands to no tokens.

## Build time

`scripts/measure-ph-util-foreach-build.sh` times a clean `cargo build -p ph_util_foreach --offline --locked --timings` in a fresh target directory, then a second build with no source changes. CI runs that script, writes the report to the job summary, and uploads it as the `ph-util-foreach-build-time` artifact.

## User compile time

`scripts/measure-ph-util-foreach-expand.sh` times the macro from a downstream crate. It compiles `ph_util_foreach` once, then for each case cleans only the user crate and runs `cargo build`. Incremental compilation is off, so each run expands the macro again. The report is the median of three dev builds. CI job `expand time` repeats this and uploads the `ph-util-foreach-expand-time` artifact.

| Case | User source |
| --- | --- |
| `flat_structs_N` | one call, `pub struct Ix + #` for `0..N` |
| `hand_structs_N` | the same `N` structs written out in the crate |
| `one_call_256` | one call that emits 256 structs |
| `many_calls_256` | 256 calls that each emit one struct |
| `fan_take_N` | the `impl_take!` fan pattern for `0..N` |
| `fan_rows_N` | one const per index, `stringify!` of the fan rows `(T + #, U + #)[0..=#]` |

## License

Licensed under the [MIT License](../../../LICENSE).

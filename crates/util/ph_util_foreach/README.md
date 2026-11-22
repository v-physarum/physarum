# ph_util_foreach

Repeats a token tree once for each index of a finite range. `syn` parses the range as an ordinary Rust expression. Evaluation walks `Range` and `RangeInclusive` with `step_by`.

## Invocation

```rust
impl_foreach!(n in 0..4 {
    pub struct paste(Ix, n);
});
```

The caller chooses the loop variable. The call is `impl_foreach!(n in <range> { ... })`. A comma before the block is optional: `impl_foreach!(n in 0..4, { ... })`. The body is an arbitrary token tree. A semicolon ends the invocation when it is an item. The loop variable cannot be named `end`.

## Ranges

`syn` parses the range. Parentheses around the range are required when the step is a method call, because `0..10.step_by(2)` is `0..(10.step_by(2))`.

| Form | Indexes |
| --- | --- |
| `0..4` | `0, 1, 2, 3` |
| `0..=3` | `0, 1, 2, 3` |
| `(0..10).step_by(2)` | `0, 2, 4, 6, 8` |
| `(0..=9).step_by(2)` | `0, 2, 4, 6, 8` |
| `(1..10).step_by(2)` | `1, 3, 5, 7, 9` |

The step counts from the start. It is a positive integer literal, the loop variable, or the loop variable plus or minus an integer (`2`, `n`, `n + 1`). A step that is not positive is a compile error. A step wider than `usize` is applied by arithmetic instead of `Iterator::step_by`.

`-1` is the integer negative one. `0x10`, `0o10`, and `0b1000` are ordinary integer literals. An uppercase prefix such as `0X10` is not an integer literal.

### Outer range

The outer set has to be finite:

| Form | |
| --- | --- |
| `n in 0..4` | allowed |
| `n in 0..=3` | allowed |
| `n in (0..10).step_by(2)` | allowed |
| `n in 0..`, `..4`, `..=4`, `..` | compile error |
| `n in (0..).step_by(2)` and the other open forms | compile error |

The loop variable and `end` cannot appear in the outer range.

### Domain

A `Name[...]` pattern is clipped to the domain. Flat mode uses the outer set. Fan mode uses `0..=index`. A bound outside that domain is a compile error. An empty slice on the hole edge is allowed: `n + 1..=n` when `n` is the last index of the domain.

### Bounds in a pattern

A pattern bound is an integer literal, the loop variable, `loop + integer` or `loop - integer`, or the path `end`. `end` is the last domain index, and only when it is the whole bound (`0..=end`, `end..=end`, `end..`). `-1` is negative one, so `Name[-1..=-1]` on a non-negative domain is outside the domain.

Exactly one pattern is allowed in one pair of brackets. `A[0..n, Off]` is a compile error. Brackets that are not a range, not `(range).step_by(...)`, and not exactly the loop variable stay as written.

## Placeholders

| Written | Expansion |
| --- | --- |
| `n` | the current index as an integer literal |
| `paste(Ix, n)` | the identifier `Ix` with the decimal digits of the index appended |
| `Name[0..=n]` | the identifiers `Name0` through `Namen` |
| `repeat(Name, 0..=n)` | the same identifiers |

`paste(Ix, n)` pastes a non-negative index. A negative index is a compile error. `paste` takes one identifier and the loop variable: `paste(Ix, n + 1)` is a compile error. A `paste` that is not a call is an ordinary identifier.

`repeat(Name, 0..2)` expands to `Name0, Name1`. A comma separates list pieces, and an empty `repeat` drops the comma beside it: `X, repeat(Name, ..)` expands to `X, Name0, Name1`. `repeat((paste(T, n), paste(U, n)), 0..3)` repeats the row. `X Name[0..2]` and `X + Name[0..2]` are compile errors. `paste(Ix, n) + 1` is the identifier `Ix0` followed by `+ 1`.

`Name[n]` is not a range. It stays a bracket with the index substituted, so at index 0 it is `Name[0]`. Write `Name[0..=n]` or `repeat(Name, 0..=n)`.

Inside a tuple row, the loop variable is the row index. `(paste(T, n), paste(U, n))[0..3]` expands to `(T0, U0), (T1, U1), (T2, U2)`.

`#[...]` and `#![...]` stay attributes. `arr[(n)]` becomes `arr[(0)]`. `Off[0..2]` and `repeat(Off, 0..2)` are `Off0, Off1`. A bare `Off` between repeats is the hole marker.

## Index bindings

| Form | Captures |
| --- | --- |
| `$name = paste(Ident, n);` | the pasted identifier |
| `$name = n;` | the index as an unsuffixed integer literal |

```rust
impl_foreach!(n in 0..4 {
    $id = paste(Ix, n);
    $n = n;
    pub const $id: i128 = $n;
});
```

Any other right-hand side is a compile error. The statement needs the trailing `;`. The captured value is the index at the binding. A later row does not rebind it. An unbound `$name` is copied through for a nested `macro_rules`.

## Flat and fan

Flat mode emits one copy per outer index. The domain is the outer set. `pub struct paste(Ix, n);` is flat. A step that mentions the loop variable stays flat when no pattern bound does.

Fan mode is used when a range bound mentions the loop variable, as in `Name[0..=n]` or `repeat(Name, ..=n)`. `end` does not select fan. Fan mode still emits one copy per outer index. The domain is `0..=n`.

```rust
impl_foreach!(n in 0..4 {
    impl_take!(
        args: (T[..=n]),
        ix: paste(Ix, n),
        out: paste(T, n),
        rest: repeat(T, ..n), Off, repeat(T, n + 1..=n),
    );
});
```

| `n` | `args` | `ix` | `out` | `rest` |
| --- | --- | --- | --- | --- |
| 0 | `(T0)` | `Ix0` | `T0` | `Off` |
| 1 | `(T0, T1)` | `Ix1` | `T1` | `T0, Off` |
| 2 | `(T0, T1, T2)` | `Ix2` | `T2` | `T0, T1, Off` |
| 3 | `(T0, T1, T2, T3)` | `Ix3` | `T3` | `T0, T1, T2, Off` |

## Compile-time checks

A syntax or range error expands to `compile_error!`. That includes:

- an open outer range;
- a step that is not positive;
- a start after the end, other than an empty hole-edge slice;
- a bound outside the domain;
- more than one range in one pair of brackets;
- a `paste(...)` that is not `paste(Ident, n)`;
- a `repeat(...)` that is not `repeat(Ident, range)` or `repeat((row), range)`;
- `X Name[..]` or `X + Name[..]` where `repeat(Name, range)` is required;
- an index binding that is not `$name = paste(Ident, n);` or `$name = n;`.

An empty hole-edge slice expands to no tokens.

## Build bench

`cargo bench --bench ph_util_foreach_measure` asks `ph_bench_build` to time a downstream crate that depends on this one. The case sources live under `benches/crates/ph_util_foreach/cases`. CI runs this bench offline.

## License

Licensed under the [GNU GPL v3 or later](../../../LICENSE) (`GPL-3.0-or-later`).

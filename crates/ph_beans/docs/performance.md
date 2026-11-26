# Performance requirements

`ph_beans` moves work to build time. Each requirement here is a contract on the
generated code, and each one is checked by a test. A requirement without a test
is not met.

| ID | Requirement | Test | Status |
| --- | --- | --- | --- |
| PERF-1 | The systems of the emitted event run at once, as a direct call with no run-time check | `tests/crates/ph_beans/perf.rs` (asm), `a_deferred_event_runs_after_the_direct_maps_and_each_map_runs_once` (behaviour) | **met** |

## Emit kinds

There are two kinds of emit. User systems are pure functions over their
claims: they do not emit. Events are emitted from outside, through `Tick`.

| Kind | Call | When the systems run |
| --- | --- | --- |
| Direct | `Tick::emit(event, &mut instance, injects)` | now, in this call |
| Deferred | `Tick::defer(event, &mut instance)` | later, the next time the event's scope event is emitted, in the order of the `defer` calls |

## The tick's descriptor

`impl Tick<M>` hides the pipeline type. What stays visible is a constant,
`Tick::DESC` (`ModuleDesc`): the events the pipeline declares with `.emit`
(`emits`, `declares::<N>()`) and whether an emit reads an event buffer
(`reads_queue::<Ev>()`). The library decides from it, and a caller can ask it
too; every answer is a build-time constant. Test:
`a_hidden_tick_still_describes_its_pipeline`.

## PERF-1: a direct emit is a direct call

The emitted event is a type, so every mask it needs is a constant:
`Collect::MASK` of the emit tree, and `MatchEmit::NEED` of each map filter. The
walk keeps or drops each map with a `const` test, so the compiler removes the
dropped maps. Events deferred into the event's scope are known only at run
time, so they are the one place with run-time checks.

`Tick::emit(E, …)` runs two walks, in this order:

1. **The systems of `E`.** In the generated code this part:
   - contains no load, test, or branch on an `EmitMask`, a scope buffer, or a
     filter;
   - calls the systems whose filters match `E`, in pipeline order, and nothing
     else. After inlining, it is the bodies of those systems;
   - has no indirect or virtual call, no allocation, and no panic path added by
     `ph_beans`.
2. **Deferred events.** A deferred event waits in the state of its scope
   event: that state holds an `EventBuf` (trait `DeferState`), and the
   module holds the event as a bean. The next emit of the scope event `E`
   runs the buffer after step 1.
   - Whether a buffer is read at all is asked of the tick's descriptor,
     `Tick::DESC.reads_queue::<E>()`, a constant: `.emit` declares an event
     that can be deferred into `E`. Then the cost of an empty buffer is one
     load of its length and one branch on it. A pipeline with no `.emit`
     reads no buffer.
   - Which buffer is read is a constant too: the walk over the declared
     `.emit` tree keeps only the scope event whose bit is `E`'s.
   - The entries run oldest first, in the order of the `defer` calls; an
     event deferred twice runs twice. Another event's buffer is not touched.
   - One entry runs like a direct emit of its path inside `E` (the maps of the
     event and of its parents), in pipeline order. The path test against a
     map's filter is the one run-time check; a map on `E`'s own path never
     runs from the buffer (step 1 ran it).
   - Running the buffer does not allocate. Only `defer` can grow it, and it
     keeps its capacity.
   - A scope event the module does not hold, or whose state has no buffer, is
     `FilterError<7>`, a trait bound: `cargo check` reports it.
[`perf.rs`](../../../tests/crates/ph_beans/perf.rs) compiles the probe
[`probes/emit.rs`](../../../tests/crates/ph_beans/probes/emit.rs) in release
against the built `ph_beans` rlib (`ph_test_asm::AsmTester::extern_crate`).
Each system writes its own constant into a `u32` state, so the store count
shows which systems run. Every probe also asserts no virtual call, no
allocation and no unwind path.

| Probe | Pipeline | Asserts |
| --- | --- | --- |
| `emit_direct` | two maps under `Update`, emit `Update` | no branch, no load, 1 store (the two systems; no buffer is read) |
| `emit_without_declared` | one map under `Update`, one under `Update[Main]`, no `.emit` | no branch, no load, 1 store: no buffer is read |
| `emit_into_child` | `Shell` forwards `Update` to the `Inner` it provides | no branch, no load, 1 store: the child's system, inlined |
| `emit_with_deferred` | the same maps and `.emit(Update[Main])`, emit `Update` | no virtual call, allocation or unwind path in the function and in the buffer loop |

A call into a `ph_beans` helper that is not inlined shows up as extra stores
(the call's prologue), so the store caps also catch an out-of-line call. Small
non-generic helpers on the emit path (`EmitMask` operations, `EventBuf`,
`EmptySlots::empty`) are `#[inline]` for this reason: without it a probe linked
against the debug rlib kept calls such as `bl …contains`.

On aarch64 (macOS) the release assembly of `emit_direct` is:

```text
emit_direct:          mov x8, #11 ; movk x8, #22, lsl #32
                      str x8, [x0]                ; write_first + write_second
                      ret
```

## Gaps

- The probes run on every target CI builds. The asserted counts were read on
  aarch64 macOS; the x86_64 Linux job is the first run there.
- `FilterError<6>` is a `const` assert. It fires in `cargo build` and
  `cargo test`, not in `cargo check`, and `ph_test_errors` (metadata only)
  cannot assert it. The check reads `Tick::DESC`, a constant; a constant
  cannot be a trait bound on stable, so it is not a `Descriptor::Status`.
- The probe API cannot count the loads of the empty-buffer path alone. On
  aarch64 it is one load of the buffer length and one `cbz`; that is read from
  the assembly, not asserted.
- The asm probes check cost, not behaviour: a map that runs twice with the same
  constant is merged by the compiler. The behaviour test covers that.

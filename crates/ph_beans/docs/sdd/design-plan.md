# `ph_beans` — implementation plan

Design source of truth: [`design.md`](design.md). **Do not rewrite design mid-flight.** If Rust cannot express a clause, stop and report.

This file tracks **what is done**, **what is still open**, and **coding MUST/FORBIDDEN** for the prototype. It does not replace `design.md`.

---

## Design map

| Topic | `design.md` |
| --- | --- |
| Rules 1–23 (**R23** wins: never duplicate a walk, accessor, or operator list; R8 strategy; R21 newtype; R22 const on type; R20 ClaimDesc) | Crate module structure |
| DI / inject `&dyn`: thin `Dyn` + `DynObject` on the trait object; `Get`/`Mut` frozen (R19) | Modules (DI) → Inject `&dyn`; Accessors; rule 19 |
| Proxy (`type = proxy`, `proxy = P::T`; no bare proxy inject; wrap on target inject) | Modules (DI) → Proxy |
| `.inject_resources`, OpDesc L1/L2 | OpDesc; Accessors |
| Live Accessors, wait, replace | Descriptor → Accessors |
| Tick, `MAX_MAP_SYSTEMS = 16`, shared `List`/`Leaf`/`Grow` | Context → ContextTick |
| `E` inverted spine | Events → Operator `E` |
| Filter AST + `FilterDesc` | Events → Filters; Filter effect descriptor |
| Emit, Parents, runtime select | Events → Emit; Runtime selection |
| Direct vs deferred (`EventBuf` in the scope event's state) | Events → Dispatch: direct and deferred |
| Public runtime | Outside: `Tick::emit` (direct) and `Tick::defer` (next tick). Inside: direct child or defer |

---

## Coding MUST / FORBIDDEN

Aligned with `design.md` rules. Prefer these IDs in review notes. **R23 outranks every row below.**

| ID | MUST | FORBIDDEN |
| --- | --- | --- |
| **R5** Step return | Inherent builder method + **named step trait** with `type Out`. Signature returns `<Self as Step<…>>::Out`. | Inherent `-> Init<E, C::With<…>>` / `Ctx<…>` / `Pipe<…>` tip spelled inline. |
| **R6** Macros | Explicit impls / trait recursion. | `macro_rules!` / proc-macros / `impl_foreach` without owner OK. |
| **R8** Strategy + no matrices | **Trait = strategy**, **structs = variants**, GAT `Args` when inputs differ; **one** blanket on fan-out carriers (`PathNode<S>`, flags, …). Sibling fan-out methods (`walk` / `walk_mask` / `walk_exact`) are one `walk<S: Strategy>`. A root bound sits on that method's `where`. | Parallel traits per mode (`PathWalk`+`RunDense`); Sparse×Dense×arity copies; the same forwarding body copied once per mode on every tuple; product tables; matrix “for clarity”; the same bound on the leaf `impl` header (solver overflow). |
| **R9** Context tip | One GAT: `Context::With<Cfg>` + `CtxConfig` (+ `ApplyPatch`). **`impl<C, …> CtxConfig<C>`** + seal. Type params on the config **only for a carried value**; bean-derived steps use a **unit** config (`SetDefault`, `SetProviders`) — seal reads `Bean` from `Ctx`. With the bound present, **`T::Assoc`**. Defaults: **`Default::default()`**. | Hand-built `type Out = Ctx<B,A,NeedBuild, T::WithInit<<B::Proxy as ForwardInit<…>>::Out>>` (or any spelled tip); `WithArgs`/`WithStatus`/`WithTick`/`WithInit` GATs; `CtxConfig<Ctx<B,A,S,T>>`; `SetDefault<D>` / step spelling `Bean::State`; stacked lenses; needless UFCS. |
| **R10** Generics | **One letter** from the role (`B`,`A`,`S`,`T`,`E`,`H`,`F`,`N`,`O`,`P`,…), or **letter + digit** (`E0`); **comment** on the item maps each letter to its role. | Multi-letter params (`Bean`, `Step`, `Strat`, …); alphabet arity runs; silent one-letter params with no role comment. |
| **R7 / R11–12** Lists | Shared `List` / `Leaf` / `Grow`; **flat** leaf ≤ **8**; opaque `E0…`; 9 → `(List, Grow)`; 16 → **4** `List`s. | Cons spines `(Get<I0>, (Get<I1>, (…, ())))` / `MapNode` / `Cons` (solver re-proves); `room_*`; flat 9…16 megaimpls; domain structs in arity heads; bare `(E0,E1)` push ≡ `(List, Grow)`. |
| **R13** Push | One `context/push.rs`; Args/`TakeMany` shape — one `PushShape` blanket + sealed `AddItem` (Cap6). | Split `push/{leaf,group,wide,add}.rs`; unsealed `AddItem` on 7-tuples; `MapEntry` (or other domain) in push arity heads. |
| **R14** Meta | `BeanInfo` / `EventInfo` / `ProxyInfo` data-only; `Query` + `StartDesc` for entry tip. | Methods / new assoc types on meta traits without owner OK; spelling `Ctx`/`Pipe`/`MapList` on `query` / `BeanInfo`. |
| **R15** Phantom | One phantom per struct; named field is always **`_markers`**. | Several phantoms; named phantom as `event` / `_filter` / `_path` / … |
| **R16** Safety | `#![forbid(unsafe_code)]`; no owned `Box<dyn>`; inject cast → `&dyn` args (may inline). | `unsafe`; `Box<dyn>` in `State`; `dyn` / `as Trait` in **provides**. |
| **R17** Trait bounds | Shared predicates on the **trait head** (`Update`/`Tick`/`Setup`: `BeanInfo` + `Injects: ArgsView`). `Update` only for `Built`. | Same bounds re-copied on every `fn`; stub `Update for Init`; dropping `injects` from `emit` because the body ignores them. |
| **R18** Pure systems | Systems touch **only** args (+ init return). Bound `Fn`. Call site = **function item**. Under the hood = **pure `fn` / function item** (build speed + future SIMD store). | Capturing closures; `FnMut` / env mutation; storing systems as closure objects “for convenience”. |
| **R19** Frozen shapes | Leave landed public/crate API types as-is; extend with **new** types beside them. Prior **owner OK** before any field/generic/signature change. | Quiet reshape of `Get`/`Mut`/`Marker`/meta/list carriers; “small” extra type params without OK. |
| **R20** Claim wrappers | Pure forward carriers (`Live`/`Idle`/…) implement **`ClaimDesc`**; one `ClaimShape`/`Claim` blanket. | Per-wrapper `impl ClaimShape for Live/Idle/…`; copy-paste forwards. |
| **R21** Domain values | Named **struct** + inherent methods (`EmitMask::or` / `contains`). | Raw `u128`/`u32` as the domain value; `type Alias = u128`. |
| **R22** Consts | Associated const on the owning type (`EmitMask::BITS`). | Free `EMIT_BITS` beside the type. |
| **R23** No copies | **One** descriptor names an operator set (`ClaimForm`, `ClaimDesc`, `CarrierDesc`, `FilterDesc`). The next trait is **one blanket**. Event state is `Get<State<Ev>>` / `Mut<State<Ev>>`: the state part of `Ev`'s slot (`ClaimLeaf`). This row wins over the others. | A second copy of `()` / `Get` / `Mut` / `Dyn` / `And` on the next trait; a `StateGet` / `StateMut` accessor; any new accessor that only repeats an existing one. |
| **R24** One-line types | A type in a signature (`type` alias, associated type, return type) fits on **one** line at `max_width`. If it would wrap, name the inner piece on its owner until it fits. | Wrapping `Ctx<…>` / `Pipe<…>` / `MapList<…>` across lines instead of changing the signature. |
| **R25** PhantomData | `PhantomData` is a **struct field** for a type the struct does not store. | `let _ = PhantomData::<T>` (or inside a discarded tuple) to pretend a generic is used. |

Also always: Borrowing (`At`/`Ix*`/`TakeMany`); no API `type` aliases; no god modules; one-crate PRs (`ph_beans`); push only when asked.

**Verify before handoff:**

```bash
cargo fmt --check
cargo clippy -p ph_beans --all-targets -- -D warnings
cargo test -p ph_beans
```

---

## Status (explicit)

| Item | Status | Evidence / gap |
| --- | --- | --- |
| Descriptor / `PipelineOk` / linear `Ctx` | **Done** | `descriptor/`, `context/mod.rs` |
| `Setup` / `Tick::init(injects)` → instance / `Tick::emit(event, &mut instance, injects)` | **Done** | `query/setup.rs`, `query/boot.rs`; `Update` only on `Built`; `Built` holds systems, `Owned` is the instance |
| **R17** trait-head bounds | **Done** | `Update`/`Tick`/`Setup` carry `BeanInfo` + `Injects: ArgsView`; no `Update for Init` |
| `init_state` | **Done** | `SetInit<H>` + `SealInit`; a default state is `.init_state(\|_\| Default::default())` (`init_state_default` removed) |
| **R9** unit configs (no fake `D`) | **Done** | `SetDefault`, `SetProviders` are unit; step = `ApplyPatch<SetDefault>` |
| `.inject_resources` + wait gate | **Done** | `query/resource.rs` |
| `Claim` trait-recursive | **Done** | `query/args.rs` — watch leftover pair tables |
| Accessors + wait / replace | **Done** | `descriptor/live.rs`, filters |
| `FilterDesc` + one `ApplyFilter` | **Done** | `filter/apply.rs` |
| Filter AST + `E` + emit / schedule | **Done** | `filter/`, `query/emit.rs`, `query/schedule.rs` |
| Direct emit + event buffer | **Done** | `Tick::emit` / `Tick::defer`; deferred events wait in the scope event's state (`EventBuf`, `DeferState`). By design a system never emits: `.map_at` / `Direct` removed |
| Emit **bitmask** + build-time `NEED`/`MASK` | **Done** | `EmitMask` newtype (`BITS=128`, `from_bit`/`or`/`contains`); `Collect::MASK` / `MatchEmit::NEED`; Drive via `contains` |
| Path **Gate** + `PathStrategy` (R8: Sparse/Dense structs, GAT `Args`, `PathNode<S>` blankets) | **Partial** | `filter/path.rs`; flat `Drive` unchanged; Query builder still flat — wire gates into Drive later |
| **R9** `CtxConfig` / `Context::With` / `ApplyPatch` | **Partial** | Seals + unit/`payload` configs; keep sweeping leftover needless UFCS / any remaining `CtxConfig<Ctx<…>>` |
| **R5** step trait + `Out` on inherent methods | **Done** | `InjectResourcesStep`, `FilterStep`, `MapStep`, `InitStateStep`, `EmitStep`, `BuildStep`, … |
| **R11–13** shared list + push table | **Done** | `context/list.rs` (`List`/`Leaf`/`Grow`); one `context/push.rs` (`AddItem` + `PushShape`); no `push/` submods |
| **R14** meta freeze + `Query` / `StartDesc` | **Done** | `BeanInfo` data-only; `context/start.rs` (`StartDesc`); `query/entry.rs` (`Query`) |
| **R15** one phantom per struct | **Done** | Every phantom is a named `_markers` field. `Get` / `Mut` keep the frozen tuple shape (rule 19) |
| **R16** no `unsafe` / no `Box<dyn>` | **Done** | Rule fixed; `&dyn` views documented under Provide / inject `&dyn` |
| Inject `&dyn` (`as Trait`; claim `Bean::T`; only the system argument is `&dyn`) | **Done** | `Entry` / `Holds` + `Claim<B>::{Seen, Arg}`; cast at the `init` / `emit` call; systems take `&mut dyn Trait`; `DynObject` marks a dynamic trait and names its object for a borrow (`DynView` removed); asm probe `probes/dyn_inject.rs` |
| **Proxy** (`type = proxy`, wrap on target inject) | **Open** | Design under Modules → Proxy; `ProxyInfo` meta TBD; no code yet |
| **R19** no silent structure changes | **Done** | Rule fixed; `Get`/`Mut`/`Marker` called out under Accessors |
| **R20** `ClaimDesc` blankets | **Done** | `Live`/`Idle` → `ClaimDesc`; `Get<State<P>>`/`Mut<State<P>>` → `ClaimLeaf { Part = StatePart }`; `ClaimShape`/`Claim` blankets in `query/args.rs` |
| **R23** no duplicate walks | **Done** | `ClaimForm` + one `StateGate` blanket; state claim reuses `Get`/`Mut` |
| Map cap 16 | **Done** | `MAX_MAP_SYSTEMS`; push refuses 17th |
| No macros in crate src | **Done** | No `macro_rules!` under `ph_beans/src` |
| **R8** filter ops / flags | **Done** | `FilterOperand` RHS; per-LHS orphan residual; `OrWith` / `WaitFlag`; `ops_node` removed |
| **R8** DI `FromParent` | **Done** for provides | `Provided` + `Allows` (no Get×Mut split, no `ClaimMark`); `OfProvide<N>` is any slot through `Take`. `OfInject<0>` / `<1>` remain as the E0119 Via for a pair of injects |
| **R10** one-letter generics | **Done** | No multi-letter type parameter in `src`; every `impl<…>` carries a `Generics:` comment. Trait and struct declarations name their parameters in rustdoc |
| `OpDesc` L1 gate on `.inject_resources` | **Partial** | Types exist — confirm call is gated on `OpOk` + tests |
| Handlers `Fn` (not `FnMut`) | **Done** | `.map` / init use `Fn()` — see **R18** |
| **R18** pure systems / `fn` store | **Partial** | Bound + docs: no captures, prefer fn items; storage still generic `H` — tighten to `fn` when store lands |
| Warning `#[deprecated]` on `.build` | **Open** | Not implemented |
| Derive / root `physarum` rewrite | **Out of track** | Needs owner OK (macros) |

---

## Next slice (ordered)

Do in this order. Each item is a small PR on `ph_beans` only.

1. **R9 seal sweep (mandatory)** — every `CtxConfig` → `impl<C, …> CtxConfig<C>`; unpack `Ctx` only in seal helpers. Convert remaining `SetFilter` / `SetEmit` / `SetMap` / `SetArgs` / `SetStatus` / … (reject new `CtxConfig<Ctx<B,A,S,T>>` heads).
2. **R1 file size** — `core/each.rs` (233) and `filter/ops.rs` (240) are over the 150–200 aim: one impl per tuple length, and one operator arm per left-hand type (orphan rule).
3. **R8 DI** — replace `OfInject<0>` / `<1>` when a non-overlapping membership encoding exists (E0119 residual today).
4. **OpDesc L1** — ensure `.inject_resources` refuses duplicates / `T`+`Mut` on that line; add/strengthen tests.
5. **FilterDesc follow-ups (optional)** — `ArgsView`+`OpLeaf` merged into `ClaimShape` (+ `Claim<B>` indexes); rlib incr unchanged in local measure — revisit if a heavier claim probe shows solver cost.
6. **Keep green** — fmt, clippy `-D warnings`, `cargo test -p ph_beans`.

---

## Later polish (optional)

- Warning statuses via `#[deprecated]` on `.build`
- Stronger OpDesc L1 diagnostics / tests

---

## Out of this track

- Proc-macro derive: **only** `BeanInfo` + `At` / `EventInfo` / `ProxyInfo` (+ dyn/`proxy` links) — see design Derive + Proxy (**owner OK required** — R6). Not `Query` / `Setup` / `Module` / …
- Root `physarum` package rewrite
- Provided beans built by their own init system (today: `Default` only); injects in `init_state` (today: only on `emit`)
- `Dyn` views do not borrow resources yet; a module holds at most 4 beans (`Take` arity), 3 when it has injects (`AddItem` cap)
- Derive for `EmitBit` (owner OK — R6); auto bit allocation. The clash check is done: `SlotOf` + `Bit<N>`

---

## When blocked

Report the `design.md` clause + rustc limit. Do not edit design to match a shortcut unless the owner opens a design change.

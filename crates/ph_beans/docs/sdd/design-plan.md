# `ph_beans` — implementation plan

Design source of truth: [`design.md`](design.md). **Do not rewrite design mid-flight.** If Rust cannot express a clause, stop and report.

This file tracks **what is done**, **what is still open**, and **coding MUST/FORBIDDEN** for the prototype. It does not replace `design.md`.

---

## Design map

| Topic | `design.md` |
| --- | --- |
| Rules 1–19 (… pure systems; no silent API shape changes) | Crate module structure |
| DI / inject `&dyn`: thin `Dyn` + marker `DynView`; `Get`/`Mut` frozen (R19) | Modules (DI) → Inject `&dyn`; Accessors; rule 19 |
| Proxy (`type = proxy`, `proxy = P::T`; no bare proxy inject; wrap on target inject) | Modules (DI) → Proxy |
| `.inject_resources`, OpDesc L1/L2 | OpDesc; Accessors |
| Live Accessors, wait, replace | Descriptor → Accessors |
| Tick, `MAX_MAP_SYSTEMS = 16`, shared `List`/`Leaf`/`Grow` | Context → ContextTick |
| `E` inverted spine | Events → Operator `E` |
| Filter AST + `FilterDesc` | Events → Filters; Filter effect descriptor |
| Emit, Parents, runtime select | Events → Emit; Runtime selection |
| Public runtime | Setup → `Tick::update` only (no `boot`) |

---

## Coding MUST / FORBIDDEN

Aligned with `design.md` rules. Prefer these IDs in review notes.

| ID | MUST | FORBIDDEN |
| --- | --- | --- |
| **R5** Step return | Inherent builder method + **named step trait** with `type Out`. Signature returns `<Self as Step<…>>::Out`. | Inherent `-> Init<E, C::With<…>>` / `Ctx<…>` / `Pipe<…>` tip spelled inline. |
| **R6** Macros | Explicit impls / trait recursion. | `macro_rules!` / proc-macros / `impl_foreach` without owner OK. |
| **R8** No matrices | Collapse whenever possible (flag policy, `FilterOperand`, `ClaimMark`). | Product tables; new Get×Mut×slot; keeping a matrix “for clarity” when a thinner shape works. |
| **R9** Context tip | One GAT: `Context::With<Cfg>` + `CtxConfig` (+ `ApplyPatch`). **`impl<C, …> CtxConfig<C>`** + seal. Type params on the config **only for a carried value**; bean-derived steps use a **unit** config (`SetDefault`, `SetProviders`) — seal reads `Bean` from `Ctx`. With the bound present, **`T::Assoc`**. Defaults: **`Default::default()`**. | `WithArgs`/`WithStatus`/`WithTick`; `CtxConfig<Ctx<B,A,S,T>>`; `SetDefault<D>` / step spelling `Bean::State`; `T::default()` / `<B as BeanInfo>::State::default()` when the type is inferred; stacked lenses; needless UFCS. |
| **R10** Generics | **One letter** from the role (`B`,`A`,`S`,`T`,`E`,`H`,`F`,`N`,`O`,…), or **letter + digit** (`E0`). | Multi-letter params (`Bean`, `Args`, `Status`, `PrevE`, `New`, …); alphabet arity runs `A,B,C,…`. |
| **R11–12** Lists | Shared `List` / `Leaf` / `Grow`; leaf ≤ **8**; opaque `E0…`; 9 → `(List, Grow)`; 16 → **4** `List`s. | Flat 9…16 megaimpls; `MapNode` / `room_*`; tuples of domain structs in arity heads (`MapEntry<…>, …`); bare `(E0,E1)` push that overlaps `(List, Grow)`. |
| **R13** Push | One `context/push.rs`; Args/`TakeMany` shape — one `PushShape` blanket + sealed `AddItem` (Cap6). | Split `push/{leaf,group,wide,add}.rs`; unsealed `AddItem` on 7-tuples; `MapEntry` (or other domain) in push arity heads. |
| **R14** Meta | `BeanInfo` / `EventInfo` / `ProxyInfo` data-only; `Query` + `StartDesc` for entry tip. | Methods / new assoc types on meta traits without owner OK; spelling `Ctx`/`Pipe`/`MapList` on `query` / `BeanInfo`. |
| **R15** Phantom | One phantom per struct; named field is always **`_markers`**. | Several phantoms; named phantom as `event` / `_filter` / `_path` / … |
| **R16** Safety | `#![forbid(unsafe_code)]`; no owned `Box<dyn>`; inject cast → `&dyn` args (may inline). | `unsafe`; `Box<dyn>` in `State`; `dyn` / `as Trait` in **provides**. |
| **R17** Trait bounds | Shared predicates on the **trait head** (`Update`/`Tick`/`Setup`: `BeanInfo` + `Injects: ArgsView`). `Update` only for `Built`. | Same bounds re-copied on every `fn`; stub `Update for Init`; dropping `injects` from `update` because the body ignores them. |
| **R18** Pure systems | Systems touch **only** args (+ init return). Bound `Fn`. Call site = **function item**. Under the hood = **pure `fn` / function item** (build speed + future SIMD store). | Capturing closures; `FnMut` / env mutation; storing systems as closure objects “for convenience”. |
| **R19** Frozen shapes | Leave landed public/crate API types as-is; extend with **new** types beside them. Prior **owner OK** before any field/generic/signature change. | Quiet reshape of `Get`/`Mut`/`Marker`/meta/list carriers; “small” extra type params without OK. |

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
| `Setup` / `Tick::update` / no `boot` | **Done** | `query/setup.rs`; `Update` only on `Built` |
| **R17** trait-head bounds | **Done** | `Update`/`Tick`/`Setup` carry `BeanInfo` + `Injects: ArgsView`; no `Update for Init` |
| `init_state` / `init_state_default` | **Done** | `SetInit<H>` / unit `SetDefault` + `SealDefault` (Bean::State in seal) |
| **R9** unit configs (no fake `D`) | **Done** | `SetDefault`, `SetProviders` are unit; step = `ApplyPatch<SetDefault>` |
| `.inject_resources` + wait gate | **Done** | `query/resource.rs` |
| `Claim` trait-recursive | **Done** | `query/args.rs` — watch leftover pair tables |
| Accessors + wait / replace | **Done** | `descriptor/live.rs`, filters |
| `FilterDesc` + one `ApplyFilter` | **Done** | `filter/apply.rs` |
| Filter AST + `E` + emit / schedule | **Done** | `filter/`, `query/emit.rs`, `query/schedule.rs` |
| **R9** `CtxConfig` / `Context::With` / `ApplyPatch` | **Partial** | Seals + unit/`payload` configs; keep sweeping leftover needless UFCS / any remaining `CtxConfig<Ctx<…>>` |
| **R5** step trait + `Out` on inherent methods | **Done** | `InjectResourcesStep`, `FilterStep`, `MapStep`, `InitStateStep`, `EmitStep`, `BuildStep`, … |
| **R11–13** shared list + push table | **Done** | `context/list.rs` (`List`/`Leaf`/`Grow`); one `context/push.rs` (`AddItem` + `PushShape`); no `push/` submods |
| **R14** meta freeze + `Query` / `StartDesc` | **Done** | `BeanInfo` data-only; `context/start.rs` (`StartDesc`); `query/entry.rs` (`Query`) |
| **R15** one phantom per struct | **Done** | `Ctx` / `MapList` / `SetFilter` use one `_markers` or tuple phantom |
| **R16** no `unsafe` / no `Box<dyn>` | **Done** | Rule fixed; `&dyn` views documented under Provide / inject `&dyn` |
| Inject `&dyn` (`as Trait`; claim `Bean::T`; only ArgsView is `&dyn`) | **Open** | No `as_dyn` on Get/Mut; marker-only `DynView`; `ph_test_asm` |
| **Proxy** (`type = proxy`, wrap on target inject) | **Open** | Design under Modules → Proxy; `ProxyInfo` meta TBD; no code yet |
| **R19** no silent structure changes | **Done** | Rule fixed; `Get`/`Mut`/`Marker` called out under Accessors |
| Map cap 16 | **Done** | `MAX_MAP_SYSTEMS`; push refuses 17th |
| No macros in crate src | **Done** | No `macro_rules!` under `ph_beans/src` |
| **R8** filter ops / flags | **Done** | `FilterOperand` RHS; per-LHS orphan residual; `OrWith` / `WaitFlag`; `ops_node` removed |
| **R8** DI `FromParent` | **Partial** | `ClaimMark` done; a pair uses `OfProvide0`/`1`; the inject tuple is `Collected::Injects` on the pipeline descriptor — no index walk, no recursive tail, no Get×Mut split |
| **R10** one-letter generics | **Open** | Rule fixed; many files still use `Bean`/`Args`/`Status`/`Tick`/`PrevE` — **sweep required** |
| `OpDesc` L1 gate on `.inject_resources` | **Partial** | Types exist — confirm call is gated on `OpOk` + tests |
| Handlers `Fn` (not `FnMut`) | **Done** | `.map` / init use `Fn()` — see **R18** |
| **R18** pure systems / `fn` store | **Partial** | Bound + docs: no captures, prefer fn items; storage still generic `H` — tighten to `fn` when store lands |
| Warning `#[deprecated]` on `.build` | **Open** | Not implemented |
| Derive / root `physarum` rewrite | **Out of track** | Needs owner OK (macros) |

---

## Next slice (ordered)

Do in this order. Each item is a small PR on `ph_beans` only.

1. **R9 seal sweep (mandatory)** — every `CtxConfig` → `impl<C, …> CtxConfig<C>`; unpack `Ctx` only in seal helpers. Convert remaining `SetFilter` / `SetEmit` / `SetMap` / `SetArgs` / `SetStatus` / … (reject new `CtxConfig<Ctx<B,A,S,T>>` heads).
2. **R10 generics sweep** — rename every multi-letter type param to one letter or letter+digit (`B`,`A`,`S`,`T`,`E0`,…); include `context/`, `query/`, `filter/`, `bean/`, docs examples that still show `Bean`/`PrevE`.
3. **R8 DI** — keep `ClaimMark`; replace `OfProvide0`/`1` when a non-overlapping membership encoding exists (E0119 residual today).
4. **OpDesc L1** — ensure `.inject_resources` refuses duplicates / `T`+`Mut` on that line; add/strengthen tests.
5. **FilterDesc follow-ups (optional)** — align `Claim` / `ArgsView` / `OpLeaf` if still duplicated walks.
6. **Keep green** — fmt, clippy `-D warnings`, `cargo test -p ph_beans`.

---

## Later polish (optional)

- Warning statuses via `#[deprecated]` on `.build`
- Stronger OpDesc L1 diagnostics / tests

---

## Out of this track

- Proc-macro derive: **only** `BeanInfo` + `At` / `EventInfo` / `ProxyInfo` (+ dyn/`proxy` links) — see design Derive + Proxy (**owner OK required** — R6). Not `Query` / `Setup` / `Module` / …
- Root `physarum` package rewrite
- Full production scheduler beyond emit-mask rules

---

## When blocked

Report the `design.md` clause + rustc limit. Do not edit design to match a shortcut unless the owner opens a design change.

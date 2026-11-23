# Query pipeline and modules

## Core idea

**One `Descriptor` surfaces the whole pipeline status.**

Resources, init, map, filters, events, emit, and build all update the same `Descriptor::Status` (and related projections). Diagnostics — errors and warnings — go through that one channel. Events do not get a separate error path.

Modules (`provide` / `inject`) form the DI tree. Query pipelines run per module; each module can be set up and tested with only its injects.

```text
Operator<E, C>
       │
       └─ C: Context + Descriptor
              Descriptor::Status     ← pipeline health
              Descriptor::Accessors  ← live resource accessors (type-level set)
```

---

## Crate module structure

Layout of `crates/ph_beans/src/`. Keep the surface readable: **small files**, **named types**, **Borrowing-style abstractions**, **descriptors** for status — not god modules and not `type` aliases.

### Top-level modules

| Module | Owns |
| --- | --- |
| `borrow/` | Slot story: `At`, `Ix*`, `Off`, `Take` / `TakeMany`. One Borrowing vocabulary for provides and claims. |
| `descriptor/` | Pipeline lens: `Descriptor`, named `Status` types, `PipelineOk`, `Accessors`, `Get`/`Mut`/`Marker`. Split files (`status.rs`, `accessors.rs`, …). |
| `context/` | Linear `Ctx`, `Context`, `ContextTick` layout (init slot + map list). |
| `bean/` | `BeanInfo`, `EventInfo`, DI (`FromParent` / `Module`, provide–inject). |
| `query/` | Operators and steps: `Init`, `Built`, `Setup`/`Tick`, `update`, `.inject_resources`, `.map`, `.filter`, `.emit` — **separate files**, shallow `impl` heads (rule 8). |
| `filter/` | Predicate AST: `And` / `Or` / `Not` / `On`, shape / eval — not dumped into `init.rs`. |
| `context/` | `Ctx`, `Pipe`, map list as **`List` / `Leaf` / `Grow`** (rules 7, 11–13); push in **one** `push.rs` — not `MapNode`+`room_*`, not flat 9…16 impls, not `push/{leaf,group,wide}`. |

`lib.rs` re-exports the public API; it does not hold logic.

### Rules

1. **No god modules / god types** — aim ~150–200 lines per file. Grow by **adding a file** under the right module (`query/map.rs`, `descriptor/op_desc.rs`), not by fattening one hub. No single “engine” struct that owns status + accessors + systems + DI.
2. **No `type Alias = …` in the API** — aliases hide structure and make errors/docs harder to read. Prefer a **named struct or enum** (or a clear tuple of named ZSTs). Associated types on traits (`Descriptor::Status`, `At::Ix`, `BeanInfo::Provides`) are fine; they are not file-level aliases.
3. **Borrowing-style abstractions** — indexes and holes are **named zero-sized types** (`Ix0`…, `Off`), projection via `At` / `Take` / `TakeMany`, not raw `usize` or aliased tuples. New claim/filter machinery should follow the same pattern (trait + small ZSTs), not ad-hoc helpers.
4. **Descriptors over ad-hoc flags** — pipeline and operator health go through `Descriptor` / `OpDesc` and **named status types** (`NeedBuild`, `ErrTooManySystems`, …). Do not scatter bools or stringly errors across modules.
5. **Inherent step + named step trait with `Out`** — public builder calls (`.inject_resources`, `.filter`, `.map`, `.build`, `.init_state`, `.emit`, …) stay **inherent** on the operator. Each such method is backed by a **named step trait** (`InjectResourcesStep`, `FilterStep`, …) with associated type **`Out`**. The inherent signature returns **`<Self as StepTrait<…>>::Out`** (and bounds `Self: StepTrait<…>`), so the return type stays readable and does not spell `Init<E, Ctx<…>>` / `C::With<…>` towers.

   ```rust
   // Required — step trait carries Out; inherent method only forwards
   pub trait InjectResourcesStep<R> {
       type Out;
       fn inject_resources(self, resources: R) -> Self::Out;
   }

   impl<E, C> Init<E, C> {
       pub fn inject_resources<R>(self, resources: R) -> <Self as InjectResourcesStep<R>>::Out
       where
           Self: InjectResourcesStep<R>,
       {
           InjectResourcesStep::inject_resources(self, resources)
       }
   }
   ```

   **Forbidden:** inherent methods whose return type writes the next tip inline (`-> Init<E, C::With<SetArgs<Live<R>>>>`, deep `Ctx`/`Pipe` paths). Bounds traits (`Claim`, `PipelineOk`, `ContextTick`, …) stay separate from step traits.

   Same for **pipeline entry**: `query()` is a **named step trait** [`Query`] with `type Out` (not a method on [`BeanInfo`]). `Out = Init<(), B::Ctx>` — the empty tip comes from the **context start descriptor**, not a spelled `Ctx<…>` / `Pipe<…>` / `MapList<…>` on the call site.
6. **No macros without owner permission** — do not add `macro_rules!`, `impl_foreach`, or proc-macros in `ph_beans` unless the owner explicitly allows that use. Default: **explicit impls** and **trait-recursive / extractor bounds** (as with `Claim`, `At`, `TakeMany`). Existing macros in the prototype (`maps`/`schedule`/`filter` helpers, future derive) need an explicit OK or must be removed/replaced.
7. **Tuple / trait-recursive lists only — no nested “room” tables** — multi-value type lists (maps, claims, DI, …) use **tuples** (and optional wrapper structs — rules 12–13) with trait recursion over **groups**, not a cons spine of `MapNode<F, H, MapNode<…>>`. Role names for params (rule 10). Leaf arity and grouping: **rules 11–13**. **Forbidden:** hand-unrolled recursive nestings with a separate `Room` (or similar) impl per length (`room.rs` / `room_mid` / `room_hi` / `room_top` style).
8. **No closed matrices when a thinner shape exists** — if there is **any** workable way to replace a product table (A×B×C impls, Live×Yes/No, Get×Mut×slot, …) with a **policy on a marker**, an **extractor / membership trait**, or **one blanket**, do that. Reviewers **reject** new matrices when a collapse is possible. Arity walks like `Take` / `AddItem` (one axis = length) are not this rule’s target.

   **Required shapes:**

   | Area | Do | Don’t |
   | --- | --- | --- |
   | Wait / flags | Policy on `Yes`/`No` (`WaitFlag`, `OrWith`) + blankets | `Live`×`Idle`×`Yes`×`No` / `(Yes\|No)×(Yes\|No)` tables |
   | Filter ops | RHS = `FilterOperand`; one thin op set per **local** LHS node | Get×Mut×And×Or×On product; macros to stamp the product (rule 6) |
   | DI | `ClaimMark` (Get\|Mut once) + membership; Via only if rustc forces it | Get×Mut × Provide/Inject × every slot |
   | Filter effects | Per-node descriptor + one `ApplyFilter` | Re-impl the step per AST node |

   ```rust
   // Required — flag policy (Rewait / OrWith)
   impl<C> WaitFlag<C> for Yes { type OnLive = Idle<C>; }
   impl<C, F: WaitFlag<C>> Rewait<F> for Live<C> { type Out = F::OnLive; }

   // Required — ops: RHS open; LHS is orphan residual (E0210 blocks
   // `impl<L: FilterOperand> BitAnd<R> for L`), not an excuse for a product table
   impl<T, R: FilterOperand> BitAnd<R> for Get<T> { type Output = And<Self, R>; … }

   // Forbidden — product matrix
   impl … BitAnd<Get<_>> for Get<_> {}
   impl … BitAnd<Mut<_>> for Get<_> {}
   impl … BitAnd<And<_,_>> for Mut<_> {}
   // …
   ```

   DI (`FromParent`): match through **membership** + `ClaimMark`. **Do not** split Get vs Mut. Slot Via (`OfProvide0` / `OfProvide1`, …) is allowed **only** as an E0119 residual when head/tail blankets overlap — document it; collapse when a non-overlapping encoding exists.

   ```rust
   // Forbidden — Get×Mut×slot
   impl FromParent<P, OfProvide0> for Get<T> where Provides = (Get<T>, R) {}
   impl FromParent<P, OfProvide0> for Mut<T> where Provides = (Mut<T>, R) {}

   // Required residual — one arm per slot for every ClaimMark
   impl<P, C: ClaimMark, R> FromParent<P, OfProvide0> for C
   where P: BeanInfo<Provides = (C, R)> {}
   ```

   **Same for filter *effects*:** policy on a descriptor, one blanket for the step — see **Filter effect descriptor**.
9. **Flat trait bounds — no projection towers; next tip via config structs** — steps resolve complexity **inside traits**, with **shallow** `where` clauses (Borrowing / `Args` style). Do **not** nest `as` projections or chain `WithTick` / `WithArgs` / `WithStatus` in return types or associated types.

   ```rust
   // Forbidden — today’s query/emit.rs / init.rs style
   -> Init<E, C::WithTick<<C::Tick as Reemit<<<C::Tick as EmitOf>::Emit as UnionEmit<T>>::Out>>::Out>>
   where
       C::Tick: Reemit<<<C::Tick as EmitOf>::Emit as UnionEmit<T>>::Out>,
       …

   type Next =
       <C::WithTick<<C::Tick as FreshInit>::WithInit<H>> as Context>::WithStatus<NeedBuild>;
   ```

   **Forbidden:** stacking context lenses (`WithTick<…>` then `as Context` then `WithStatus<…>`, or `WithArgs` + `WithTick` in one associated type); recursive `<<<… as …>::… as …>::Out` chains in `where` or `type Out` / `type Next`; unpacking `Ctx`/`Pipe`/`MapList` in the `impl` head; spelling the next tip as a deep return type.

   **Required:** one GAT on the context — **`Context::With<Cfg>`** — driven by a **config struct** + [`CtxConfig`]. **Forbidden:** separate `WithArgs` / `WithStatus` / `WithTick` GATs (they are the same pattern three times). Single-field and multi-field steps both use configs (`SetArgs`, `SetStatus`, `SetInit`, `SetFilter`, …).

   ```rust
   trait CtxConfig<C: Context> {
       type Out: Context;
       type ScopeOf<E>; // filter may replace event scope; others keep E
       fn apply(self, ctx: C) -> Self::Out;
   }

   trait Context: Descriptor + Sized {
       type With<Cfg: CtxConfig<Self>>: Context; // = Cfg::Out
       fn configure<Cfg: CtxConfig<Self>>(self, cfg: Cfg) -> Self::With<Cfg>;
       …
   }

   // Init glues scope + context once
   // Out = Init<Cfg::ScopeOf<E>, C::With<Cfg>>
   type Out = <Self as ApplyPatch<SetFilter<F, E>>>::Out;
   ```

   Nesting of `EmitOf` / `UnionEmit` / `FreshInit::WithInit` / `ApplyFilter` stays **inside** the config impl (and `ScopeOf`), not at the step’s `type Out`.

   **Mandatory — no needless `<T as Trait>::Assoc`:** when `T` is a **type parameter** already bound `T: Trait` (or `T: Trait<…>`), write **`T::Assoc`** / **`T::Assoc<…>`**, not `<T as Trait>::Assoc` (e.g. `F::NextArgs`, `T::Out`, `C::Out`, `B::Ctx`, `G::ScopeOf<E>`, `A::Out`).

   ```rust
   // Forbidden — redundant UFCS when the bound is present
   F: ApplyFilter<E, A>,
   <F as ApplyFilter<E, A>>::NextArgs: AccSet,
   type Out = Ctx<B, <F as ApplyFilter<E, A>>::NextArgs, S, <T as Rescope<F>>::Out>;

   // Required
   F: ApplyFilter<E, A>,
   F::NextArgs: AccSet,
   type Out = Ctx<B, F::NextArgs, S, T::Out>;
   ```

   **Keep** UFCS when rustc needs it: inherent step returns (`<Self as Step>::Out`, rule 5); disambiguating another trait’s `Out` (`<Self as ApplyPatch<…>>::Out`); projecting through an **associated type path** (`<C::Bean as BeanInfo>::State`, `<M::Injects as ArgsView>::View`, `<F::Scope as ScopeOf<E>>::NextE` — E0223 without the cast); or when several traits could supply the same name.

   **Mandatory — `CtxConfig` over one context generic `C` (do not unpack `Ctx` on the config impl):**

   A [`CtxConfig`] impl **must** be written as `impl<C, …> CtxConfig<C> for Config…`. Extra type params on the config only for a **carried payload** (rule below). **Do not** write `impl<B, A, S, T, …> CtxConfig<Ctx<B, A, S, T>> for …` on the config type.

   Unpacking `Ctx<B, A, S, T>` belongs in a **seal / apply helper** trait implemented **for** `Ctx` (e.g. `SealInit`, `SealDefault`). The config only bounds `C: Context + Seal…` and forwards `apply` → seal.

   **Mandatory — unit config when there is no payload:** if the step does not move a value into the tip, the config is a **unit struct** (no type params, no `PhantomData` for bean-derived types). The seal reads what it needs from `Ctx` / `C` (`BeanInfo::State`, provider tree, …). **Do not** thread `D = Bean::State` (or similar) through `SetDefault<D>` / the step’s `ApplyPatch` bound — that belongs in `SealDefault for Ctx`.

   | Kind | Config | Seal |
   | --- | --- | --- |
   | Carries a value | `SetInit<H>`, `SetMap<H>`, `SetFilter<F, E>`, `SetEmit<E>`, `SetArgs<A>`, … | typed by that payload |
   | Bean / tip derived | **`SetDefault`**, **`SetProviders`** (unit) | `SealDefault` / `SealProviders` read `Bean` from `Ctx` |

   The step stays thin: `Self: ApplyPatch<SetDefault>` and `type Out = <Self as ApplyPatch<SetDefault>>::Out` — not `ApplyPatch<SetDefault<<C::Bean as BeanInfo>::State>>`.

   ```rust
   // Forbidden — config impl lists every Ctx slot
   impl<B, A, S, T> CtxConfig<Ctx<B, A, S, T>> for SetDefault { … }

   // Required — one context param; seal owns the Ctx layout
   // SetDefault has no type param: Bean::State is read inside SealDefault for Ctx
   trait SealDefault: Sized {
       type Out: Context;
       fn seal_default(self) -> Self::Out;
   }
   impl<B, A, S, T> SealDefault for Ctx<B, A, S, T>
   where
       B: BeanInfo,
       <B as BeanInfo>::State: Default,
       …
   { … }

   impl<C> CtxConfig<C> for SetDefault
   where
       C: Context + SealDefault,
   {
       type Out = C::Out;
       fn apply(self, ctx: C) -> Self::Out { ctx.seal_default() }
   }
   ```

   Same duty for **every** config: `SetInit`, `SetDefault`, `SetArgs`, `SetStatus`, `SetFilter`, `SetEmit`, `SetMap`, … Reviewers **reject** new `CtxConfig<Ctx<B, A, S, T>>` heads and **reject** phantom type params on unit configs that only restate `Bean::State` / other tip data.

   **Mandatory — construct defaults with `Default::default()`:** when a seal / install path needs a `Default` value whose type is already fixed by the bounds (e.g. `<B as BeanInfo>::State: Default`), write **`Default::default()`**, not `T::default()` / `<B as BeanInfo>::State::default()`. The type is inferred; spelling it again is noise.

   ```rust
   // Forbidden
   self.tick.install_config(DefaultInit { state: <B as BeanInfo>::State::default() })

   // Required — one `FreshInit::Config` GAT; the slot wrapper lives in the config
   self.tick.install_config(DefaultInit { state: Default::default() })
   ```

   **Target shape** for ordinary bounds (like root `Args` + `TakeMany` / `At`):

   ```rust
   // Preferred — flat type params + short trait bounds
   impl<A0, A1, M, T> Args<(A0, A1), T> for M
   where
       M: BeanInfo,
       M: At<A0> + At<A1>,
       T: TakeMany<M, (A0, A1)>,
   {
       fn init(self, state: T) -> (A0, A1) { state.take_many() }
   }
   ```

   Builder steps (also **rule 5**): **named step trait** + associated `Out`; inherent method returns `<Self as Step>::Out`. Bounds are **lists of traits**; next context is **`C::With<Cfg>`** / `ApplyPatch`, never a hand-built lens tower:

   ```rust
   trait EmitStep<T> {
       type Out;
       fn emit(self, tree: T) -> Self::Out;
   }

   impl<E, C, T> EmitStep<T> for Init<E, C>
   where
       Self: ApplyPatch<SetEmit<T>>,
       T: NestOk,
   {
       type Out = <Self as ApplyPatch<SetEmit<T>>>::Out;
       …
   }
   ```
10. **Generic names — one letter, or letter + digit** — a type parameter is **exactly one letter** taken from the role word (`B` bean, `A` args, `S` status, `T` tick, `E` event, `H` handler, `F` filter, `N` new, `O` old, …). For a position in a list, **that letter + index digit(s)** only: `E0`, `E1`, `S0` (two or more characters **only** when a digit is present).

   ```rust
   // Forbidden — multi-letter params; alphabet as bare arity numbering
   impl<Bean, Args, Status, Tick, S> CtxConfig<Ctx<Bean, Args, Status, Tick>> for SetStatus<S> { … }
   impl<New, A, B, C, D, E, F, G, H, I, J, K, L> PushEntry<New> for (A, B, …) { … }

   // Required — one letter per role; letter+digit for slots (≤ 8 — rule 11)
   impl<B, A, O, T, S> CtxConfig<Ctx<B, A, O, T>> for SetStatus<S> { … }
   impl<N, E0, E1, E2, E3, E4, E5, E6, E7> PushEntry<N> for (E0, E1, E2, E3, E4, E5, E6, E7) { … }
   ```

   **Forbidden:** multi-letter type params (`Bean`, `Args`, `Status`, `PrevE`, `Inner`, `Head`, …); `A, B, C, D, …` solely to count slots without a role letter. Same in docs examples and new code.
11. **Leaf list arity ≤ 8 — no long unrolled tables** — a single flat tuple (or one `impl` over that tuple) holds **at most 8** elements. Do **not** add arity-9…16 megaimpls that list `F0,H0,…,F14,H14` and one `if` per slot.

   ```rust
   // Forbidden — today’s query/run/fifteen.rs (and ten…sixteen)
   impl<F0, H0, /* … */, F14, H14> Run
       for (MapEntry<F0, H0>, /* … */, MapEntry<F14, H14>) { … }
   ```

   Past 8 items, **group** leaf tuples (rule 12). Trait recursion / one impl per **group shape**, not one impl per total length 1…16.

12. **List carriers and grouping** — a type-level list is only:

   - a **tuple of opaque element generics**: `(E0, E1, …)` with ≤ 8 slots, or
   - a **struct whose only payload is such a tuple**: `List` / `Leaf` / `Grow` (shared carriers below).

   **Forbidden:** ad-hoc list shapes (parallel type-param bags without a tuple, cons `MapNode` spines, parallel `F0,H0,F1,H1,…` outside entries); **`impl` heads on tuples of domain structs** (`(MapEntry<F0,H0>, MapEntry<F1,H1>, …)` — pull the element through an opaque `E0` / `N` instead).

   **Shared grouped list (reuse, not copy):** the leaf + group layout is **one common structure** for the whole crate — not a private regrouping inside `push`, another inside `query/run`, another for claims. Carriers live in `context/list.rs`; append lives in **one** `context/push.rs` (rule 13). Domain code **bounds** walk/push; it does **not** re-invent arity tables or group shapes per feature.

   ```rust
   struct List<I>(I);  // outer carrier; Items = Leaf, (List, Grow), or four Lists
   struct Leaf<I>(I);  // flat growth 0..=8 (coherence vs bare (E0,E1) ≡ (List, Grow))
   struct Grow<I>(I);  // second group 9..=15 (coherence vs reshape / List rest)

   impl<I: Walk> Walk for List<I> { … }
   impl<I, N> PushEntry<N> for List<I> where I: PushShape<N> { … }
   ```

   **Grouping when length > 8** (same shared layout everywhere; `MAX_MAP_SYSTEMS = 16`):

   | Items | Shape |
   | --- | --- |
   | 0…8 | `List<Leaf<(E0,…)>>` (empty: `List<Leaf<()>>`). |
   | 9…15 | `List<(List<(E0,…,E7)>, Grow<(E8,…)>)>` — head opaque `List<H>`, rest `Grow<R>`. |
   | 16 | **Four** `List` groups of four. |

   Same idea for 10…15: fill `Grow` without a leaf longer than 8; at 16 reshape to four groups. `Run` / `PushEntry` / schedule go through **`List`**, which walks groups then leaves — not one flat 16-slot impl and not a second parallel list type.

13. **Push: Args / TakeMany shape, one module** — append mirrors root `Args` + `TakeMany`: **one** thin `PushShape` (or `PushEntry`) blanket per **shape**, with length growth in a sealed arity helper (`AddItem`, role of `TakeMany`). Do **not** split push into `leaf.rs` / `group.rs` / `wide.rs` / `add.rs` — keep the table in **`context/push.rs`**.

   ```rust
   // Required — one blanket; growth in AddItem (sealed Cap6)
   impl<N, H, R> PushShape<N> for (List<H>, Grow<R>)
   where
       R: AddItem<N>,
   { … }

   impl<N, T> PushShape<N> for Leaf<T>
   where
       T: AddItem<N>,
   { … }

   // Forbidden — MapEntry (or other domain structs) in the arity head
   impl<F0, H0, F1, H1, …> PushShape<MapEntry<F, H>>
       for (MapEntry<F0, H0>, MapEntry<F1, H1>, …) { … }

   // Forbidden — bare tuple PushShape that overlaps (List, Grow) (E0119)
   impl<N, E0, E1> PushShape<N> for (E0, E1) { … }
   ```

   **Carriers for coherence:** `Leaf` for flat 0..=8; `Grow` for the growing second group. **`AddItem` is sealed** (only lengths 1..=6) so it cannot apply to a 7-tuple and collide with the 15→16 reshape. Opaque `E0`… / `N` only — no domain type in the push arity table.

14. **Meta traits are frozen** — [`BeanInfo`], [`EventInfo`], [`ProxyInfo`] (and any trait this rule later lists) hold **only** their declared associated types / consts. Do **not** add methods, associated types, supertraits, or default bodies to them without **owner approval**. Pipeline entry and steps live on **separate** traits (`Query`, `Setup`, `InjectResourcesStep`, …). Context tip at start lives on [`StartDesc`], not on `BeanInfo`.

   ```rust
   // Forbidden — growing BeanInfo with entry / tip spelling
   trait BeanInfo {
       type Provides; …
       fn query() -> Init<(), Ctx<Self, Live<()>, NeedInit, Pipe<…>>> { … }
   }

   // Required — meta stays data; Query + StartDesc own the tip
   trait BeanInfo { type Provides; type Injects; type Beans; type State; }
   trait StartDesc { type Ctx: Context<Bean = Self>; fn start_ctx() -> Self::Ctx; }
   trait Query { type Out; fn query() -> Self::Out; } // Out = Init<(), Ctx>
   ```

15. **One `_markers: PhantomData<…>` per struct** — unused type params share a **single** field, always named **`_markers`** (tuple inside when several params). Do not stack several phantoms, and do not name the field `event` / `_filter` / `_path` / …

   ```rust
   // Forbidden — several phantoms, or any name other than `_markers`
   struct Ctx<B, A, S, T> {
       tick: T,
       bean: PhantomData<B>,
       args: PhantomData<A>,
       status: PhantomData<S>,
   }
   struct Init<E, C> { context: C, event: PhantomData<E> }

   // Required
   struct Ctx<B, A, S, T> {
       tick: T,
       _markers: PhantomData<(B, S, A)>,
   }
   struct Init<E, C> { context: C, _markers: PhantomData<E> }
   struct MapList<N, E, S> { entries: N, _markers: PhantomData<(E, S)> }
   ```

   Marker-only ZSTs may still be a **tuple struct** whose only field is `PhantomData` (e.g. `Get<T>(PhantomData<T>)`, `SetFilter<F, E>(PhantomData<(F, E)>)`) — that unnamed field is the one phantom. As soon as the struct has a named phantom field, the name is **`_markers`**.

16. **No `unsafe`, no `Box<dyn …>`** — `unsafe` blocks, `unsafe fn`, `unsafe impl`, and `unsafe` traits are **forbidden**. **Owned** heap trait objects are **forbidden**: `Box<dyn Trait>`, `Box<dyn Trait + Send>`, and the same via type aliases or fields in `State` / tick storage. Prefer generics, enums, and trait bounds (static dispatch). Crate-level `#![forbid(unsafe_code)]` stays on.

   **Allowed:** temporary **borrowed** views `&dyn Trait` / `&mut dyn Trait` in inject `ArgsView` (see **Inject `&dyn`**, form `Bean::T as Trait`). Do not *own* or *store* a fat pointer; do not put `dyn` / casts in **provides**.

   ```rust
   // Forbidden
   unsafe fn poke(p: *mut u8) { … }
   struct Tick { systems: Box<dyn FnMut()> }
   struct State { draw: Box<dyn Draw> }

   // Required — typed handlers / generics (no dyn erasure in storage)
   struct WithInit<H> { handler: H } // H: Fn(…)
   // OK as a view only: ArgsView::View<'a> = &'a dyn Draw
   ```

17. **Shared bounds live on the trait head** — when every method (and the meaningful impls) need the same predicates, put them on the **trait** (`T: Bound` / `where` on the trait), not again on each `fn` and not as a copy-paste block on every impl beyond what that impl uniquely needs.

   ```rust
   // Forbidden — same bounds repeated on the method
   pub trait Update<M> {
       fn update(&mut self, injects: <M::Injects as ArgsView>::View<'_>)
       where
           M: BeanInfo,
           M::Injects: ArgsView;
   }

   // Required — bounds on the trait; method stays thin
   pub trait Update<M: BeanInfo>: Sized
   where
       M::Injects: ArgsView,
   {
       fn update(&mut self, injects: <M::Injects as ArgsView>::View<'_>);
   }
   ```

   Same for [`Tick`] / [`Setup`]: `M: BeanInfo` and `M::Injects: ArgsView` (or `Self: BeanInfo` + `Self::Injects: ArgsView` on `Setup`) sit on the trait. An `impl` only adds **extra** bounds that that impl needs (e.g. `C: Context`, `C::Tick: Drive` on `Update for Built`).

   **Runtime surface:** [`Update`] is implemented for **`Built`** only (ready tip after `.build()`). Do **not** stub `Update for Init` — `Init` is not a `Tick` (`PipelineOk`). `injects` on `update` is the **API contract** (typed inject view for `M`); the body may ignore it until maps/module tick consume it — that is not a reason to drop the parameter or to implement `Update` on the builder tip.

18. **Systems are pure — no captures; store as `fn`, not closures** — a system (init or map) may touch **only** its resource/inject args (and return init `State`). It must **not** capture, borrow, or mutate anything from the surrounding environment. Bound is **`Fn`**, never `FnMut` / `FnOnce` with captures.

   Prefer a **function item** at the call site (`fn system(…) { … }` / `.map(system)`). That is not a style preference: a capturing closure would break a future **SIMD / packed store** of systems (uniform callable layout, no per-handler environment). Under the hood, store systems as **pure function items / `fn` pointers** (or equivalent non-capturing callables), **not** as closure objects — capturing types also slow monomorphization of long `.map` chains.

   ```rust
   // Forbidden — captures / FnMut / env mutation
   let mut n = 0;
   .map(|| { n += 1; })
   .init_state(|_| { /* reads outer local */ … })

   // Required — function item; only args (and init return) change
   fn boot(args: (&Id, &mut Keys)) -> u8 { … }
   fn on_tick() { … }
   .init_state(boot)
   .map(on_tick)
   ```

19. **Do not change existing structures without prior owner approval** — public (and crate-visible API) types already in the tree — structs, enums, traits, and their **signatures** (fields, generics, associated items, inherent methods that define the shape) — stay as they are unless the **owner explicitly allows** the change first. Prefer a **new** type / wrapper / view (`Dyn<…>`, seal, config) beside the frozen one. Applies in particular to [`Get`] / [`Mut`] / [`Marker`], [`BeanInfo`] / [`EventInfo`], list carriers, and other landed API. Drive-by “small” renames, extra type params, or field reshuffles are **forbidden** without that OK (stronger than taste: same bar as rule 14 for practical review).

   ```rust
   // Forbidden without owner OK — reshape a landed type
   struct Get<T, V = ()>(…);          // was Get<T>
   impl<T> Get<T> { fn as_dyn<V>(…) → … } // growing Get with cast API

   // Required — leave Get/Mut; dyn cast is attribute / Injects encoding, not a Get method
   #[inject(Draw::T as Painter)]
   .inject_resources(Draw::T)   // same claim shape as any other bean
   ```

(Rules 8–9: extractors + `CtxConfig` / shallow assoc. Rule 10: role names. Rules 11–16: lists / push / meta / phantom / safety. Rule 17: trait-head bounds. Rule 18: pure systems. Rule 19: no silent API shape changes.)

### How pieces talk

```text
query::*  ──uses──►  descriptor::*  (Status, Accessors, Get/Mut, OpDesc)
    │                    ▲
    ├──uses──►  filter::*   (And/Or/Not/On, shape)
    ├──uses──►  context::*  (Ctx, Tick, List, CtxConfig + With<Cfg>)
    ├──uses──►  bean::*     (BeanInfo, DI)
    └──uses──►  borrow::*   (At, Ix*, Take)
```

Operators stay thin: `impl<E, C> Init<E, C> where C: Context` (rule 9); they forward `Descriptor` from `C` and apply **`configure(Cfg)`** / `ApplyPatch` — not `WithArgs`/`WithStatus`/`WithTick`.

---

## Modules (DI)

A **module** is a unit with `#[config(type = module)]`.

| Attribute | Role |
| --- | --- |
| `#[provide(…)]` | Beans this module owns / exposes downward. |
| `#[inject(…)]` | Beans taken from the parent (or the parent’s inject/provide surface). |

### Derive — only meta traits

Proc-macro / derive (when owner-allowed under rule 6) **emits only** these trait impls. Nothing else.

| Target | Emitted | Not emitted |
| --- | --- | --- |
| **Module** (`#[config(type = module)]`) | [`BeanInfo`], one [`At`]`<P>` per entry in `Beans` | `Query`, `Setup`, `StartDesc`, `Module`, `FromParent`, pipeline steps, … |
| **Event** | [`BeanInfo`] + [`EventInfo`] (`Parents`, `ON`); if `proxy = …`, also the proxied link (see **Proxy**) | same exclusion |
| **Proxy** (`#[config(type = proxy)]`) | [`BeanInfo`] + [`ProxyInfo`] (+ `At` for its `Beans` / provides) | bare inject of the proxy; pipeline steps |

Manual impls in the prototype must match this surface so derive is a drop-in.

**`BeanInfo` (module):**

| Assoc | Source |
| --- | --- |
| `Provides` | ordered list from `#[provide(…)]` — **concrete beans only** (no `dyn` / cast forms) |
| `Injects` | ordered list from `#[inject(…)]` — **concrete bean keys** (a cast `Bean::T as Trait` still records `Bean`, not the trait; see **Inject `&dyn`**) |
| `Beans` | **`Provides` then `Injects`** (stable concat of concrete keys). |
| `State` | module’s owned state type (field / attribute as the derive defines) |

**`At` (module):** for each concrete `P` at position `n` in `Beans`, emit `impl At<P> for Module { type Ix = IxN; }`. Membership for claims is `Provides ∪ Injects` (a cast inject still requires the parent to provide/inject that concrete `Bean`). Duplicate concrete `P` is a derive error.

**Event:** emit `BeanInfo` (typically empty `Provides` / `Injects` / `Beans` unless the event also declares them) + `EventInfo { type Parents = … }` from the event’s parent attribute(s). `const ON` stays the default on [`EventInfo`]. Do **not** generate `At` for an event unless that type has a non-empty `Beans` (same rule as modules).

```rust
// Module — derive emits only:
impl BeanInfo for App {
    type Provides = (Logic,);
    type Injects = ();
    type Beans = (Logic,); // Provides ++ Injects
    type State = …;
}
impl At<Logic> for App {
    type Ix = Ix0;
}

// Event — derive emits only:
impl BeanInfo for Update { type Provides = (); type Injects = (); type Beans = (); type State = (); }
impl EventInfo for Update {
    type Parents = (); // or (Main,) / (Main, Other)
}
```

**Forbidden for derive:** impls of `Query`, `Setup`, `Tick`, `StartDesc`, `Module`, `FromParent`, `Claim`, step traits, or any other non-meta API. Those stay hand-written blankets / user code.

### Proxy

A **proxy** is a thin bean wrapper around another bean (often an event). Declare the proxy type, then point the target at it. Derive name in examples may be `Component` / `Module` — the contract is `#[config(type = …)]`.

```rust
#[derive(Component)]
#[config(type = proxy)]
#[provide(/* proxy-owned surface */)]
struct Update;

#[derive(Component)]
#[config(type = event, proxy = Update::T)]
struct GameTick; // event (or other bean) wrapped by Update
```

| Piece | Role |
| --- | --- |
| `#[config(type = proxy)]` | This type **is** a proxy. Emits [`BeanInfo`] + [`ProxyInfo`]. May `#[provide]` / `#[inject]` like a module surface. |
| `proxy = Update::T` on the target | Target bean/event is **proxied by** `Update`. Marker strength follows `T` / `MUT`. |
| [`ProxyInfo`] | Meta (rule 14): data-only link proxy ↔ target. Associated types: `Targets` (the wrapped bean or the next inner proxy), `Link` (`Get<Self>` / `Mut<Self>`), `Chain` ([`Stop`] or [`More`] of the next outer proxy). Names pending owner OK. No methods. |
| [`DefaultProxy`]<`B`> | The proxy a bean or event uses when it has no `proxy = ...`. Ordinary proxy: `Targets = B`, `Link = Get<DefaultProxy<B>>`, `Chain = Stop`, no provides, no parent events. |
| [`Proxied`] | Manual stand-in for `proxy = ...` on a **bean** (not a reshape of [`BeanInfo`] / [`EventInfo`]). `Proxy` + `Link`. Absent attribute → `Proxy = DefaultProxy<Self>`, `Link = Get<DefaultProxy<Self>>`. A proxy does not write it: `Chain = More<Outer>` means `Proxy = Outer` and `Link = Outer::Link`. |

**Rules:**

1. **Cannot inject the proxy alone** — `Update::T` / `Update::MUT` as a claim / `#[inject(Update::…)]` / `.inject_resources(Update::T)` is a **compile error** (Descriptor / `FromParent` / claim gate). The proxy is not a free-standing inject key.
2. **Inject the target → proxy comes as wrapper** — injecting the ordinary proxied bean (`GameTick`, …) **defaults** to injecting through its proxy: the stored claim is a **wrapper** (proxy around the bean), not a bare target-only slot. A chain nests wrappers, outer around inner: `Via<Outer, Via<Inner, Get<T>>>`. Call sites still name the **bean** (`GameTick::T`), not the proxy. The system argument stays the target reference; each proxy forwards it.
3. **Target sees proxy providers** — a bean that has a proxy has access to that proxy’s **`Provides`**, and to the provides of every outer proxy in the chain (union into the target’s usable surface / `At` membership for those beans). The prototype checks that union on the claim gate (the pipeline bean's own [`At`], its direct proxy's [`At`], or an outer proxy's [`At`]). A blanket `impl At<S> for Target` is not expressible next to hand-written [`At`] impls (coherence).
4. **Emit from proxy → bean** — from the proxy you can **reach the proxied bean via emit** (proxy emit / schedule path activates the target). Exact emit sugar stays on the emit / event channel; the proxy does not replace `EventInfo::Parents`.
5. **Default proxy** — a bean or event with no `proxy = ...` uses [`DefaultProxy`]. Injection, init, and scheduling go through it the same way they go through any other proxy. It adds no provides and no parent events.
6. **Init goes through the proxy** — the init stage ([`Init`] / [`InitStateStep`] / `init_state` / `init_state_default`) of a proxied bean runs through its whole proxy chain, outer then inner. The stored handler is `Fwd<Outer, Fwd<Inner, H>>`. The direct proxy produces default state. [`Update`] stays on [`Built`] only (rule 17).
7. **Proxy events apply to the target** — [`EventInfo`], `Parents`, and emit/schedule wiring on any proxy in the chain also cover the proxied bean (an explicit proxy or [`DefaultProxy`]). This does **not** write those parents into the target's [`EventInfo::Parents`].
8. **Proxies are multi-level** — a proxy may itself declare `proxy = ...`. The chain is a finite list of wrappers, outer around inner, in the style of layered advice. Injecting any proxy in the chain alone is still a **compile error** (proxy types are not injectable beans). An explicit outermost proxy that names no further proxy ends the chain with `Chain = Stop`. [`DefaultProxy`] is not added outside that chain.

```rust
// Forbidden — inject proxy by itself
#[inject(Update::T)]
.inject_resources(Update::MUT)

// Required — inject the bean; proxy wraps by default
#[inject(GameTick::T)]   // wrapper: Update around GameTick
.inject_resources(GameTick::T)

// A chain nests outer around inner. The default proxy is not added outside it.
// Via<Outer, Via<Inner, Get<Bean>>>

// Target may use beans from Update’s #[provide(…)]
// From Update’s pipeline / emit path → activate GameTick
```

**Not a reshape of [`Get`] / [`Mut`]** (rules 19): wrapper encoding is a **side** type / claim projection (like `Dyn`), not new params on `Get`/`Mut`.

### Inject from the parent surface

A child may inject anything that appears in the parent’s **injects or provides**. The parent does not have to re-list every transitive bean for the child if it already holds it.

```rust
#[derive(Module)]
#[config(type = module)]
#[inject(Logic::MUT)]
struct Render;

#[derive(Module)]
#[config(type = module)]
#[inject(App::T)]
#[provide(Render::T)] // provide is support-only for the child graph
struct Logic;

#[derive(Module)]
#[config(type = module)]
#[provide(Logic::T)]
struct App;
```

Tree: `App` → `Logic` → `Render`. `Render` injects `Logic::MUT`; `Logic` injects `App::T` and provides `Render`.

### `FromParent` impl style

Compile-time “child may take this claim from parent” is **`FromParent`** (and `Module`). Use **`ClaimMark`** + membership on `Provides` / `Injects` — **not** Get×Mut×slot. `OfProvide0` / `OfProvide1` (and inject twins) are **E0119 residuals** only (head vs tail Via); collapse further if a non-overlapping encoding appears (rule 8).

### `T` vs `MUT`

Markers on provide/inject may be `T` (shared / immutable) or `MUT` (exclusive).

- An immutable provide (`::T`) may be injected as an immutable bean.
- Injecting `::MUT` against an immutable type / provide is a **compile error** (Descriptor / trait gate on that attribute use).
- Cast injects use the same markers on the **concrete** side: `Draw::T as Trait` / `Draw::MUT as Trait`.

```text
#[provide(Render::T)]   // OK as T; may be injected immutably
.inject(X::MUT)         // on an immutable X → error
```

### Inject `&dyn` (not in provides)

**`dyn` / cast forms do not appear in `Provides`.** Parents expose **concrete** beans only. The cast is declared on **inject**. The claim stays the ordinary concrete bean — **not** `Get::as_dyn` / a special claim API.

**Under the hood — only the final argument is `&dyn`.** Same model as ordinary Rust unsizing: the value is a concrete struct; the callee parameter is the trait object. Coercion happens at the call boundary, nowhere else (no stored fat pointer, no claim wrapper at the inject site).

```rust
trait Platform {}
struct Unix;
impl Platform for Unix {}

fn process(platform: &dyn Platform) { /* … */ }

fn main() {
    let platform = Unix;
    process(&platform); // concrete in; &dyn only on the parameter
}
```

Mapped onto beans: `Draw` is `Unix`; `Painter` is `Platform`; the system / init fn is `process`. Claim and storage stay `Draw`. **`ArgsView` is the only place that becomes `&dyn Painter` / `&mut dyn Painter`.**

**Declare the view on inject; inject the bean normally:**

```rust
#[config(type = dyn)]
trait Painter { fn paint(&self); }

impl<T: Painter> DynView<T> for dyn Painter {} // marker only (derive)

struct Draw;
impl Painter for Draw { … }

#[inject(Draw::T as Painter)]   // ArgsView → &dyn Painter (final arg only)
struct Ui;

fn boot(view: &dyn Painter) { view.paint(); } // like process(&dyn Platform)

// Call site — same claim as any other bean (Get/Mut unchanged, rule 19)
.inject_resources(Draw::T)      // concrete Draw, like `let platform = Unix`
.init_state(boot)               // framework passes &Draw as &dyn Painter
```

| Side | Form | Meaning |
| --- | --- | --- |
| Provide | `Draw::T` | Concrete only. **No** `as` / `dyn` in provides. |
| Inject attr | `Draw::T as Painter` | Same bean key `Draw`; **only** `ArgsView` / system arg is `&dyn Painter` (`::MUT` → `&mut dyn Painter`). |
| Claim / `.inject_resources` | `Draw::T` / `Draw::MUT` | **Ordinary** marker — same shape as `Keys::T`. **No** `.as_dyn(…)`. |
| Trait | `#[config(type = dyn)]` → `DynView` | Allows `… as Painter`. |

**`DynView` — marker only:** no `Shared` / `Exclusive` GATs, no coerce helpers on the marker. Unannotated traits cannot appear after `as`.

**Type-level (derive / encoding):** `Injects` may record the cast (e.g. thin `Dyn<Get<Draw>, dyn Painter>` **inside** assoc types) so `ArgsView` resolves to `&V`. Users never construct that at the call site — they keep writing `Draw::T`. `At` / `FromParent` / `Beans` / live claim stay **`Draw`**. **Do not** add methods on [`Get`] / [`Mut`] / [`Marker`] for the cast (rule 19). **Do not** put `Dyn` on filter ops until a `.filter` cast is required.

**Why `&dyn Trait` only on the arg:** under a cast inject, system / inject `ArgsView` is `(&dyn Painter, …)` — the “конечный аргумент”. Simple monomorphized paths **inline** — [`ph_test_asm`](../../../../crates/tests/ph_test_asm) `assert_no_virtual_calls()` on release probes. Not `Box<dyn>` (rule 16).

```rust
AsmTester::new("dyn_inject_probe.rs")
    .function("on_tick")
    .hot_loop(|hot| {
        hot.assert_no_virtual_calls();
    });
```

| Surface | Type-level |
| --- | --- |
| `#[inject(Draw::T as Painter)]` | `Injects` encodes cast; bean key `Draw` |
| `.inject_resources(Draw::T)` | same claim API as any bean |
| `ArgsView` (cast inject) | `&dyn Painter` / `&mut dyn Painter` |

**Forbidden:**

```rust
#[provide(Draw::T as Painter)]
#[provide(dyn Painter)]
#[inject(dyn Painter::T)]
Draw::T.as_dyn::<dyn Painter>()     // not a claim API; dyn only on ArgsView
impl Get<T> { fn as_dyn… }          // do not grow Get — rule 19
struct Get<T, V>(…);                // reshape Get
trait DynView<T> { type Shared<'a>; … }  // keep marker-only
struct State { p: Box<dyn Painter> }     // rule 16
```

Derive still emits only `BeanInfo` / `At` / `EventInfo` / `ProxyInfo` as listed, plus the **`DynView` marker impl** for `#[config(type = dyn)]` traits (and cast encoding in `Injects`).

---

## Shape (query operators)

Operators always take two type parameters:

```text
{Operator}<E, C>
```

| Param | Meaning |
| --- | --- |
| `E` | Current event scope (linear inverted spine — see Operator `E`). |
| `C` | Context implementing `Descriptor` (bean, args, tick, status). |

Examples: `Init<E, C>`, `Map<E, C>`, `Built<E, C>`. Prefer a **linear** `Ctx<Bean, Args, Status, Tick>` over recursive operator nesting.

## Operator description (`OpDesc`)

Operators also carry a **local** description trait for errors that are about the operator itself (before or besides the pipeline `Descriptor`). Same pattern as `Descriptor::Status`: named status types + `OpOk` / refuse the next step on the bad line.

```rust
pub trait OpDesc {
    type Status; // Valid | ErrDuplicateResource<T> | ErrConflictAccess<T> | …
}
```

Pipeline `Descriptor` remains the channel for the whole tip. `OpDesc` is the per-operator lens (claim list for `.inject_resources`, map args, …). A step may update both; module wiring always re-checks level 2.

**Same Status mechanism as the pipeline descriptor:** assemble claims → set a **named** `Status` (optionally parameterized by the bean, e.g. `ErrDuplicateResource<Id>`) → `.inject_resources` (and other gated steps) require `Status: OpOk` → compile error on that call, with the bean type visible in the diagnostic. Duplicates and `T`+`Mut` conflicts use this one pattern; only the status type differs. Missing beans use level 2 (`At` / `ClaimOn`) on the **same** `.inject_resources` line.

| Claims | Meaning | Example status |
| --- | --- | --- |
| `PointX::T & PointY::Mut` | different beans | `Valid` |
| `PointX::T & PointX::T` | duplicate shared | `ErrDuplicateResource<PointX>` |
| `PointX::T & PointX::Mut` | shared + exclusive on one bean | `ErrConflictAccess<PointX>` |

Without specialization, `A & A` (or `T & Mut` of the same bean) may still type-check at the `&` itself; the Status gate fires on **`.inject_resources`** (requires `OpOk`). Prefer parameterizing the error by the bean so the message names the field (`ErrConflictAccess<PointX>`).

### Two validation levels

| Level | When | What |
| --- | --- | --- |
| **1 — claim shape** | `.inject_resources(…)` | Duplicates, `T`+`Mut` on one bean, empty where forbidden → `OpDesc::Status` / no `OpOk` on that line. |
| **2 — module surface** | same `.inject_resources(…)` under bean `B` | Every claim covered by `B`’s **Provides ∪ Injects** (`At` / `ClaimOn`). Missing → error on the method bound. |

Checked with rustc: both levels work on the **same** call.

**Level 1 — bad claims** (`.inject_resources` requires `Status: OpOk`):

```text
error[E0277]: duplicate resource claim: `ErrDuplicateResource<Id>`
  --> … .inject_resources(Id::T & Id::T)
   = note: Id was claimed twice

error[E0277]: operator invalid: Status=`ErrConflictAccess<PointX>`
  --> … .inject_resources(PointX::T & PointX::Mut)
   = note: cannot claim T and Mut for the same bean
```

**Level 2 — claim not in Provides/Injects** (`Claim<B>` / `At`):

```text
error[E0277]: the trait bound `App: At<Other>` is not satisfied
  --> … .inject_resources(Other::T)
note: required for `Get<Other>` to implement `ClaimOn<App>`
```

Happy path:

```rust
// Level 1 + 2 OK — distinct beans on App’s At / Provides surface
App::query()
    .inject_resources(Id::T & Keys::Mut) // OpDesc::Status = Valid; Claim<App>
    .init_state(|(id, keys)| keys[id])
    .build();
```

### Universal impls via traits (not per-pair tables)

`Claim`, `ClaimOn`, `ArgsView`, accessor claim/drop, and similar checks are written **generically over traits**, the same way `Accessors` rules compose — not as a closed table of every pair shape.

Prefer:

```rust
impl<B, A0, A1> Claim<B> for (A0, A1)
where
    A0: Claim<B>,
    A1: Claim<B>,
{ /* Indexes = (A0::Indexes, A1::Indexes) or flattened */ }
```

over enumerating

```rust
impl<B, P0, P1> Claim<B> for (Mut<P0>, Get<P1>) where B: At<P0> + At<P1> { … }
impl<B, P0, P1> Claim<B> for (Get<P0>, Mut<P1>) where … { … }
// … every Get/Mut combination
```

Leaves (`Get<P>`, `Mut<P>`) carry the `B: At<P>` bound once; tuples/lists only require “each part implements `Claim` / `ArgsView` / …”. Dup and `T`+`Mut` conflict Status similarly compose over trait predicates on the claim set. That keeps arity and marker mixes open without specializing every `(Mut<_>, Get<_>)` line.

## Descriptor

```rust
pub trait Descriptor {
    type Status;    // Valid | warning | blocking — entire pipeline
    type Accessors; // type-level set of live accessors (Get/Mut, …)
}
```

Operators forward pipeline status from context. `setup` → `impl Tick` gates on:

```rust
where <Tip as Descriptor>::Status: PipelineOk<Tip>
```

### Status

| Kind | Examples | Result |
| --- | --- | --- |
| Ok | `Valid` | Compiles. |
| Warning | `WarnUnusedArgs`, … | Compiles + rustc warning on `.build()`. |
| Blocking | `NeedBuild`, `NeedInit`, `ErrInitAlreadySet`, `ErrBadEvent`, `ErrNeedWaitFilter`, `ErrTooManySystems`, … | Compile error. |

Warnings: specialized `#[deprecated]` `.build()` when `Status` is a warning type. `on_unimplemented` is for errors only.

### Accessors

`Descriptor::Accessors` is the **live** type-level set of resource markers along the chain (`()`, `Get<T>`, `Mut<T>`, tuples). Same composition style as `Claim`: trait-recursive, not a closed table of every `(Mut<_>, Get<_>)` pair.

**Frozen — `Get` / `Mut` / `Marker`:** these types and their public signatures **do not change**.

```rust
pub struct Get<T>(PhantomData<T>);
pub struct Mut<T>(PhantomData<T>);
pub trait Marker: Sized {
    const T: Get<Self>;
    const MUT: Mut<Self>;
}
```

No extra type params, no new fields, no rename, no wrapping (`Get<AsDyn<…>>`, `Get<T, Dyn<…>>`, …). Features that need another view use **`#[inject(Bean::T as Trait)]`** + normal `Bean::T` claims (see **Inject `&dyn`**), not methods or extra params on `Get`/`Mut`. Covered by **rule 19**.

**Model: persist + explicit wait.** Live claims survive a plain `.filter`. A wait filter such as `Main::ON[Keys::MUT]` **clears** that `Mut` claim. Read/read under different filters stays allowed.

#### `.inject_resources` (replace, not append)

One method: **`.inject_resources(A::T & …)`** (markers combined with `&`; single method, replace semantics).

Both classes of claim error can surface **on this call** (different channels, same span):

| Problem | Channel | Typical diagnostic |
| --- | --- | --- |
| Missing bean (not in Provides ∪ Injects) | Level 2: `Claim<B>` / `At` / `ClaimOn` bound on `.inject_resources` | `App: At<Other>` not satisfied |
| Duplicate / `T`+`Mut` inside the argument | Level 1: `OpDesc` Status (`ErrDuplicateResource<T>`, `ErrConflictAccess<T>`) — method gated on `OpOk` | Status name with bean type |

- The call **redefines** the resource set for the next `map` / step: it replaces the previous `.inject_resources` argument entirely; it does not accumulate with prior calls.
- **Before** replace: the new set is checked against **still-live** claims from earlier in the chain. If `Accessors` still holds `Mut<T>` and the new `.inject_resources` touches `T` again without a wait — refuse on that line.
- **After** a successful `.inject_resources(…)`: `Accessors` becomes that new set (replace).

**API shape (rules 5 + 9):** step trait with `Out` (e.g. `InjectResourcesStep<R>`); inherent `.inject_resources` returns `<Self as InjectResourcesStep<R>>::Out`. Do not write `Init<E, Ctx<B, Live<R>, S, Pipe<I, L>>>` (or `C::With<…>`) on the inherent signature.

#### Ops (traits, not per-pair tables)

| Op | When | Effect on `Accessors` |
| --- | --- | --- |
| `ClaimAcc<…>` | successful `.inject_resources(…)` | Replace live set with the argument (after conflict check vs prior live claims). |
| `WaitAcc<…>` | wait filter naming `Mut<T>` (or `Get` if needed) | Drop that live claim **before** the next `.inject_resources`. |
| plain `.filter` | no wait | **Unchanged.** |

#### Lattice / conflict rules

| Prior live | Next `.inject_resources` | Result |
| --- | --- | --- |
| `Get<T>` | again includes `Get<T>` | OK (parallel reads). |
| `Mut<T>` | any use of `T` without wait | Hard refuse (no `…Allowed` impl) + pipeline `Status = ErrNeedWaitFilter` (bean-parameterized when useful). |
| `Mut<T>` cleared by wait | `Get<T>` / `Mut<T>` under the new filter | OK — set redefined by the new `.inject_resources`. |

**Right line:** primary diagnostic on the offending `.inject_resources(…)`. Pipeline `Status` is the backup so `PipelineOk` still fails with a named status if reached.

**Vs `OpDesc` / `Claim`:** the `.inject_resources` argument is the full claim set for this step (`OpDesc` / `Claim<B>`). After success, `Accessors` holds that same set as live for filter / parallelism checks. Level 2 `At` / `ClaimOn` unchanged.

## Context

```rust
pub trait Context: Descriptor {
    type Bean;              // mandatory
    type Args;
    type Tick: ContextTick; // systems live here
}

/// Empty tip for `Query` — tip type is the context descriptor, not BeanInfo.
pub trait StartDesc: Sized {
    type Ctx: Context<Bean = Self>;
    fn start_ctx() -> Self::Ctx;
}
```

### `ContextTick` layout and caps

Systems are stored **only** on `Context::Tick` (not on operators). Layout:

| Slot | Role |
| --- | --- |
| **Init** | At most one: `NoSystems` → `WithInit<H>` or `WithDefault<S>` via `init_state` / `init_state_default`. Outside the map cap. |
| **Maps** | `.map` entries live in the **shared** `List` / `Leaf` / `Grow` carriers (rules 11–13): leaf ≤ 8, groups past 8 (9 → 8+1, 16 → four groups). Push is one `context/push.rs` table; `Run` / schedule reuse that `List` — not a private flat 16-slot table. |

```text
Ctx<Bean, Args, Status, Tick>
                         │
                         └─ Tick = (InitSlot, MapList)
                              InitSlot: NoSystems | WithInit<H> | WithDefault<S>
                              MapList:  List<Leaf<…>> → (List, Grow) → four Lists
                                        // NOT MapNode; NOT MapEntry arity heads; NOT push/*.rs split
```

Push / run / schedule go through the **shared `List`** (groups then leaves — rules 11–13). Cap refusal stays at total length 16. Do **not** encode capacity as `room_*`, arity-9…16 megaimpls, domain structs in push heads, or a second list layout beside `List`/`Leaf`/`Grow`.

**Hard cap:** at most **`16` map systems** per pipeline tip.

```rust
pub const MAX_MAP_SYSTEMS: usize = 16;
```

The 17th `.map(…)` does **not** get a push impl. Pipeline (or OpDesc) `Status` becomes a named blocking error, e.g. `ErrTooManySystems`, gated like other Status failures so the diagnostic lands on that `.map` line:

```text
error[E0277]: at most 16 map systems are supported (Status = ErrTooManySystems)
  --> … .map(|..| { … })
   = note: MAX_MAP_SYSTEMS = 16; split the pipeline or drop a map
```

Prefer `on_unimplemented` / `PipelineOk` (or `OpOk`) naming `ErrTooManySystems` and the cap — same Status channel as `NeedBuild` / `ErrInitAlreadySet`. Counting is type-level (length of `MapList`); no runtime check.

Init remains a separate single slot: a second `init_state` is still `ErrInitAlreadySet`, independent of the map cap.

## Setup and per-module tick

`setup` runs the builder (query → `.inject_resources(…)` / `init_state` or `init_state_default` → build). Object init runs **inside** that path — not as a public `tick.init()`.

Two behaviours for state init on the builder:

1. **No `Default` on state** — tip requires `.init_state(…)`.
2. **`Default` available** — may use `.init_state_default()` instead.

`setup` returns a ready `Tick`. The **only** public runtime call is `Tick::update` with inject refs (`Get` vs `Mut` as claimed). There is **no** `boot` / `tick.init` — those were prototype names; init stays inside `setup`, runtime is `update` only.

Bounds for that surface sit on the traits (rule 17): `Update<M: BeanInfo> where M::Injects: ArgsView`, `Tick<M: BeanInfo> where …`, `Setup: BeanInfo where Self::Injects: ArgsView`. `Update` is only for `Built` (not `Init`).

```rust
let tick: impl Tick<Render> = Render::setup(); // builder finished; init already applied inside
tick.update(injects: /* … */); // only #[inject] refs; Get vs Mut distinguished
```

Each module can be **tested alone** without the full app tree: pass only that module’s injects into `tick.update(…)`. The `injects` argument is required by the contract even if a given tip still only drives maps and does not read the refs yet.

---

## Diagnostics (right line)

| Mistake | Span | Via? |
| --- | --- | --- |
| Illegal step (e.g. 2× `init_state`) | That call | Pipeline `Descriptor`. |
| 17th `.map` (`ErrTooManySystems`) | That `.map` | Pipeline `Descriptor` / `OpDesc` (`MAX_MAP_SYSTEMS = 16`). |
| Duplicate / `T`+`Mut` on one bean | `.inject_resources` | **`OpDesc` Status** (e.g. `ErrDuplicateResource<T>`, `ErrConflictAccess<T>`). |
| Claim not in Provides∪Injects | `.inject_resources` under bean | **`At`/`ClaimOn`** (level 2). |
| No `.build()` (`NeedBuild`) | `setup` → `impl Tick` | Pipeline `Descriptor`. |
| `MUT` inject of immutable provide | Attribute / inject site | DI / Descriptor. |
| Bad / incomplete event / bad parent edge | `.filter` / `.emit` / `setup` | Pipeline `Descriptor` (`ErrBadEvent`). |
| Accessor conflict across filters | Offending `.inject_resources` / `.filter` | `Accessors` + Status. |
| Warning status | User `.build()` | Pipeline `Descriptor`. |

```rust
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `{Self}` (tip `{Tip}`)",
    label = "fix Descriptor::Status",
    note = "one Status for the whole chain (build, events, accessors, DI)"
)]
pub trait PipelineOk<Tip> {}
impl<Tip> PipelineOk<Tip> for Valid {}
impl<Tip> PipelineOk<Tip> for WarnUnusedArgs {}
```

### Accessor conflict (checked)

```text
.filter(Update::ON[Graphics::ON]) // E = (Graphics, Update)
.inject_resources(Keys::MUT)              // Accessors = Mut<Keys>
.map(|..| {})
.filter(Update::ON[Main::ON])     // E = (Main, Update); Accessors unchanged — Mut still live
.inject_resources(Keys::T)                // refuse: Mut<Keys> still live
```

Hard refuse on that `.inject_resources` line (no `…Allowed` impl). Also set `Status = ErrNeedWaitFilter` so `PipelineOk` names the failure.

```text
.filter(Update::ON[Graphics::ON])        // E = (Graphics, Update)
.inject_resources(Keys::MUT)
.map(|..| {})
.filter(Update::ON[Main::ON[Keys::MUT]]) // E = (Main, Update); WaitAcc: drop Mut<Keys>
.inject_resources(Keys::T)                       // OK — Accessors redefined to Get<Keys>
.map(|..| {})
```

Wait clears the mut claim; read/read without a prior mut stays parallel.

---

## Events

### Purpose

Multithreading, GPU, network, and other callback-style scheduling. A system runs when its event is emitted and filters pass.

### Filters and predicate AST

Surface syntax uses only `&`, `|`, `!` — not comma lists. Under the hood the type is an **explicit AST** (not anonymous ops alone):

| Node | Meaning |
| --- | --- |
| `And<A, B>` | `A & B` |
| `Or<A, B>` | `A \| B` |
| `Not<A>` | `!A` |
| `On<Ev, Inner>` | `Ev::ON[Inner]` (event nesting) |
| leaf marker | `Get<T>` / `Mut<T>` / `T` filter bean / wait (`Keys::MUT`) |

`BitAnd` / `BitOr` / `Not` on the public markers build these nodes (ops for UX; named AST for rules). **Impl style (rule 8):** extractor / descriptor traits on the RHS so each LHS kind has **one** `BitAnd` / `BitOr` / `Not`, not a Get×Mut×And×Or×On matrix. Example shape:

```rust
trait FilterOperand {} // Get/Mut/And/Or/Not/On implement this once each

impl<L, R: FilterOperand> BitAnd<R> for L where L: FilterOperand { type Output = And<L, R>; … }
// or: impl only on Get/Mut/On leaves + one impl for any already-built node type
```

Validation and emit walk **traits over the AST** (`Claim`-style), not per-pair op tables. Filter `&`/`|`/`!` use `FilterOperand` on the RHS; per-LHS `BitAnd` arms remain only because of orphan E0210 (rule 8).

#### Filter effect descriptor

`.filter` changes two projections: **event scope `E`** and **live args** (wait). Today’s `filter/apply.rs` repeats the same `ApplyFilter` body for every AST node — that matrix is **out of design**.

**Target:** each filter node implements a small descriptor once; the step is **one** blanket.

```rust
/// Per-node policy for `.filter` (not pipeline `Descriptor` / claim `OpDesc`).
trait FilterDesc {
    type Mut;   // wait flag: Yes | No  (same role as HasMut::Mut)
    type Scope; // Keep | Replace<Path>
}

trait ScopeOf<PrevE> {
    type NextE;
}

struct Keep;
struct Replace<Path>(…);

impl<PrevE> ScopeOf<PrevE> for Keep {
    type NextE = PrevE;
}
impl<PrevE, Path> ScopeOf<PrevE> for Replace<Path> {
    type NextE = Path;
}

impl<PrevE, Args, F> ApplyFilter<PrevE, Args> for F
where
    F: FilterDesc,
    F::Scope: ScopeOf<PrevE>,
    Args: Rewait<F::Mut>,
{
    type NextE = <F::Scope as ScopeOf<PrevE>>::NextE;
    type NextArgs = <Args as Rewait<F::Mut>>::Out;
}
```

| Node | `Mut` | `Scope` |
| --- | --- | --- |
| `()`, `Get`, `Mut`, `And`/`Or`/`Not` | from `HasMut` (or inlined) | `Keep` |
| `On<Ev, Inner>` | from inner | `Replace<Spine::Path>` (+ `NestOk` on that node) |

`HasMut` / `Spine` / `NestOk` stay as the recursive walks that *compute* the policy. `ApplyFilter` must not re-dispatch per node. Macro helpers such as `keep_scope!` go away with this collapse.

This is **not** claim-list `OpDesc` (`Valid` / `ErrDuplicateResource`). Same *idea* (descriptor + one step), different lens.

#### Where else the same pattern applies

Use a per-node (or per-leaf) descriptor + one blanket whenever the code today is “same associated types / same body, copy-pasted across AST / Live×Idle / Get×Mut”:

| Area | Today | Collapse toward |
| --- | --- | --- |
| `filter/apply.rs` | `ApplyFilter` × every AST node (+ `keep_scope!`) | `FilterDesc` + `ScopeOf` + one blanket (above) |
| `filter/ops.rs` | Get×Mut×node product | RHS `FilterOperand`; per-LHS ops only (orphan E0210); **no** `ops_node` |
| `filter/flag.rs` | `(Yes\|No)×(Yes\|No)` `OrFlag` | **`OrWith` on the left flag** |
| `bean/inject.rs` | Get×Mut×slot | `ClaimMark` + Via **only** for E0119 head/tail |
| `query/args.rs` `Claim` / `ArgsView` | near-duplicate walks over `()` / `Get` / `Mut` / `And` / `Live` / `Idle` | shared claim-shape descriptor; step blankets over it |
| `descriptor/op_desc.rs` `OpLeaf` | parallel walk of the same claim nodes | stay aligned with that claim-shape descriptor (one Status lens, not a second matrix) |
| `descriptor/live.rs` `Rewait` | `Live`/`Idle` × `Yes`/`No` table | **`WaitFlag` on `Yes`/`No`** + one blanket for `Live`, one for `Idle` |
| `filter/eval.rs` | `Collect` / `MatchEmit` / `UnionEmit` per node | **keep** per-node *behavior* when semantics differ; factor only shared “fold over AST” scaffolding if several traits share the same skeleton |
| `filter/shape.rs` `HasMut` / `SpineTail` / `Under` | recursive policy walks | **keep** — these *are* the descriptors’ inputs; do not replace them with another ApplyFilter-style matrix |

Rule of thumb: **policy on the node, one impl for the step.** Forbidden: step matrices and macros that only hide those matrices.

```rust
.filter(Update::ON[Valid::T & Alive::T])  // On<Update, And<Get<Valid>, Get<Alive>>>
.filter(Update::ON[Graphics::ON])         // On<Update, On<Graphics, ()>>  (or empty inner)
.filter(Update::ON[Main::ON])
```

```text
Event::ON[Event::ON & Filter & Filter & …]
EVENT::ON & PR::MUT | PR1::MUT
!SomeFilter & Other::T
```

Leaves that are wait markers participate in `WaitAcc`; event `On` spines update `E`; predicate-only trees leave `E` unchanged (see Operator `E`).

### Parallelism and resources

Maps under different filters may run in parallel. Shared read is fine. After `Keys::MUT`, a later `Keys::T` under another filter needs a wait, e.g. `.filter(Update::ON[Main::ON[Keys::MUT]])`, then sequential relative to the mutator.

### Operator `E`

`E` is the operator’s **current event scope**: a linear **inverted** path of event types only (for hints / diagnostics). Innermost event first.

| Shape | Meaning |
| --- | --- |
| `()` | No event scope yet (`query()` start). |
| `Ev` | Single event. |
| `(Innermost, …, Outermost)` | Spine from nested `::ON[…]`, events only. |

Examples:

| Filter argument | Resulting `E` |
| --- | --- |
| `Update::ON[Graphics::ON]` | `(Graphics, Update)` |
| `Update::ON[Main::ON[Keys::MUT]]` | `(Main, Update)` — `Keys::MUT` is wait, not part of `E` |
| predicates only, e.g. `Valid::T & Alive::T` (no event in the filter) | **`E` unchanged** |

#### `.filter` → `E`

- **Filter contains an event spine** (`Event::ON[…]` nesting): **replace** `E` with that inverted spine (new scope).
- **Filter has no event** (predicates / `&|!` only, or otherwise no `Event::ON`): **`E` stays the same**.

Wait markers inside the same `.filter` (e.g. `Main::ON[Keys::MUT]`) still drive `WaitAcc` / Accessors; they do not lengthen `E`.

Bad / incomplete event path → pipeline `Status = ErrBadEvent` (same Descriptor channel).

### Emit and `EventInfo::Parents`

Derive (or manual impl) gives each event **only** `BeanInfo` + `EventInfo { type Parents = … }` (`()` / one parent / tuple of parents) — see **Derive — only meta traits**. No other event traits.

**Compile-time rules** (Descriptor / trait bounds on the emit or filter line):

| Rule | Check | On failure |
| --- | --- | --- |
| Nested `On<Child, …>` under `On<Parent, …>` | `Child: EventInfo` and `Parent` appears in `Child::Parents` (trait-recursive over the `Parents` list) | `ErrBadEvent` / no `EmitAllowed` on that `.filter` / `.emit` |
| `.emit(tree)` | AST is well-formed (`And`/`Or`/`Not`/`On`/leaves); every `On<Ev, _>` event is an `EventInfo`; parent edges hold | same Status / refuse |
| `.emit(…)` under module | Emitted events covered by module surface where required (Provides ∪ Injects / providers) | level-2 style bound |
| `emit_providers()` | Emit the module’s provider events only | refuse if none / not a provider module |

```rust
.emit(Update::ON[Async::ON & Predicate::1])
// On<Update, And<On<Async, ()>, PredLeaf>> — Async must list Update (or a path) in Parents

emit_providers()
```

Parent check is **required** for nesting and emit (not optional). Multi-parent events: any one listed parent in the current outer `On` is enough for that edge; full path vs `E` remains a hint.

### Runtime selection (multi-level emit)

Events form a **tree** (parents + nested `On`). Runtime does **not** enter an event scope unless that event was **emitted**.

| Rule | Meaning |
| --- | --- |
| No emit → no enter | A system under `.filter(…::ON[Ev::ON…])` runs only if every event on that path was emitted for this tick. |
| Multi-level | Outer level first: parent/root must be active; then each nested child is considered only if **it** was emitted. Missing emit at a level **skips that subtree** — no jump *into* the missing event. |
| Siblings / other branches | Other emitted branches at the same level still run (independent subtrees). Skipping one child does not invent a transition into it. |
| `And` in filters | All conjuncts must pass (events emitted + predicates). |
| `Or` in filters | Alternative **filter** arms (match if any arm passes); not a “fallback jump into an un-emitted event”. Un-emitted events still never become the current scope. |
| `Not` | Negation of a leaf/subtree match; still cannot enter an event that was not emitted. |

```text
emit Update
emit Main          // Graphics not emitted this tick

.filter(Update::ON[Graphics::ON])  // skip — Graphics not emitted
.map(…)

.filter(Update::ON[Main::ON])      // run — Update and Main emitted
.map(…)
```

**Interrupt / “прыжок”:** leaving or skipping a level means **not scheduling** maps under the non-emitted node; control continues with other matching emitted paths in the tree. There is no silent transition onto an event that nobody emitted.

Compile-time graph/accessors still go only through `Descriptor` / `OpDesc`. Beans hold state; emit updates masks; the scheduler walks the tree with the rules above.

---

## Usage

### Query (bean systems)

```rust
#[derive(Module)]
#[module(state = u8)]
#[provide(Id::T, Keys::T)]
struct App;

impl Setup for App {
    fn setup(self) -> impl Tick<Self> {
        App::query()
            .inject_resources(Id::T & Keys::Mut)
            .init_state(|(id, keys)| keys[id])
            // or .init_state_default() when State: Default
            .build()
    }
}
```

### Event-oriented chain (sketch)

```rust
// query() → E = ()
.filter(Update::ON[Graphics::ON])           // E = (Graphics, Update)
.inject_resources(Keys::MUT)
.map(|..| {})
.filter(Update::ON[Main::ON[Keys::MUT]])  // E = (Main, Update); WaitAcc drops Mut<Keys>
.inject_resources(Keys::T)
.map(|..| {})
.build()
```

### Isolated module test

```rust
let tick: impl Tick<Render> = Render::setup();
tick.update(/* only Render’s #[inject] refs */);
```

## System callbacks

Stored inside `Context::Tick` only (see `ContextTick` layout). Rule **18**.

**Handlers are `Fn`, not `FnMut` / capturing closures.** A system does not take or change anything outside its args: only bean/resource (and inject) views, plus the returned init `State`. No captures, no env mutation. Callables must work through shared `&self` of handler storage — no exclusive `&mut self` on the system itself.

**Call site:** prefer a **function item** (`.map(on_tick)`, `.init_state(boot)`). Captures are forbidden not as taste, but because a future **SIMD / packed system store** needs a uniform non-capturing layout.

**Under the hood:** keep systems as **pure `fn` / function items**, not closure objects — faster builds (less monomorphization pressure on long map chains) and store-ready.

```rust
fn (args: (&Id, &mut Keys)) -> u8   // InitState — Fn / fn item, single init slot
fn (args: (&Id, &mut Keys))         // MapArgs  — Fn / fn item, one of ≤ 16 map slots
// not FnMut, not capturing closure
```

# Error codes

Pipeline and operator failures are zero-sized types with a numeric code. Workflow markers (`Valid`, `NeedInit`, `NeedBuild`, `WarnUnusedArgs`) stay named and are not in this table.

| Type | Code | Name | Meaning |
| --- | ---: | --- | --- |
| `FilterError` | 1 | bad event | Bad or incomplete event / emit path (nested `On`, parents, …). |
| `FilterError` | 2 | need wait | A live `Mut` is still held; the next `.inject_resources` needs a wait filter. |
| `FilterError` | 3 | not used | Was the child check of the removed in-system direct emit. The code is not reused. |
| `FilterError` | 4 | bad event bit | An event in `.filter`, `.emit`, `Tick::emit` or `Tick::defer` has no `EmitBit`, is outside the module's `EventReg::Reg`, or shares a path with an event of another registry. |
| `FilterError` | 5 | slot not owned | `EmitBit::Slot` names a slot with no `SlotOf` owner in that registry. |
| `FilterError` | 6 | not declared | `Tick::defer` named an event the pipeline's `.emit` does not declare. A `const` assert: reported by `cargo build`, not by `cargo check`. |
| `FilterError` | 7 | no event buffer | A declared `.emit` or a `Tick::defer` names a scope event the module does not hold under `Mut`, or whose state does not hold an `EventBuf` (`DeferState`). |
| `PipelineError` | 1 | init already set | `init_state` ran twice. |
| `PipelineError` | 2 | too many systems | More than `MAX_MAP_SYSTEMS` (16) `.map` calls. |
| `PipelineError` | 3 | no default | A provided bean or its state has no `Default`, so the module cannot create its slot. |
| `PipelineError` | 4 | not in reach | A system claims a bean it cannot borrow: `Mut` of a shared inject (plain or `Dyn`), `Mut` of a bean provided under `Get`, or a bean the module's `Beans` does not hold at its `At` index. |
| `PipelineError` | 5 | no marker | An entry of `Provides` is a bare bean. A provide names its access: `Get<P>` or `Mut<P>`. |
| `ClaimError<T>` | 1 | duplicate | Bean `T` named twice with the same access in one claim list. |
| `ClaimError<T>` | 2 | conflict | Bean `T` named shared and exclusive in one claim list. |

Use the type with the code, for example `FilterError<1>` or `ClaimError<Id, 1>`. Codes are unique **within** each type family, not across families.

Two `SlotOf` impls for one slot of one registry are a coherence error of the compiler (`E0119`), and an `EmitBit::Slot` owned by another event is a type mismatch (`E0271`). Those two have no code in this table.

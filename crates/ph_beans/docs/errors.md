# Error codes

Pipeline and operator failures are zero-sized types with a numeric code. Workflow markers (`Valid`, `NeedInit`, `NeedBuild`, `WarnUnusedArgs`) stay named and are not in this table.

| Type | Code | Name | Meaning |
| --- | ---: | --- | --- |
| `FilterError` | 1 | bad event | Bad or incomplete event / emit path (nested `On`, parents, …). |
| `FilterError` | 2 | need wait | A live `Mut` is still held; the next `.inject_resources` needs a wait filter. |
| `FilterError` | 3 | not child | Direct emit named an event that is not a child of the current one. |
| `PipelineError` | 1 | init already set | `init_state` / `init_state_default` ran twice. |
| `PipelineError` | 2 | too many systems | More than `MAX_MAP_SYSTEMS` (16) `.map` calls. |
| `ClaimError<T>` | 1 | duplicate | Bean `T` claimed twice in one list. |
| `ClaimError<T>` | 2 | conflict | Shared and exclusive claims on bean `T` in one list. |

Use the type with the code, for example `FilterError<1>` or `ClaimError<Id, 1>`. Codes are unique **within** each type family, not across families.

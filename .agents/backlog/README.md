# Backlog

What is still to be done in and around SolidEuclid: defects, hardening and follow-ups, one row per
item. The backlog holds open work only. An item leaves it when its fix is in the code.

Each item carries the problem as the code is **today** with `file:line` references, its cause where
it is known, the proposed change and the test that guards it. It never carries status history,
"found while …" notes, investigation narratives or the record of a failed attempt.

## Open items

| Item                                                               | To do                                                                                                                                                                            | Priority |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| [Model transformations](./transformations.md)                      | `transform.rs` exists with the `Transformation` enum and `Transform<T>` wrapper, but the bridges cannot deserialize the generic `Transform<T>` from JSON and no examples or bridge functions exist | high     |
| [CSG operations (union, difference, intersection)](./csg-operators.md) | The kernel has no boolean operations; the plan is a `csg` module behind the `GeometryModel` contract, developed on `feat/csg`. Define the operation set, the tolerance model and the per-layer exposure (Rust core first, then the C and web bridges) | high     |

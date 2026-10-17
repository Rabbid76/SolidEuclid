# Refactoring Analyses

One document per refactoring, written **before** the work and carrying the report once the work is
done. The analysis and the report are the same document — the report is appended at close-out, not
written into a separate file.

Triggered by **"analyse the refactoring"** / **"analyze the refactoring"** /
**"refactoring analysis"** — see **Analysis Documents** in
[`../../AGENTS.md`](../../AGENTS.md#analysis-documents).

## What belongs here

The analysis must cover:

- what the current code does and why it is a problem
- the full scope of the change, with file references for every affected location
- the proposed target shape
- the tests covering the affected behaviour and the output changes to expect
- for a performance refactoring, the benchmark to run before and after

## What does not belong here

A general algorithm write-up — how a pattern, a protocol, or a data structure works, independent of
any one refactoring — belongs in [`../../README.md`](../../README.md) or the skill files under
`.agents/skills/`.

## Close-out

When the refactoring is complete, append the report to this same document with:

- Status set to `Done`
- Pre-work sections rewritten into past tense
- Report appended with:
  - Summary of changes
  - Changed files
  - Before/after comparison
  - Test adaptations
  - Risks and open items
  - Performance measurements (if applicable)

Once the work is on `main`, the durable outcome is promoted and the document deleted.

## Current Documents

| Document | Status | Description |
| -------- | ------ | ----------- |

Add refactoring analysis documents as needed following the naming convention:
`kebab-case-description.md` (e.g., `sweep-model-simplification.md`).

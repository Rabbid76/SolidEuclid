# Bug Analyses

One document per bug analysis: a deep root-cause investigation written **before** the fix, closed
out **after** it.

Triggered by **"analyse the bug"** / **"analyze the bug"** / **"bug analysis"** — see
**Analysis Documents** in [`../../AGENTS.md`](../../AGENTS.md#analysis-documents).

## What belongs here

Point-in-time, investigation-driving write-ups. The document must cover:

- symptom and reproduction steps
- investigation trace with code references
- the exact root cause (file:line)
- why the current code produces wrong results
- the proposed clean fix

## What does not belong here

A symptom-level fix or workaround. All bug fixes must address the root cause, not hide the symptom.

## Close-out

When the bug is fixed, the durable outcome (the invariant, the rule, the corrected understanding)
is promoted into the appropriate living-reference document, and this document is closed out with:

- Status set to `Fixed`
- Pre-work sections rewritten into past tense
- Fix summary and validation results

Once the fix has landed on `main`, the durable outcome is promoted and the document deleted. A
solved bug keeps no analysis.

## Current Documents

| Document | Status | Issue |
| -------- | ------ | ----- |

Add bug analysis documents as needed following the naming convention:
`kebab-case-description.md` (e.g., `wrong-normal-on-conical-frustum-cap.md`).

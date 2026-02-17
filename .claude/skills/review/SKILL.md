---
name: review
description: Multi-agent code review producing a parallelized fix plan with impact analysis
disable-model-invocation: true
---

Run a full review pass on the current changes in arachne.

## Step 0: Detect Review Context

Check git log for previous review-related commits. If this is a follow-up review pass, note prior changes before suggesting reversals.

Determine what to review:
- If there are uncommitted changes: review those
- If $ARGUMENTS contains a commit range or branch: review that
- Otherwise: review the full `src/` codebase

## Step 1: Parallel Review Agents

Launch these 3 subagents **in parallel**:

1. **consistency-reviewer** — duplicated logic, pattern divergence, missed utility reuse
2. **solid-reviewer** — SOLID violations, clean code, structural issues
3. **defensive-reviewer** — unsafe unwraps, swallowed Results, panic paths

Each reviewer gets the current diff context. Wait for all 3 to complete.

## Step 2: Automated Checks

Run in parallel:
- `cargo test` — full test suite
- `cargo clippy -- -D warnings 2>&1` — lint check
- `cargo fmt --check 2>&1` — format check

Capture all output.

## Step 3: Reconcile & Dependency Analysis

Merge all reviewer findings. For each finding:

1. **Identify affected files and functions** — what code needs to change
2. **Map dependencies** — what other code calls/uses the affected code
3. **Detect conflicts** — do any two fixes touch the same code? Do they contradict?
4. **Assess breakage risk**:
   - Does the fix change a public function signature? → HIGH risk
   - Does it change internal logic only? → LOW risk
   - Does it change a type used across module boundaries? → MEDIUM risk

When reviewers conflict:
- Defensive "must fix" items always win
- Prefer DRY (consistency) over structural purity (solid)
- If a consistency fix and a solid fix touch the same code, merge into one refactoring step

## Step 4: Build Fix Plan

Organize all findings into an **ordered, parallelized execution plan**:

### Grouping Rules
- Fixes touching **independent files/modules** → group into a parallel batch
- Fixes touching the **same file** → sequence them (earlier fix first)
- Fixes that **change a public API** used by other fixes → must run before dependents
- **Modularization suggestions** (extracting new modules from large files) → always last, after simpler fixes

### Plan Structure

For each fix item include:
- **ID**: sequential number
- **Reviewer**: which agent flagged it
- **Severity**: must-fix / should-fix / consider
- **Description**: what to change and why
- **Files**: affected files
- **Depends on**: IDs of fixes that must be applied first (empty if independent)
- **Breakage risk**: LOW / MEDIUM / HIGH + explanation
- **Test coverage**: whether existing tests cover this code (yes/no/partial)

### Parallel Batches

Group fixes into execution batches:
```
Batch 1 (parallel): [Fix 1, Fix 2, Fix 3] — independent, no shared files
Batch 2 (parallel): [Fix 4, Fix 5] — depend on Batch 1 items
Batch 3 (sequential): [Fix 6] — modularization, depends on earlier fixes
```

### Breakage Flags

For any fix with HIGH breakage risk or that changes cross-module types:
- Mark it with `⚠ NEEDS DECISION`
- Explain what breaks and suggest alternatives
- If modularization makes sense, offer it as the alternative — describe the new module structure

## Step 5: Write Review Report

Write a markdown report to `.reviews/YYYY-MM-DD-HHmm.md`:

```
# Review Pass

**Date**: YYYY-MM-DD HH:mm
**Scope**: [what was reviewed]
**Reviewers**: consistency, solid, defensive

## Automated Check Results
- cargo test: PASS/FAIL (N tests)
- cargo clippy: PASS/FAIL (N warnings)
- cargo fmt: PASS/FAIL

## Fix Plan

### Batch 1 (parallel)
| ID | Severity | Description | Files | Risk | Tests? |
|----|----------|-------------|-------|------|--------|
| 1  | must-fix | ... | ... | LOW | yes |

### Batch 2 (parallel, depends on Batch 1)
...

### Batch N (sequential — modularization)
...

## Needs Decision
- [Fix ID]: [what breaks + alternatives + modularization option]

## Modularization Opportunities
- [description of extraction, new module, interface]

## Next Pass Recommended?
[yes/no + reasoning]
```

## Step 6: Summary

Present concise summary:
- Total findings by reviewer and severity
- Number of parallel batches
- Items needing user decision
- Modularization opportunities found
- Whether another pass is recommended after applying fixes

**IMPORTANT**: Do NOT apply any changes. The output is a plan only. The user decides what to execute.

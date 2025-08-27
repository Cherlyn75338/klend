### 08 — Execution Cadence

Schedule tuned to produce actionable findings quickly, leveraging 01–07 artifacts.

- **Day 1–2: Baseline + Invariant Draft**
  - Build, test, extract handler maps and pre/postconditions.
  - Draft invariants from 03_high_risk_areas and 05_instruction_checklist.

- **Day 3–5: Property Tests + Numeric Analysis**
  - Implement fuzz/property harness with conservation/monotonicity/LTV/caps/oracle checks.
  - Perform fixed-point range analysis and rounding audits.

- **Day 6–7: Oracle/Economic Attacks + Admin Hardening**
  - Stale-price scenarios, flash sequences, stale exchange-rate combos, liquidation bonus edges.
  - Admin/pending-admin lifecycle and config bounds tests.

- **Day 8: Consolidation**
  - Findings, POCs, remediations, and regression tests finalized.

Dependencies
- Continuous feedback from roles (06_roles_plan.md) and evolving invariants (10_invariants.md).



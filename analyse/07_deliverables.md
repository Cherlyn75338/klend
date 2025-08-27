### 07 — Deliverables

Deliverables mapped to artifacts produced in 01–06 and later testing.

- **Invariants document and mapping**
  - Built incrementally from 01_architecture_map, 03_high_risk_areas, and 05_instruction_checklist.
  - Final version consolidated in 10_invariants.md with per-instruction bindings.

- **Fuzz harness and targeted tests**
  - Property tests covering conservation, monotonicity, LTV safety, price freshness, caps, and flash repay exactness.
  - Seeds and minimization scripts; CI workflow for nightly runs.

- **Vulnerability findings**
  - For each: severity, affected files/functions, root cause, exploit scenario, impact, POC tx/seed, recommended fix, and regression tests.

- **Risk register for admin/config**
  - Safe ranges, prohibited configurations, and transition procedures.

Dependencies
- Draws from roles outputs (06_roles_plan.md) and implements in code/tests during later phases.



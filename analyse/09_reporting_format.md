### 09 — Reporting Format

We will standardize findings with fields that map directly to remediation and regression testing.

- Title, Severity, Affected Files/Functions
- Description and Root Cause
- Exploit Scenario (steps/sequence)
- Impact and Conditions
- POC (transaction or fuzz seed)
- Recommended Fix (code-level guidance)
- Tests to Prevent Regression

Templates will be used in the repo’s `analyse/` for each confirmed issue; tests will reference seeds and expected assertions.



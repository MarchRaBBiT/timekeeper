-- T-04 (docs/design-docs/leave-entitlement.md): grant base dates are derived
-- from the hire date (hire date + 6 months, then yearly). Nullable because
-- existing tenants backfill it operationally; users without a hire date are
-- skipped by the grant run with an explicit reason.

ALTER TABLE users ADD COLUMN hire_date DATE;

# Novin Accounting Architecture v0.3

## Runtime
React + TypeScript UI runs inside Tauri 2. Rust is authoritative for business logic and persistence. SQLite is the local per-company database.

## Rules
- Renderer never writes SQLite directly.
- Financial mutations use Rust commands and database transactions.
- Journal posting requires debit == credit and both totals > 0.
- Posted journals are immutable; future correction uses reversal/correction workflows.
- Audit records are created for sensitive operations.
- Demo data is seeded into SQLite and can be removed from Settings.

## Current modules
Core, Identity, Company, Fiscal Year, Accounting, Contacts, Products, Warehouses, Audit, Settings.

## Planned modules
Sales, Purchase, Inventory, Treasury, Checks, Reporting, Manufacturing, Plugin Runtime, Native Integrations, API Integrations.

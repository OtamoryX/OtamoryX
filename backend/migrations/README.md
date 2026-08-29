# Database Migrations

This project uses `sqlx::Migrator` with backend-specific migration directories:

- `migrations/sqlite`
- `migrations/postgres`

## Naming

Use ordered numeric prefixes so migrations are deterministic:

- `0001_init.sql`
- `0002_add_xxx.sql`
- `0003_fix_xxx.sql`

## Workflow

1. Add one migration file to both directories when schema should stay aligned.
2. Keep each migration idempotent when possible.
3. Avoid editing old migration files after release; add a new version instead.
4. Application startup runs migrations automatically in `src/database/mod.rs`.

## Notes

- SQLx records executed migrations in `_sqlx_migrations`.
- SQLite migrations currently run with `PRAGMA foreign_keys = OFF` during migration execution to allow legacy table-rebuild migrations.
- Released migrations that only performed one-time compatibility cleanup may be removed after deployment. Keep `Migrator::set_ignore_missing(true)` so databases that already recorded a removed version continue to start, and put any required transformation for databases that skipped that version in a later forward migration. The forward migration must be idempotent and must not repeat cleanup that could affect current data.

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
- Released migrations that only performed one-time compatibility cleanup may be removed after deployment. Their original SHA-384 checksums and executable compatibility SQL must remain as retired migration tombstones in `src/database/mod.rs`: applied databases need checksum validation, while older databases that have not reached the retired version still need the original transformation.

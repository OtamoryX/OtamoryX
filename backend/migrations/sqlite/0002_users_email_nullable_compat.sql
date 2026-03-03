DROP TABLE IF EXISTS users_email_migration_tmp;

CREATE TABLE users_email_migration_tmp (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    role TEXT NOT NULL DEFAULT 'user',
    password_hash TEXT NOT NULL,
    api_key TEXT UNIQUE NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users_email_migration_tmp (
    id, username, email, role, password_hash, api_key, created_at, updated_at
)
SELECT
    id, username, NULLIF(email, ''), role, password_hash, api_key, created_at, updated_at
FROM users;

DROP TABLE users;

ALTER TABLE users_email_migration_tmp RENAME TO users;

# SeaORM wiring for the `leptos-auth-template-community` template

This document describes the current **template architecture** and SeaORM wiring used in this repository.

Template focus:

- Leptos + Axum SSR/hydration
- `axum-login` session auth
- SeaORM-backed auth data model
- WebAuthn/passkeys for account security
- Practical migration + entity generation workflow
- App module layout:
  - shared state/auth/csrf in `crates/app/src/contexts.rs`
  - feature logic in `crates/app/src/features/*`
  - route components in `crates/app/src/pages/*` (including `pages/admin.rs`)

---

## 1) Template scope

This template provides a compact baseline for:

1. Database-backed login (`users`, `roles`, `permissions`)
2. Role mapping via junction tables
3. Account security:
   - password change
   - WebAuthn passkey register/list/delete
4. Admin user management basics:
   - create/update/delete users
   - status (`active` / `disabled`)
   - password reset required flag
   - admin-triggered password change for users
5. Clear SeaORM CLI workflow for migrations + entities

---

## 2) SeaORM CLI workflow used

### 2.1 Migration crate location

Migrations are in:

- `crates/migration/Cargo.toml`
- `crates/migration/src/lib.rs`
- `crates/migration/src/main.rs`
- `crates/migration/src/m20260327_070930_create_users.rs`
- `crates/migration/src/m20260327_070930_create_roles.rs`
- `crates/migration/src/m20260327_070930_create_permissions.rs`
- `crates/migration/src/m20260327_070930_create_user_roles.rs`
- `crates/migration/src/m20260327_070930_create_role_permissions.rs`
- `crates/migration/src/m20260327_070930_create_webauthn_credentials.rs`

### 2.2 Migration crate config (SQLite + tokio runtime)

`crates/migration/Cargo.toml` uses:

- `sea-orm-migration`
- features:
  - `runtime-tokio-rustls`
  - `sqlx-sqlite`

### 2.3 Apply migrations

From workspace root:

~~~sh
DATABASE_URL='sqlite://app.sqlite?mode=rwc' sea-orm-cli migrate -d crates/migration up
~~~

### 2.4 Full dev reset

~~~sh
rm -f app.sqlite
DATABASE_URL='sqlite://app.sqlite?mode=rwc' sea-orm-cli migrate -d crates/migration fresh
~~~

### 2.5 Generate entities from live schema

~~~sh
sea-orm-cli generate entity \
  --database-url 'sqlite://app.sqlite?mode=ro' \
  --output-dir ./crates/db/src/entities \
  --entity-format dense
~~~

---

## 3) Schema shape (current)

The split migration set creates auth + passkey tables:

- `users`
- `roles`
- `permissions`
- `user_roles`
- `role_permissions`
- `webauthn_credentials`

### 3.1 Important `users` fields used by auth policy

- `username` (unique)
- `password_hash`
- `status` (`active` / `disabled`)
- `password_reset_required` (bool)
- `webauthn_user_handle` (unique, nullable for rollout)
- timestamps

### 3.2 `webauthn_credentials` fields

- `id`
- `user_id`
- `credential_id` (unique)
- `passkey_json`
- `sign_count`
- `name`
- timestamps

---

## 4) Runtime SeaORM wiring

### 4.1 App state ownership

`AppState` includes:

- SeaORM `DatabaseConnection`
- `webauthn_rs::Webauthn`
- Leptos route/options context data

`AppState` is injected into server handlers/server functions via context.

### 4.2 `axum-login` backend is DB-backed

`Backend` (implemented in `crates/app/src/features/auth.rs`) authenticates via SeaORM queries:

- find user by `username`
- verify password hash
- enforce login policy (`status` must be `active`)
- build role/permission sets via `user_roles` + `role_permissions`
- reload user by ID for session refresh

### 4.3 Session invalidation semantics

Session auth hash is derived from policy-relevant fields:

- password hash
- status
- password reset required flag

This ensures policy changes can invalidate existing session auth state promptly.

### 4.4 Shared app contexts and paths

Current module paths:

- auth snapshot/state + CSRF wiring live in `crates/app/src/contexts.rs`
- app/root wiring imports from `crate::contexts::*`
- feature modules import from `crate::features::*`:
  - auth: `crate::features::auth::*`
  - account: `crate::features::account::*`
  - admin API/types/table: `crate::features::admin::*`

### 4.5 Generated entities source of truth

Entities are generated into:

- `crates/db/src/entities`

The app imports these generated entities (`pub use db::entities;`) to minimize schema drift.

---

## 5) Auth policy wiring (template behavior)

### 5.1 Status gating

- Users with non-`active` status cannot log in.

### 5.2 Password-reset-required gating

- If `password_reset_required = true`, user is redirected to account password flow and blocked from other protected routes until password change succeeds.
- Successful password change clears `password_reset_required`.

### 5.3 Route-level enforcement

Protected routes use auth-state conditions that block access when reset is required (except password-change route itself).

### 5.4 WebAuthn login policy parity

Passkey login enforces the same account policy checks (`active`, not reset-required) before finalizing login.

---

## 6) WebAuthn/passkey wiring

### 6.1 Startup wiring

Server builds one `webauthn_rs::Webauthn` instance at startup and stores it in `AppState`.

### 6.2 Server functions

Current server-side passkey flows include:

- account passkey registration start/finish
- account passkey list/delete
- login passkey start/finish (discoverable auth)

### 6.3 Client-side browser helpers

Browser-side helpers call `navigator.credentials.create/get` and pass typed payloads to server functions.

---

## 7) Admin seeding workflow

The template ships an idempotent seed binary:

- `crates/app/src/bin/seed_admin.rs`

Behavior highlights:

- explicit gate: `SEED_ALLOW_ADMIN=1`
- password sources:
  - `SEED_ADMIN_PASSWORD` (explicit)
  - secure random (default)
  - insecure dev default only when `SEED_DEV_INSECURE=1`
- optional forced rotation for existing admin:
  - `SEED_ADMIN_FORCE_RESET=1`
- optional staff seed:
  - `SEED_STAFF=1`
  - `SEED_STAFF_USERNAME`
  - `SEED_STAFF_PASSWORD`
  - `SEED_STAFF_FORCE_RESET=1`
- optional seed summary output file:
  - `SEED_ADMIN_OUT_FILE=/path/to/seed-summary.txt`

Run:

~~~sh
SEED_ALLOW_ADMIN=1 \
DATABASE_URL='sqlite://app.sqlite?mode=rwc' \
cargo run --features ssr --bin seed_admin
~~~

---

## 8) End-to-end runbook

Set env:

~~~sh
export DATABASE_URL="sqlite://$PWD/app.sqlite?mode=rwc"
export LEPTOS_ENV=dev
export WEBAUTHN_RP_ORIGIN="http://localhost:3000"
export WEBAUTHN_RP_ID="localhost"
export WEBAUTHN_RP_NAME="example-app"
~~~

Migrate:

~~~sh
sea-orm-cli migrate -d crates/migration up
~~~

Seed admin:

~~~sh
SEED_ALLOW_ADMIN=1 cargo run --features ssr --bin seed_admin
~~~

Run app:

~~~sh
cargo leptos watch
~~~

Open:

- `http://localhost:3000`

---

## 9) Why this structure

This template keeps the auth foundation understandable while still covering realistic integration points:

- official SeaORM migration/codegen workflow
- Axum + `axum-login` + session auth integration
- CSRF and protected server-fn boundaries
- baseline account security with WebAuthn passkeys
- admin CRUD and policy controls for users

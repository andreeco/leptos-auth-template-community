# leptos-auth-template-community

> ⚠️ Community template: not production-hardened by default. Review and secure before real-world deployment.

Work-in-progress **login template** for:

- **Leptos** (SSR + islands hydration)
- **Axum**
- **axum-login** + **tower-sessions**
- **SeaORM** (SQLite)
- **WebAuthn** (passkeys)

This repository is intentionally focused on a clean, readable authentication baseline you can copy and extend.

## What’s included

- Session login/logout with `axum-login`
- Auth UI state sync via server snapshot (`AuthState` / `auth_snapshot`)
- Modular app layout with:
  - `contexts.rs` for auth/csrf state + bootstrap helpers
  - `features/` for domain logic (`auth`, `account`, `admin`)
  - thin route/UI pages under `pages/`
- Database-backed auth model (`users`, `roles`, `permissions`, join tables)
- Protected server functions under `/api/secure/*`
- CSRF validation on state-changing actions
- Role-aware route protection (including admin-only area)
- Account security pages:
  - change password
  - passkeys (WebAuthn): register, list, delete
- Admin users table:
  - create/update/delete users
  - set/reset `password_reset_required`
  - set user status (`active` / `disabled`)
  - change user password

## Policy behavior in this template

- Users with status other than `active` cannot log in.
- If `password_reset_required = true`, user is redirected to the password-change page and blocked from other protected routes until password is changed.
- Successful password change clears `password_reset_required`.
- Session auth hash includes policy-relevant fields so policy updates invalidate existing sessions promptly.

## Quick start

Set environment:

~~~bash
export DATABASE_URL="sqlite://$PWD/app.sqlite?mode=rwc"
export LEPTOS_ENV=dev
export WEBAUTHN_RP_ORIGIN="http://localhost:3000"
export WEBAUTHN_RP_ID="localhost"
export WEBAUTHN_RP_NAME="example-app"
~~~

Apply migrations:

~~~bash
sea-orm-cli migrate -d crates/migration up
~~~

Seed admin user (idempotent):

~~~bash
SEED_ALLOW_ADMIN=1 cargo run --features ssr --bin seed_admin
~~~

Run app (islands architecture enabled):

~~~bash
cargo leptos watch
~~~

Release build:

~~~bash
cargo leptos build --release
~~~

Open:

- `http://localhost:3000`

## Optional dev reset workflow

~~~bash
rm -f app.sqlite
sea-orm-cli migrate -d crates/migration fresh
SEED_ALLOW_ADMIN=1 cargo run --features ssr --bin seed_admin
# Seed admin result:
# - action=created
# - username=admin
# - password_source=generated_random
# - password_applied=true
# - roles=[user,admin]
# - permission=[admin.read]
# - webauthn_user_handle=3e429a8c-be40-4604-b0ca-8cacdeb6d3f2

# Effective plaintext credential (store securely):
# admin=example-app-admin-6c4d88cb3f23-2f26cde80584
~~~

## Notes

- This is a focused template, not a complete production auth platform.
- This template uses the islands architecture (`#[island]` + `hydrate_islands`) instead of lazy routes.
- Standard frontend build commands are sufficient:
  - `cargo leptos watch`
  - `cargo leptos build --release`
- Current session store is in-memory for simplicity and should not be used for production deployments.
- For production, use a persistent session backend, HTTPS-only deployment, hardened cookie/security settings, and full operational monitoring.
- Login throttling is intentionally basic in this template; consider per-account and per-IP lockout/rate-limiting policies for stronger protection.

## Code layout (current)

`crates/app/src` is organized as:

- `app.rs`: app shell, router, route registration
- `contexts.rs`: auth/csrf context types, server functions, and SSR/hydration bootstrap helpers
- `features/`:
  - `auth.rs`: auth backend/session-facing types and logic
  - `account.rs`: account/profile/password/webauthn server/client logic
  - `admin/`: admin API/types/users table logic
- `pages/`: route components (kept thin)
- `components/`: shared UI components
- `i18n_utils.rs`: localized path helpers and alternate link generation

This layout keeps route/view files simple and co-locates feature logic in `features/*`.

## Project docs

- SeaORM wiring + migration/codegen workflow: `docs/seaorm-wiring.md`

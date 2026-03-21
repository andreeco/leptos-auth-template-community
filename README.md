# leptos-axum-login-try

Work in progress — demo repo.

Small reference app for **session-based authentication** with **Leptos (SSR + hydration)** and **Axum**, using **axum-login + tower-sessions**.  
Based on the official Leptos Axum starter template.

## What’s in here
- Session login/logout (`axum-login` + `tower-sessions`)
- UI auth state kept in sync via a server snapshot (`AuthState` / `auth_snapshot`)
- Protected server functions under `/api/secure/*` (enforced via Axum middleware)
- Extra checks (e.g. admin role) inside server functions
- Basic hardening: security headers, rate limiting, CSRF (token stored in session; login/logout validate CSRF)

## Demo users
- `user` / `password`
- `admin` / `password` (sees `/admin`)

## Run
```bash
cargo leptos watch
```

## Notes
This is a focused demo, not production-ready (it uses an in-memory session store). For real use, you’d want a persistent session store (e.g. Redis), HTTPS-only cookies, and complete CSRF coverage for all state-changing actions.

## Bigger sibling project (`auth-leptos-demo`)
There is also a larger demo project, which is not published (yet?).

In a summary, it extends the same foundation into a broader, more complete setup with:

- full account flows (sign-up/sign-in/sign-out, confirmation, recovery, reset)
- stronger authentication options (including MFA choices such as TOTP and WebAuthn)
- user self-service pages (profile, sessions, security-related actions)
- admin and operational screens (management, audit/activity visibility, policy controls)
- shared database model and migrations (SeaORM)
- an integration example that wires everything together in one place

`leptos-axum-login-try` remains the small, readable starting point.  
`auth-leptos-demo` is the bigger one for when one needs a wider feature set.

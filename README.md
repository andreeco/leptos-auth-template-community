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
This is just a demo, not production-ready (uses an in-memory session store). For real use you’d want a persistent session store (e.g. Redis), HTTPS-only cookies, and full CSRF coverage for all state-changing actions.

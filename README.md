# Newage LMS SaaS

Multi-tenant learning management platform with a Rust API and a Nuxt.js web frontend.

## Repository layout

- **backend/** — Rust, Axum/Tokio, MongoDB API, HTTP security, cookie-session refresh endpoints, tenant-scoped course/content workflows, media/payment integrations, and backend smoke tests.
- **frontend/** — Nuxt 4, Vue 3, and Tailwind CSS 4 website with an HttpOnly-cookie BFF, registration/sign-in, course authoring, learner enrolment/progress, company users, and payment settings.

## Start the backend

Install stable Rust and start a local MongoDB instance (or configure MongoDB Atlas). From the repository root:

    cd backend
    Copy-Item .env.example .env

Edit backend/.env for your environment, then run:

    cargo run

The default API origin is http://127.0.0.1:8080. Check /health/live and /health/ready before starting the frontend.

For backend setup and production caveats, see [backend/README.md](backend/README.md).

## Start the frontend

Use Node.js 22.12 or newer. In a second terminal, from the repository root:

    cd frontend
    Copy-Item .env.example .env
    npm install --global npm@11
    npm install
    npm run dev

Open http://localhost:3000. Nuxt's server-side BFF calls the Rust API using NUXT_API_INTERNAL_BASE; do not expose this setting with the NUXT_PUBLIC_ prefix.

For session behavior and frontend setup, see [frontend/README.md](frontend/README.md).

## Validation

- Backend: cargo fmt --all, cargo test --all-targets, cargo clippy --all-targets, cargo build --release, and API smoke tests against a disposable database.
- Frontend: npm run typecheck and npm run build from frontend/; GitHub Actions runs both.

This repository is a development foundation. Provider sandbox verification, operational monitoring, atomic quota enforcement, production secrets/TLS/backups, and production security/load reviews are required before public launch.

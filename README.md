# Newage LMS SaaS

Multi-tenant learning management platform with a Rust API and a Nuxt.js web frontend.

## Repository layout

- **backend/** — Rust, Axum/Tokio, MongoDB API, authentication, tenant-scoped course workflows, media/payment integrations, and backend smoke tests.
- **frontend/** — Nuxt 4, Vue 3, and Tailwind CSS 4 website with workspace registration/sign-in and an API-backed course dashboard.

## Start the backend

Install stable Rust and start a local MongoDB instance (or configure MongoDB Atlas). From the repository root:

    cd backend
    Copy-Item .env.example .env

Edit backend/.env for your environment, then run:

    cargo run

The default API origin is http://127.0.0.1:8080. Check /health/live and /health/ready before starting the frontend.

For the full backend configuration and security limitations, see [backend/README.md](backend/README.md).

## Start the frontend

Use Node.js 22.12 or newer. In a second terminal, from the repository root:

    cd frontend
    Copy-Item .env.example .env
    npm install
    npm run dev

Open http://localhost:3000. The frontend API origin is configured by NUXT_PUBLIC_API_BASE; the backend must allow the exact browser origin through CORS_ALLOWED_ORIGINS.

For page descriptions, session behavior, and frontend setup details, see [frontend/README.md](frontend/README.md).

## Validation

- Backend: cargo fmt --all, cargo test --all-targets, cargo clippy --all-targets, cargo build --release, and the disposable-database API smoke test.
- Frontend: npm run typecheck and npm run build from frontend/; GitHub Actions runs both checks for frontend changes.

This repository is still a development foundation. Provider sandbox verification, a production session strategy, operational monitoring, and production security/load reviews are required before public launch.
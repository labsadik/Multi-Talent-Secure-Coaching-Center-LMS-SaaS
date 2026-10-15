# Newage LMS SaaS

Multi-tenant learning management platform with a Rust API and a Nuxt.js web frontend.

## Quick start on Windows / VS Code

Install Node.js 22.12+, the stable Rust toolchain, and Docker Desktop. From VS Code's PowerShell terminal at the repository root, run:

    .\\scripts\\dev.ps1

The script creates local `.env` files from the development examples when missing, starts the included MongoDB development container if MongoDB is not already reachable, opens the Rust API and Nuxt frontend in separate terminals, and waits for `/health/ready` before starting the frontend. Review `backend/.env` before use. To start MongoDB manually, run `docker compose -f compose.dev.yml up -d mongo`. The container stores development data in a named Docker volume; stop it with `docker compose -f compose.dev.yml down` (add `-v` only when you intentionally want to delete that local database volume).

## Repository layout

- **backend/** — Rust, Axum/Tokio, MongoDB API, HTTP security, cookie-session refresh endpoints, tenant-scoped course/content workflows, media/payment integrations, and backend smoke tests.
- **frontend/** — Nuxt 4, Vue 3, and Tailwind CSS 4 website with an HttpOnly-cookie BFF, course authoring, a company public-page/theme editor, branded course catalogues, learner checkout/progress, company users, and payment settings.

## Where company features live

- Company workspace/course dashboard: `/dashboard`
- Public page and theme editor: `/dashboard/site`
- Public company page: `/p/<company-slug>` (publish the page in its editor)
- Course curriculum and media asset library: open a course from the dashboard, then choose **Manage content**.
- Company Razorpay/payment and plan settings: `/dashboard/settings`

The course curriculum is organised as Subject → Part → Chapter → Chapter part → Lesson. Lessons can be text, video or file content; image/PDF/video assets are managed from the selected part's asset library.

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

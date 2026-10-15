# Backend

The Newage backend is a Rust HTTP API. It handles authentication, company-scoped permissions, course and curriculum data, learner progress, media access, public company pages, and course payment verification.

## Requirements

- Rust stable toolchain, Rust 1.85 or newer.
- MongoDB 7 for local development, or a configured MongoDB Atlas database.
- A local environment file created from the example.

## Run locally

From the repository root, prepare the development settings and database:

    Copy-Item .\backend\.env.example .\backend\.env
    docker compose -f compose.dev.yml up -d mongo

Review backend/.env, then start the API:

    cd backend
    cargo run

The default listener is 127.0.0.1:8080.

Check these endpoints before starting the frontend:

- http://127.0.0.1:8080/health/live — confirms that the API process is responding.
- http://127.0.0.1:8080/health/ready — confirms that the API can reach MongoDB.

## Configuration

The API reads environment variables from the process environment and, for local development, from backend/.env. Process environment values take precedence.

Required settings include:

- **APP_ENV** — use development locally and production in deployment.
- **MONGODB_URI** and **MONGODB_DATABASE** — database connection and name.
- **JWT_SECRET**, **JWT_ISSUER**, and **JWT_AUDIENCE** — access-token settings. Use a unique random secret of at least 32 bytes for every environment.
- **CORS_ALLOWED_ORIGINS** — comma-separated exact web origins, without wildcard values or trailing slashes.
- **APP_ENCRYPTION_KEY_BASE64** — required in production; it must decode to exactly 32 bytes.

Optional media configuration is documented in backend/.env.example. Bunny Storage requires both **BUNNY_STORAGE_ZONE** and **BUNNY_STORAGE_API_KEY**. Bunny Stream requires both **BUNNY_STREAM_LIBRARY_ID** and **BUNNY_STREAM_API_KEY**. Do not put provider secrets in the frontend environment or in client-side code.

Each company configures its own Razorpay key ID, API secret, and webhook secret through its workspace settings. The backend verifies payments and grants access based on server-side payment state, not only on the browser checkout callback.

## Development checks

Run from the backend directory:

    cargo fmt --all --check
    cargo test --all-targets
    cargo clippy --all-targets
    cargo build --release

The API smoke test script creates test data. Run it only against a disposable development or test database, never a production database. It requires curl and jq.

From the backend directory:

    bash scripts/smoke-test.sh

## Production

Do not use development secrets or a local development database for a public release. Configure TLS, production credentials, a durable MongoDB deployment, backups, log collection, monitoring, and a secret-rotation process. The API rate limiter is in memory and per process, so a multi-instance deployment needs a shared rate-limit layer or an appropriate gateway.

See [DEPLOYMENT.md](../DEPLOYMENT.md) for the release checklist, provider webhooks, and production environment settings.

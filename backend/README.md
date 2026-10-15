# Newage Rust API

This is the Rust backend foundation for the multi-tenant Newage LMS SaaS. The Nuxt.js frontend is intentionally not started until backend contracts and tenant security are in place.

## Included in this first backend slice

- Axum/Tokio HTTP server with graceful shutdown, structured JSON logs, security headers, exact-origin CORS, and request body limits.
- Centralized configuration loaded from environment variables and a local .env via dotenvy. Process environment variables take precedence.
- MongoDB connection pooling and unique indexes for company slugs, company-scoped users, and company-scoped course slugs.
- Company-owner and learner registration, password hashing with Argon2, tenant-scoped login, short-lived JWT access tokens with issuer/audience validation, and /auth/me.
- Tenant-aware course creation/listing and course publishing. Learners only discover published courses; all course queries are filtered by the authenticated user's company ID.
- Subject → Topic → Lesson APIs with tenant-scoped parent checks, content validation, ordering, and media-asset ownership checks for video/file lessons.
- Tenant owner/admin company-user listing, creation, role updates and disable/enable with privilege-escalation checks.
- Learner course enrollment listing, lesson progress updates, and per-course completion statistics backed by unique MongoDB indexes.
- Tenant Razorpay credentials stored only as AES-256-GCM encrypted values, course order creation using server-side course prices, checkout signature + captured-payment verification, idempotent enrollment writes, and a signature-verified `payment.captured` webhook.
- Bunny Storage topic-scoped image/PDF uploads, Bunny Stream per-course collection provisioning and time-limited TUS upload tickets, video encoding status sync, Razorpay platform order helper, and AES-256-GCM utility for tenant secrets.
- CI checks for formatting, Clippy, tests, and compilation.

This is a working foundation, not yet a complete production LMS. The next backend slices should add subjects/topics/lessons CRUD, Bunny collection provisioning and upload orchestration, signed playback access, student enrollment/progress, Razorpay webhook verification/idempotency, plan enforcement, super-admin operations, email verification/password reset, audit logging, and distributed rate limiting.

## Local setup

1. Install a current stable Rust toolchain and MongoDB Community Server or create a MongoDB Atlas database.
2. Copy backend/.env.example to backend/.env and replace development secrets. Keep the real .env untracked.
3. Start MongoDB, then run these commands from the repository root: cd backend; cargo fmt --all; cargo test; cargo run.
4. Confirm GET http://127.0.0.1:8080/health/live and GET http://127.0.0.1:8080/health/ready.

The API listener defaults to 127.0.0.1:8080. Set HOST=0.0.0.0 only in a container or deployment environment where an external reverse proxy/firewall controls ingress.

## Environment variables

All runtime configuration is read from environment variables; a local .env is loaded for development. See .env.example for supported settings.

Required: APP_ENV, MONGODB_URI, MONGODB_DATABASE, JWT_SECRET, JWT_ISSUER, JWT_AUDIENCE, and CORS_ALLOWED_ORIGINS.

Optional upload/playback settings: `MAX_ASSET_UPLOAD_BYTES`, `BUNNY_TUS_ENDPOINT`, `BUNNY_UPLOAD_SIGNATURE_TTL_SECONDS`, `BUNNY_STREAM_TOKEN_AUTH_KEY`, and `BUNNY_PLAYBACK_TOKEN_TTL_SECONDS`. Enable Embed View Token Authentication in the Bunny Stream library security settings before using signed playback. Direct browser video uploads use a short-lived TUS signature; the Bunny API key never leaves the server. Storage file uploads are streamed through the API and bounded by both request and asset size caps.

A company’s Razorpay key ID, API secret, and webhook secret are submitted through its authenticated settings endpoint. Only encrypted API/webhook secrets are stored in MongoDB; `APP_ENCRYPTION_KEY_BASE64` is required to enable this and must be managed as a durable secret with a rotation/backup plan.

Optional provider configuration must be supplied in matching pairs:
- Bunny Storage: BUNNY_STORAGE_ZONE + BUNNY_STORAGE_API_KEY
- Bunny Stream: BUNNY_STREAM_LIBRARY_ID + BUNNY_STREAM_API_KEY
- Platform Razorpay: RAZORPAY_KEY_ID + RAZORPAY_KEY_SECRET

APP_ENCRYPTION_KEY_BASE64 must decode to 32 bytes when set and is mandatory in production. Generate one using openssl rand -base64 32. Use a separate random JWT_SECRET for each environment. Never place secrets in frontend variables, client bundles, logs, repository files, or issue comments.

## API routes

- GET / — service identity
- GET /health/live — process liveness
- GET /health/ready — MongoDB readiness
- POST /api/v1/auth/register — creates a company and its owner
- POST /api/v1/auth/login — accepts companySlug, email, and password
- GET /api/v1/auth/me — returns the current authenticated user and company
- GET /api/v1/courses — lists courses for the authenticated company
- POST /api/v1/courses — creates a draft course for the authenticated owner/admin/instructor

Registration JSON fields: companyName, companySlug, name, email, password. Passwords must be 12-128 characters.

Course creation requires an Authorization: Bearer access token and accepts title, slug, description, and priceInPaise. Price is stored in integer paise (for example, 49900 means INR 499.00).

## Security and deployment notes

- Never commit .env, real credentials, or generated keys. The repository contains .env.example only.
- MongoDB documents are shared between tenants. Handlers must always filter by the authenticated companyId; never accept a tenant ID from an untrusted browser as authorization.
- Provider API keys remain server-side. Do not return Bunny AccessKey or Razorpay secrets to a browser.
- Tenant provider credentials should be encrypted with crypto::encrypt_tenant_secret before database storage, using that tenant's immutable company ID as associated data.
- Configure TLS at a trusted reverse proxy/load balancer. Add distributed per-IP and per-account rate limits (for example via Redis or the edge gateway) before public production launch.
- Course payment amounts must be derived from server-side database records. Payment success must be based on verified Razorpay signatures/webhooks and idempotent server-side state transitions, never a frontend success redirect alone.
- Run cargo fmt --all -- --check, cargo clippy --all-targets, and cargo test in CI before release.

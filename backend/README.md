# Newage Rust API

This is the Rust backend foundation for the multi-tenant Newage LMS SaaS. The Nuxt.js frontend is intentionally held until API contracts and tenant security have been exercised.

## Included

- Axum/Tokio HTTP server with graceful shutdown, structured JSON logs, security headers, exact-origin CORS, rate limits, and request body limits.
- Centralized configuration from process environment variables and a local `.env` via dotenvy. Process environment variables take precedence.
- MongoDB connection pooling and unique indexes for company slugs, company-scoped users/course slugs, learning content, enrollments, payments, and subscriptions.
- Company-owner and learner registration, Argon2 password hashing, tenant-scoped login, short-lived JWT access tokens with issuer/audience validation, and database revalidation of the user's active status and current role on authenticated requests.
- Opaque refresh sessions with SHA-256 hash-only storage, 30-day sliding inactivity expiry, TTL cleanup, explicit logout revocation, and HttpOnly-cookie handling through the Nuxt BFF.
- Opaque, 256-bit refresh-session credentials with SHA-256 hash-only storage, 30-day sliding inactivity expiry, TTL cleanup, explicit logout revocation, and server-side HttpOnly-cookie handling in the Nuxt BFF.
- Tenant-aware course create/list/publish, subject/topic/lesson creation and discovery, preview-only lesson listings for non-enrolled learners, and learner progress/completion summaries.
- Tenant owner/admin company-user listing, creation, role updates, disable/enable workflows, and active learner quota checks.
- Bunny Storage topic-scoped file uploads with MIME/signature/size validation and authenticated downloads; Bunny Stream per-course collections, short-lived TUS upload tickets, encoding-status synchronization, and signed playback access.
- Tenant Razorpay credentials encrypted with AES-256-GCM, server-calculated course order amounts, signature-verified payment confirmation/webhooks, idempotent enrollment/payment writes, plus separate platform SaaS subscription billing and webhook processing.
- CI for formatting, unit tests, Clippy, release compilation, and HTTP smoke tests against a temporary MongoDB service. Smoke tests cover health/readiness, security headers, auth guards, registration/login, role enforcement, course lifecycle, and cross-tenant isolation.

This is a substantial backend foundation, not yet a complete production LMS. Before a public production launch, remaining work includes email verification/password reset and transactional email, fuller content edit/delete workflows, shared/distributed rate limiting for multi-instance deployments, audit logs and operational alerting, atomic quota enforcement under concurrent writes, and end-to-end verification against real Bunny/Razorpay sandbox accounts. Production TLS, secrets, backups, deployment monitoring, and a documented secret-rotation procedure must also be configured. Do not treat a green CI run as a substitute for provider sandbox tests or production security review.

## Local setup

1. Install a current stable Rust toolchain and MongoDB Community Server, or create a MongoDB Atlas database.
2. Copy `backend/.env.example` to `backend/.env` and replace development secrets. Keep the real `.env` untracked.
3. Start MongoDB, then run these commands from the repository root:

   ```bash
   cd backend
   cargo fmt --all
   cargo test --all-targets
   cargo clippy --all-targets
   cargo run
   ```

4. Confirm `GET http://127.0.0.1:8080/health/live` returns `{"status":"live"}` and `GET http://127.0.0.1:8080/health/ready` returns `{"status":"ready","database":"ok"}`.
5. In a second terminal, run `backend/scripts/smoke-test.sh` from the repository root (or `scripts/smoke-test.sh` from `backend/`). It creates test companies, a learner, and a course in the configured database. **Run it only against a disposable development/test database, never production.** Requires `curl` and `jq`.

The API listener defaults to `127.0.0.1:8080`. Set `HOST=0.0.0.0` only in a container or deployment environment where a reverse proxy/firewall controls ingress.

## Environment variables

All runtime configuration is read from environment variables; a local `.env` is loaded for development. See `.env.example` for supported settings.

Required: `APP_ENV`, `MONGODB_URI`, `MONGODB_DATABASE`, `JWT_SECRET`, `JWT_ISSUER`, `JWT_AUDIENCE`, and `CORS_ALLOWED_ORIGINS`.

Optional upload/playback settings include `MAX_ASSET_UPLOAD_BYTES`, `BUNNY_TUS_ENDPOINT`, `BUNNY_UPLOAD_SIGNATURE_TTL_SECONDS`, `BUNNY_STREAM_TOKEN_AUTH_KEY`, and `BUNNY_PLAYBACK_TOKEN_TTL_SECONDS`. Enable Embed View Token Authentication in the Bunny Stream library security settings before using signed playback. Direct browser video uploads use a short-lived TUS signature; the Bunny API key never leaves the server. Storage uploads are streamed through the API and bounded by both request and asset size caps.

A company's Razorpay key ID, API secret, and webhook secret are submitted through its authenticated settings endpoint. Only encrypted API/webhook secrets are stored in MongoDB; `APP_ENCRYPTION_KEY_BASE64` is required in production and must be managed as a durable secret with a rotation/backup plan.

Per-plan course/learner caps are read from `PLAN_*_MAX_COURSES` and `PLAN_*_MAX_LEARNERS`; paid plans downgrade to free when the stored subscription expires. Platform billing reads `RAZORPAY_KEY_ID`, `RAZORPAY_KEY_SECRET`, `RAZORPAY_WEBHOOK_SECRET`, and `PLAN_*_MONTHLY_PRICE_PAISE` from `.env`. Platform credentials are separate from each company's encrypted course-payment credentials.

Optional provider configuration must be supplied in matching pairs:
- Bunny Storage: `BUNNY_STORAGE_ZONE` + `BUNNY_STORAGE_API_KEY`
- Bunny Stream: `BUNNY_STREAM_LIBRARY_ID` + `BUNNY_STREAM_API_KEY`
- Platform Razorpay: `RAZORPAY_KEY_ID` + `RAZORPAY_KEY_SECRET`

`APP_ENCRYPTION_KEY_BASE64` must decode to 32 bytes when set and is mandatory in production. Generate one with `openssl rand -base64 32`. Use a separate random JWT secret per environment. Never put secrets in frontend variables, client bundles, logs, repository files, or issue comments.

## Main API routes

- `GET /` — service identity.
- `GET /health/live` and `GET /health/ready` — liveness and MongoDB readiness.
- `POST /api/v1/auth/register`, `POST /api/v1/auth/register-learner`, `POST /api/v1/auth/login`, `POST /api/v1/auth/refresh`, `POST /api/v1/auth/logout`, `GET /api/v1/auth/me` — identity and authentication.
- `GET/POST /api/v1/courses`, `POST /api/v1/courses/{course_id}/publish` — tenant-scoped course workflows.
- `GET/POST /api/v1/courses/{course_id}/subjects`, `GET/POST /api/v1/subjects/{subject_id}/topics`, `GET/POST /api/v1/topics/{topic_id}/lessons` — learning content.
- `GET/POST /api/v1/company/users` and `PATCH /api/v1/company/users/{user_id}` — company user management.
- `GET /api/v1/me/courses`, `PUT /api/v1/lessons/{lesson_id}/progress`, `GET /api/v1/courses/{course_id}/my-progress` — learner progress.
- Media: `PUT /api/v1/topics/{topic_id}/assets`, `POST /api/v1/topics/{topic_id}/video-upload-ticket`, `GET /api/v1/assets/{asset_id}/download`, `GET /api/v1/lessons/{lesson_id}/playback`.
- Payments: `POST /api/v1/courses/{course_id}/orders`, `POST /api/v1/payments/verify`, `POST /api/v1/webhooks/razorpay`, `POST /api/v1/company/subscription/orders`, `POST /api/v1/webhooks/platform-razorpay`.

Registration JSON fields: `companyName`, `companySlug`, `name`, `email`, `password`. Passwords must be 12–128 characters. Course creation requires an `Authorization: Bearer <access-token>` header and accepts `title`, `slug`, `description`, and `priceInPaise`. Price is stored in integer paise (for example, `49900` means INR 499.00).

## Security and deployment notes

- Never commit `.env`, real credentials, or generated keys. The repository contains `.env.example` only.
- MongoDB documents are shared between tenants. Handlers must filter by the authenticated tenant's company ID; never accept an untrusted client-supplied tenant ID as authorization.
- Provider API keys remain server-side. Do not return Bunny AccessKey or Razorpay secrets to browsers.
- Configure TLS at a trusted reverse proxy/load balancer. The current rate limiter is in-memory and per process; deploy a shared rate-limit store or edge gateway before horizontally scaling or public production launch.
- Course payment amounts must come from server-side database records. Success depends on verified payment signatures/webhooks and idempotent server-side state transitions, never frontend redirects alone.
- Quota checks currently use count-then-write patterns. Add transaction/atomic enforcement before relying on caps as a hard commercial entitlement under concurrent requests.


## Refresh sessions and frontend integration

Login and registration responses include an opaque refresh credential for server-to-server/session management. The Nuxt BFF keeps both the access JWT and refresh credential in HttpOnly, SameSite=Lax cookies; browser JavaScript receives only the public user/company profile. The backend stores only the SHA-256 digest of refresh credentials in the refresh_sessions collection, with unique-token and TTL indexes. The 15-minute (configurable JWT_TTL_SECONDS) access JWT is renewed when needed. Refresh activity extends a 30-day inactivity window; POST /api/v1/auth/logout revokes the refresh session. Do not expose the refresh credential or call the server session endpoints from an untrusted cross-origin form.

Test session restoration, access expiry, explicit logout, and revoked-token rejection with the Nuxt BFF as part of release testing. The backend smoke script validates refresh and logout against a disposable MongoDB.

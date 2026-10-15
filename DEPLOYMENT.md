# Production Deployment

This guide describes the production setup expected by the current Newage MVP. The repository includes a Dockerfile for the Rust API, but it does not provide a complete production stack or one-click cloud deployment. Choose a hosting provider for the API and Nuxt server, a managed MongoDB deployment, and a TLS-enabled reverse proxy or managed HTTPS ingress.

## 1. Production layout

A typical deployment uses:

- **Nuxt frontend:** a Node.js server running the Nuxt production output.
- **Rust API:** the Newage API process or the image built from backend/Dockerfile.
- **MongoDB:** a managed MongoDB deployment such as MongoDB Atlas, with backups enabled.
- **HTTPS routing:** a reverse proxy or managed ingress that terminates TLS and routes requests to the correct service.
- **Media and payments:** Bunny Storage/Stream and each coaching centre's own Razorpay account, when those features are enabled.

Keep the API, database, and provider credentials on the server side. Only the frontend should be exposed as the normal learner and staff web experience. The API must also be reachable by the configured external provider webhook services.

## 2. Configure the Rust API

Set production environment variables in the hosting provider's secret/configuration manager. Do not commit a production .env file.

Required settings include:

- **APP_ENV=production**
- **HOST=0.0.0.0** when running inside a container.
- **PORT=8080**, or the port assigned by the hosting provider.
- **MONGODB_URI** and **MONGODB_DATABASE** for the production database.
- **JWT_SECRET** with at least 32 random bytes, and unique production values for **JWT_ISSUER** and **JWT_AUDIENCE**.
- **CORS_ALLOWED_ORIGINS** with the exact HTTPS frontend origin, with no wildcard and no trailing slash.
- **APP_ENCRYPTION_KEY_BASE64**, generated as a base64-encoded 32-byte key. This setting is mandatory in production because company payment secrets are encrypted before they are stored.

On a machine with OpenSSL, generate keys with:

    openssl rand -base64 48
    openssl rand -base64 32

Use the first output as a JWT secret and the second as APP_ENCRYPTION_KEY_BASE64. Store them separately in your secrets manager. Never regenerate the encryption key casually: existing encrypted company payment credentials may no longer be readable with a different key. Keep a protected backup and document a controlled rotation process.

To build the API container from the repository root:

    docker build -t newage-api:release ./backend

Run it with the production environment supplied securely by your container platform. The included image listens on port 8080 and runs as a non-root user. Do not expose MongoDB directly to the public internet.

After deployment, verify:

- /health/live responds successfully.
- /health/ready reports that the database is ready.
- API logs do not contain credentials or other secrets.

## 3. Deploy the Nuxt frontend

Use Node.js 22.12 or newer and npm 11 is recommended. From the frontend directory, install dependencies and verify the production build:

    npm install
    npm run typecheck
    npm run build

Deploy the generated Nuxt server output to a Node.js-compatible hosting environment. Start it with the provider's supported Node command; for a standard Nuxt Node output, that is:

    node .output/server/index.mjs

Configure **NUXT_API_INTERNAL_BASE** as a server-side environment variable pointing to the deployed Rust API origin, for example https://api.example.com or a private service URL accessible from the Nuxt server. Do not use the NUXT_PUBLIC_ prefix. Configure the frontend host and port using the variables supported by your hosting runtime, and terminate public traffic over HTTPS.

The frontend uses server-managed cookies for sessions, so use HTTPS in production and test login, page refresh, logout, and session expiry on the deployed domain.

## 4. Connect media and payment providers

### Bunny Storage and Bunny Stream

Media support is optional, but the paired values below must be configured together when enabled:

- Bunny Storage: **BUNNY_STORAGE_ZONE** and **BUNNY_STORAGE_API_KEY**.
- Bunny Stream: **BUNNY_STREAM_LIBRARY_ID** and **BUNNY_STREAM_API_KEY**.

Set **BUNNY_CDN_HOSTNAME** if public CDN URLs are needed for stored images or documents. For Stream uploads, configure the TUS endpoint and playback token settings in backend/.env or the production secret manager. If signed playback is enabled, configure Embed View Token Authentication in the Bunny Stream library and supply the corresponding token key.

If Bunny Stream webhooks are used, configure the public HTTPS callback:

    https://api.example.com/api/v1/webhooks/bunny-stream

Set **BUNNY_STREAM_WEBHOOK_SIGNING_KEY** to the library's Read-Only API key used to verify the webhook signature. Replace the example hostname with the real API hostname.

### Razorpay course payments

Each coaching centre must enter its own Razorpay Key ID, API secret, and webhook secret in the company payment settings. Use the centre's sandbox credentials for end-to-end testing before switching to live credentials.

Configure the Razorpay webhook URL to:

    https://api.example.com/api/v1/webhooks/razorpay

Replace the example hostname with the real API hostname. Test order creation, successful payment verification, webhook delivery, repeated webhook delivery, failed payments, and enrolment in the correct company workspace. Never grant course access solely because the browser returned from a checkout page.

The current application processes payments for a company's courses. It does not automatically charge the coaching centre a Newage platform subscription or split one course payment between the centre and the platform operator. Those commercial terms need a separate process until platform billing or revenue-share functionality is built.

## 5. Release checks

Before giving access to paying customers:

1. Confirm that production secrets are unique, stored securely, and absent from logs and the repository.
2. Confirm that the database is private, access-controlled, backed up, and tested for restoration.
3. Test owner, admin, instructor, and learner permissions across two separate company workspaces.
4. Test public pages, course publishing, learner registration, paid/free enrolment, lesson media, progress, and payment webhooks in a production-like environment.
5. Check TLS, cookie security, reverse-proxy limits, logs, uptime alerts, backups, and incident recovery.
6. Review the in-memory per-process rate limiter before scaling to multiple API instances. Use a shared rate-limit service or an appropriate gateway if deploying more than one API process.
7. Complete a security review and provider sandbox round-trip tests. Passing CI does not replace these checks.

The current codebase is an MVP and requires environment-specific validation. Do not treat a successful build as proof that a production deployment is fully configured.

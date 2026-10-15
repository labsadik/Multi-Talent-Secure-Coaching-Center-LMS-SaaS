# Frontend

The Newage frontend is built with Nuxt 4, Vue 3, TypeScript, and Tailwind CSS 4. Nuxt server routes act as a same-origin backend-for-frontend (BFF), keeping access and refresh credentials in server-managed HttpOnly cookies.

## Requirements

- Node.js 22.12 or newer.
- npm 11 recommended.
- The Rust API and MongoDB must be running before you test authenticated or data-backed features.

## Run locally

From the frontend directory, create the local environment file if it does not exist, install dependencies, and start Nuxt:

    Copy-Item .env.example .env
    npm install
    npm run dev

Open http://localhost:3000. The frontend uses NUXT_API_INTERNAL_BASE to reach the Rust API. The example value points to http://127.0.0.1:8080.

If the API is hosted at another address, update NUXT_API_INTERNAL_BASE in frontend/.env and restart Nuxt. Keep this setting server-only: do not rename it to use the NUXT_PUBLIC_ prefix, and never place database, JWT, encryption, Bunny, or Razorpay secrets in the frontend environment.

## Main areas

- Workspace dashboard and course management.
- Public company page and theme editor.
- Learner course catalogue, enrolment, and progress.
- Company users, roles, and account access.
- Company-specific course payment settings.

## Development checks

Run these commands from the frontend directory:

    npm run typecheck
    npm run build
    npm run preview

Use npm run preview to serve the production build locally. For production hosting, deploy the Nuxt server output to a Node.js-compatible runtime; this app requires server-side routes and should not be deployed as a static-only website.

See [DEPLOYMENT.md](../DEPLOYMENT.md) for production hosting and environment requirements.

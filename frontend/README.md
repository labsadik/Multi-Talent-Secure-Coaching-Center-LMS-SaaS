# Newage frontend

Nuxt 4 + Vue 3 + Tailwind CSS 4 frontend for the Newage LMS API.

## Requirements

- Node.js 22.12 or newer
- npm 11 recommended
- The Rust API and its MongoDB database running locally

## Run locally on Windows PowerShell

From the repository root:

    cd frontend
    Copy-Item .env.example .env
    npm install
    npm run dev

Open http://localhost:3000. Keep the Rust API running at http://127.0.0.1:8080 in another terminal.

If npm 10.9.x fails during dependency installation with the error "Cannot read properties of null (reading 'edgesOut')", update npm and retry:

    npm install --global npm@11
    npm install

The frontend uses NUXT_PUBLIC_API_BASE for the API origin. Add the exact frontend origin to backend/.env under CORS_ALLOWED_ORIGINS. The backend example already allows http://localhost:3000. If you use a different hostname or port, update the CORS list, restart the backend, and restart Nuxt.

## Available pages

- / — public product landing page
- /register — creates a company workspace and owner account through POST /api/v1/auth/register
- /login — authenticates a workspace member through POST /api/v1/auth/login
- /dashboard — authenticated course overview with create and publish flows

## Commands

    npm run dev
    npm run typecheck
    npm run build
    npm run preview

## Session and security notes

Access tokens are held in Nuxt's in-memory useState and are not written to localStorage. This avoids keeping a long-lived bearer token in persistent browser storage, but a browser refresh clears the current session and requires the user to sign in again. For a production deployment, consider a same-site, HttpOnly cookie session through a backend-for-frontend (BFF) rather than persisting a bearer token in browser JavaScript.

Only NUXT_PUBLIC_API_BASE belongs in this frontend environment file. Never put MongoDB URIs, JWT signing secrets, Razorpay secrets, Bunny API keys, or APP_ENCRYPTION_KEY_BASE64 in Nuxt public runtime config or any variable prefixed with NUXT_PUBLIC_.

The dashboard currently covers course listing, draft creation, and publishing. Additional lesson authoring, learner enrollment/progress views, company-user administration, media upload, and billing UI can be built on the existing API routes in later passes.
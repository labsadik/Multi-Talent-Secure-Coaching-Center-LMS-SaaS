# Newage frontend

Nuxt 4 + Vue 3 + Tailwind CSS 4 frontend for the Newage LMS API. Server API routes form a same-origin backend-for-frontend (BFF), so access and refresh credentials are never returned to client-side JavaScript.

## Requirements

- Node.js 22.12 or newer
- npm 11 recommended
- The Rust API and its MongoDB database running locally

## Run locally on Windows PowerShell

From the repository root:

    cd frontend
    Copy-Item .env.example .env
    npm install --global npm@11
    npm install
    npm run dev

Open http://localhost:3000. Keep the Rust API running at http://127.0.0.1:8080 in another terminal.

If npm 10.9.x fails during dependency installation with "Cannot read properties of null (reading 'edgesOut')", update npm to version 11 and retry installation.

The server-side API origin is NUXT_API_INTERNAL_BASE in frontend/.env. It is intentionally not prefixed with NUXT_PUBLIC_: the backend hostname is private Nuxt server configuration. The backend CORS_ALLOWED_ORIGINS should include the exact frontend origin for other browser integrations, but the main frontend-to-backend calls are server-side through the BFF.

## Available pages

- / — public product landing page
- /register — creates a company workspace and owner account
- /login — sign in for a workspace member
- /learn/register — create a learner account within an existing workspace
- /dashboard — workspace overview with visible quick links for course management, public page/theme, payments, and people
- /dashboard/site — company public-page editor, theme presets, brand/accent colours, logo/banner URLs, publication switch, live preview, and public-page link
- /p/:companySlug — public company storefront with its own branding and published course catalogue; free and paid enrolment CTAs
- /dashboard/courses/:courseId — nested curriculum builder: Subject → Part → Chapter → Chapter part → video/text/PDF/image lessons; edit/archive, thumbnail controls, and topic asset library
- /dashboard/learn — learner enrolments, course discovery, Razorpay checkout, and progress (public paid-course links continue to checkout after sign-up)
- /dashboard/learn — learner enrolments, course discovery, checkout, and progress
- /dashboard/users — company member listing, invitations, roles, and disable/reactivate controls
- /dashboard/settings — company Razorpay integration settings and SaaS plan checkout

## Commands

    npm run dev
    npm run typecheck
    npm run build
    npm run preview

## Remember-me session design

1. Login, owner registration, and learner registration are sent to Nuxt server endpoints, not directly to the Rust API from the browser.
2. Nuxt stores the short-lived access JWT and opaque refresh-session credential in HttpOnly, SameSite=Lax cookies. Cookies are Secure on HTTPS and are scoped to /. Tokens are omitted from JavaScript-readable API responses and are never stored in localStorage.
3. The Rust API stores only a SHA-256 hash of the opaque refresh credential. Refresh requests issue a new short-lived access JWT and slide the inactivity expiration of the session. Concurrent requests share the same opaque session credential to avoid refresh-token rotation races.
4. On a page refresh or navigation, the auth middleware calls /api/session/me to restore the current user from the cookie-backed API session. If the access JWT has expired, Nuxt refreshes it on the server and retries the original API request.
5. Logout calls the server endpoint, revokes the refresh session in MongoDB, clears both cookies, and clears the in-memory UI state. Inactive sessions expire after 30 days; normal use renews the inactivity window.

This is safer than localStorage bearer-token persistence and keeps tokens away from client-side JavaScript. As with any cookie-authenticated app, deploy over HTTPS, keep same-origin mutation checks enabled, and perform an XSS/security review before launch. For stronger production isolation, a dedicated BFF deployment and centralized session-management policy can be used.

Only public page configuration belongs in browser bundles. Never put MongoDB URIs, JWT signing secrets, Razorpay secrets, Bunny API keys, or APP_ENCRYPTION_KEY_BASE64 in Nuxt public runtime config or any variable prefixed with NUXT_PUBLIC_.

## Product coverage and provider setup

The company dashboard now exposes the public storefront and theme editor at /dashboard/site. Owners/admins can publish each company's page at /p/<company-slug>, choose a theme/palette, edit the copy, and configure logo/banner image URLs. Draft courses are not shown publicly. The course builder supports Subject → Part (topic) → Chapter → Chapter part → Lesson, and includes video upload, image/PDF uploads, a media asset library, lesson thumbnails, and course-cover assignment. Set BUNNY_CDN_HOSTNAME to show public URLs for uploaded Bunny Storage images/files in the asset library; otherwise HTTPS image URLs can be supplied manually. Archiving hides content from active listings while retaining historical enrolment/payment records.

Paid course enrolment CTAs use the company's Razorpay configuration and backend order/payment-verification endpoints. New learners arrive with the company's slug prefilled and continue to their selected course after registration. Provider features require working Bunny/Razorpay credentials, webhook setup, and sandbox round-trip verification before production use.

The subscription checkout UI relies on the authenticated Razorpay webhook as the activation source of truth. A checkout success screen by itself never grants a plan or course access.

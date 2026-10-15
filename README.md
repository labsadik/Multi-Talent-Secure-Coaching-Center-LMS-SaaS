# Newage — Learning, organised

Newage is a multi-tenant learning management system (LMS) MVP built for coaching centres, training institutes, and education businesses. Each organisation gets its own workspace, branded public course page, course management tools, learner access, and course payment settings.

The goal is simple: help a small coaching centre or a large institute manage its courses online and sell learning directly to its students, without building a separate platform from scratch.

## Product and business model

Newage is designed for a two-sided business:

- **Platform operator:** offers hosted access to Newage to coaching centres through a separate commercial or licensing agreement.
- **Coaching centre:** manages its own courses and learners, publishes a branded storefront, and sells free or paid courses to students. Each centre can connect its own Razorpay account for paid course sales.

The current MVP supports company workspaces and centre-level course sales. **It does not yet include automated platform subscription billing, automatic revenue sharing, or split payouts.** The platform operator's fee and any commercial terms between the parties must be handled separately. See [LICENSE](LICENSE) for the rules that apply to this repository.

## Screenshots

These screenshots show the current product interface. A live demo URL has not been supplied yet.

<p align="center">
  <img src="https://ik.imagekit.io/xvqovhmcyr/lms/Newage%20_%20Learning,%20organised%20-%20Google%20Chrome%2010_15_2026%204_54_08%20PM.png" alt="Newage interface screenshot 1" width="48%" />
  <img src="https://ik.imagekit.io/xvqovhmcyr/lms/Newage%20_%20Learning,%20organised%20-%20Google%20Chrome%2010_15_2026%204_54_45%20PM.png" alt="Newage interface screenshot 2" width="48%" />
</p>
<p align="center">
  <img src="https://ik.imagekit.io/xvqovhmcyr/lms/Newage%20_%20Learning,%20organised%20-%20Google%20Chrome%2010_15_2026%204_55_42%20PM.png" alt="Newage interface screenshot 3" width="48%" />
  <img src="https://ik.imagekit.io/xvqovhmcyr/lms/Newage%20_%20Learning,%20organised%20-%20Google%20Chrome%2010_15_2026%204_55_23%20PM.png" alt="Newage interface screenshot 4" width="48%" />
</p>
<p align="center">
  <img src="https://ik.imagekit.io/xvqovhmcyr/lms/Newage%20_%20Learning,%20organised%20-%20Google%20Chrome%2010_15_2026%204_55_55%20PM.png" alt="Newage interface screenshot 5" width="70%" />
</p>

## What the MVP includes

- Separate company workspaces with role-based access for owners, admins, instructors, and learners.
- Branded public company pages with theme presets, colour controls, font choices, layouts, visual effects, and a live editor preview.
- Course creation and a curriculum organised into subjects, parts, chapters, chapter parts, and lessons.
- Text, video, image, and PDF lesson content, with media stored through Bunny Storage and Bunny Stream integrations.
- Public course catalogues, learner registration and sign-in, enrolment, and learning progress.
- Company-specific Razorpay settings and server-side verification for paid course access.
- Responsive dashboard forms and UI-based modal confirmations for supported create, edit, archive, and delete actions.

## Current limits

This is an MVP, not a claim that every production operation is finished. The application does not currently provide platform subscription checkout or automatic revenue sharing. Provider features require valid credentials and webhook setup; real payment and media flows must be tested with the relevant provider's sandbox before launch. Email verification, password reset, production monitoring, and a full production security review also need to be completed before a broad public rollout.

## Technology

| Area | Technology |
| --- | --- |
| Frontend | Nuxt 4, Vue 3, TypeScript, Tailwind CSS 4 |
| Backend API | Rust, Axum, Tokio |
| Database | MongoDB |
| Authentication | Argon2 password hashing, short-lived JWT access tokens, hashed refresh sessions, and HttpOnly cookies through the Nuxt server-side BFF |
| Course media | Bunny Storage and Bunny Stream |
| Course payments | Razorpay, configured by each company |
| Local development | Docker Compose for MongoDB |
| Quality checks | GitHub Actions, Nuxt typecheck/build, Rust formatting/tests/Clippy/release build, API smoke tests |

## Run after checkout

### Requirements

- Node.js 22.12 or newer and npm 11 recommended.
- Rust stable toolchain (Rust 1.85 or newer).
- Docker Desktop or a reachable local MongoDB instance.

### 1. Prepare local settings

Run these commands from the repository root in PowerShell:

    Copy-Item .\backend\.env.example .\backend\.env
    Copy-Item .\frontend\.env.example .\frontend\.env
    docker compose -f compose.dev.yml up -d mongo

Review the values in both local .env files. The examples are for development only; never use their sample secrets in production.

### 2. Run the Rust API

Open a terminal at the repository root:

    cd backend
    cargo run

The API listens on http://127.0.0.1:8080 by default. Check http://127.0.0.1:8080/health/live and http://127.0.0.1:8080/health/ready.

### 3. Run the Nuxt frontend

Open a second terminal at the repository root:

    cd frontend
    npm install
    npm run dev

Open http://localhost:3000. The frontend's server-side API connection is set by NUXT_API_INTERNAL_BASE in frontend/.env; keep this server-only setting free of the NUXT_PUBLIC_ prefix.

For a Windows development setup that opens both services in separate terminals and checks MongoDB/API readiness, run .\scripts\dev.ps1 from the repository root instead.

## More information

- [Production deployment and release checklist](DEPLOYMENT.md)
- [Backend setup and checks](backend/README.md)
- [Frontend setup and checks](frontend/README.md)
- [Proprietary license and commercial permission](LICENSE)

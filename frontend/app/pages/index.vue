<script setup lang="ts">
definePageMeta({ layout: 'marketing' })

const menuOpen = ref(false)
</script>

<template>
  <div>
    <header class="sticky top-0 z-40 border-b border-line/80 bg-white/90 backdrop-blur-xl">
      <div class="mx-auto flex h-[76px] max-w-7xl items-center justify-between px-5 sm:px-8">
        <NuxtLink to="/" class="inline-flex items-center gap-3 font-semibold tracking-tight">
          <span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white shadow-lg shadow-brand/20">N</span>
          <span class="text-xl">newage<span class="text-brand">.</span></span>
        </NuxtLink>

        <nav class="hidden items-center gap-8 text-sm font-medium text-muted md:flex" aria-label="Main navigation">
          <a href="#platform" class="transition hover:text-ink">Platform</a>
          <a href="#features" class="transition hover:text-ink">Features</a>
          <a href="#workflow" class="transition hover:text-ink">How it works</a>
        </nav>

        <div class="hidden items-center gap-3 md:flex">
          <NuxtLink to="/login" class="rounded-xl px-4 py-2.5 text-sm font-semibold text-ink transition hover:bg-canvas">Sign in</NuxtLink>
          <NuxtLink to="/register" class="inline-flex items-center gap-2 rounded-xl bg-brand px-5 py-3 text-sm font-semibold text-white shadow-lg shadow-brand/20 transition hover:-translate-y-0.5 hover:bg-brand-dark">
            Create workspace
            <svg viewBox="0 0 20 20" fill="none" class="h-4 w-4" aria-hidden="true"><path d="M4 10h12m-5-5 5 5-5 5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" /></svg>
          </NuxtLink>
        </div>

        <button type="button" class="grid h-10 w-10 place-items-center rounded-xl border border-line md:hidden" :aria-expanded="menuOpen" aria-label="Toggle navigation" @click="menuOpen = !menuOpen">
          <svg v-if="!menuOpen" viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
          <svg v-else viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
        </button>
      </div>

      <div v-if="menuOpen" class="border-t border-line bg-white px-5 py-4 md:hidden">
        <nav class="mx-auto flex max-w-7xl flex-col gap-1 text-sm font-medium" aria-label="Mobile navigation">
          <a href="#platform" class="rounded-lg px-3 py-3 hover:bg-canvas" @click="menuOpen = false">Platform</a>
          <a href="#features" class="rounded-lg px-3 py-3 hover:bg-canvas" @click="menuOpen = false">Features</a>
          <a href="#workflow" class="rounded-lg px-3 py-3 hover:bg-canvas" @click="menuOpen = false">How it works</a>
          <NuxtLink to="/login" class="rounded-lg px-3 py-3 hover:bg-canvas" @click="menuOpen = false">Sign in</NuxtLink>
          <NuxtLink to="/register" class="mt-2 rounded-xl bg-brand px-4 py-3 text-center text-white" @click="menuOpen = false">Create workspace</NuxtLink>
        </nav>
      </div>
    </header>

    <main>
      <section id="platform" class="page-grid relative overflow-hidden border-b border-line/70">
        <div class="pointer-events-none absolute -right-48 top-0 h-[500px] w-[500px] rounded-full bg-brand/10 blur-3xl" />
        <div class="pointer-events-none absolute -left-32 bottom-0 h-[350px] w-[350px] rounded-full bg-emerald-200/40 blur-3xl" />
        <div class="relative mx-auto grid max-w-7xl items-center gap-14 px-5 pb-20 pt-16 sm:px-8 sm:pb-28 sm:pt-24 lg:grid-cols-[1.02fr_0.98fr] lg:gap-10">
          <div>
            <div class="inline-flex items-center gap-2 rounded-full border border-brand/15 bg-white/85 px-3.5 py-2 text-xs font-semibold text-brand shadow-sm">
              <span class="h-2 w-2 rounded-full bg-emerald-500" />
              A fresh foundation for modern learning
            </div>
            <h1 class="mt-7 max-w-2xl text-5xl font-semibold leading-[1.07] tracking-[-0.055em] text-ink sm:text-6xl xl:text-7xl">
              Turn what you know into
              <span class="relative inline-block text-brand">what's next.</span>
            </h1>
            <p class="mt-6 max-w-xl text-base leading-8 text-muted sm:text-lg">
              Create a home for your courses, keep your learning business organised, and give every learner a clearer next step.
            </p>
            <div class="mt-9 flex flex-col gap-3 sm:flex-row">
              <NuxtLink to="/register" class="inline-flex items-center justify-center gap-2 rounded-2xl bg-brand px-6 py-4 text-sm font-semibold text-white shadow-xl shadow-brand/20 transition hover:-translate-y-0.5 hover:bg-brand-dark">
                Build your workspace
                <svg viewBox="0 0 20 20" fill="none" class="h-4 w-4" aria-hidden="true"><path d="M4 10h12m-5-5 5 5-5 5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" /></svg>
              </NuxtLink>
              <a href="#features" class="inline-flex items-center justify-center gap-2 rounded-2xl border border-line bg-white/85 px-6 py-4 text-sm font-semibold text-ink transition hover:border-brand/40 hover:bg-white">
                Explore the platform
              </a>
            </div>
            <div class="mt-8 flex flex-wrap items-center gap-x-5 gap-y-2 text-xs font-medium text-muted">
              <span class="inline-flex items-center gap-2"><svg viewBox="0 0 20 20" fill="none" class="h-4 w-4 text-emerald-600" aria-hidden="true"><path d="m5 10 3.2 3.2L15.5 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>Workspace-aware API</span>
              <span class="inline-flex items-center gap-2"><svg viewBox="0 0 20 20" fill="none" class="h-4 w-4 text-emerald-600" aria-hidden="true"><path d="m5 10 3.2 3.2L15.5 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>Built to grow with you</span>
            </div>
          </div>

          <div class="relative mx-auto w-full max-w-[560px]">
            <div class="absolute -left-5 top-20 z-10 hidden rounded-2xl border border-line bg-white p-4 shadow-xl shadow-ink/10 sm:block">
              <div class="flex items-center gap-3">
                <div class="grid h-10 w-10 place-items-center rounded-xl bg-emerald-100 text-emerald-700">
                  <svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="M5 19V9m7 10V5m7 14v-7" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
                </div>
                <div><p class="text-xs text-muted">Learning, organised</p><p class="mt-0.5 text-sm font-semibold">One clear overview</p></div>
              </div>
            </div>

            <div class="absolute -right-3 bottom-20 z-10 hidden rotate-2 rounded-2xl border border-line bg-white p-4 shadow-xl shadow-ink/10 sm:block">
              <div class="flex items-center gap-3">
                <div class="grid h-10 w-10 place-items-center rounded-xl bg-brand-soft text-brand">
                  <svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="m12 3 2.7 5.5 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1-4.4-4.3 6.1-.9L12 3Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" /></svg>
                </div>
                <div><p class="text-xs text-muted">Made for teams</p><p class="mt-0.5 text-sm font-semibold">Your space, your pace</p></div>
              </div>
            </div>

            <div class="overflow-hidden rounded-[28px] border border-white bg-white p-3 shadow-2xl shadow-ink/10 sm:rounded-[32px] sm:p-4">
              <div class="rounded-[22px] bg-canvas p-4 sm:p-5">
                <div class="flex items-center justify-between gap-3">
                  <div class="flex items-center gap-3">
                    <div class="grid h-10 w-10 place-items-center rounded-xl bg-brand text-sm font-black text-white">N</div>
                    <div><p class="text-sm font-semibold">Workspace overview</p><p class="mt-0.5 text-xs text-muted">Your learning hub at a glance</p></div>
                  </div>
                  <span class="rounded-full bg-white px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted ring-1 ring-line">Preview</span>
                </div>

                <div class="mt-5 grid grid-cols-3 gap-2 sm:gap-3">
                  <div class="rounded-2xl border border-line bg-white p-3 sm:p-4">
                    <div class="h-2 w-7 rounded-full bg-brand/30" />
                    <p class="mt-3 text-xl font-semibold tracking-tight sm:text-2xl">Courses</p>
                    <p class="mt-1 text-[11px] text-muted">Your content</p>
                  </div>
                  <div class="rounded-2xl border border-line bg-white p-3 sm:p-4">
                    <div class="h-2 w-7 rounded-full bg-emerald-400/70" />
                    <p class="mt-3 text-xl font-semibold tracking-tight sm:text-2xl">People</p>
                    <p class="mt-1 text-[11px] text-muted">Your learners</p>
                  </div>
                  <div class="rounded-2xl border border-line bg-white p-3 sm:p-4">
                    <div class="h-2 w-7 rounded-full bg-amber-400/80" />
                    <p class="mt-3 text-xl font-semibold tracking-tight sm:text-2xl">Progress</p>
                    <p class="mt-1 text-[11px] text-muted">Next steps</p>
                  </div>
                </div>

                <div class="mt-3 rounded-2xl border border-line bg-white p-4 sm:p-5">
                  <div class="flex items-center justify-between">
                    <div><p class="text-sm font-semibold">Keep good ideas moving</p><p class="mt-1 text-xs text-muted">A calmer way to build your course library</p></div>
                    <span class="grid h-9 w-9 place-items-center rounded-xl bg-brand-soft text-brand"><svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="M12 5v14m-7-7h14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg></span>
                  </div>
                  <div class="mt-5 space-y-3">
                    <div class="flex items-center gap-3 rounded-xl bg-canvas p-3">
                      <div class="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-[#EDE8FF] text-brand"><svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="m4 7 8-4 8 4-8 4-8-4Zm0 5 8 4 8-4M4 17l8 4 8-4" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" /></svg></div>
                      <div class="min-w-0 flex-1"><p class="truncate text-xs font-semibold sm:text-sm">Make learning feel simple</p><p class="mt-1 text-[11px] text-muted">Organised course content</p></div>
                      <span class="h-2 w-2 rounded-full bg-emerald-500" />
                    </div>
                    <div class="flex items-center gap-3 rounded-xl bg-canvas p-3">
                      <div class="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-[#FFF0DB] text-amber-700"><svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="M5 19V5m0 14h14M8 15l3-4 3 2 4-6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" /></svg></div>
                      <div class="min-w-0 flex-1"><p class="truncate text-xs font-semibold sm:text-sm">See the next step</p><p class="mt-1 text-[11px] text-muted">Progress made visible</p></div>
                      <span class="h-2 w-2 rounded-full bg-amber-400" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section id="features" class="mx-auto max-w-7xl px-5 py-20 sm:px-8 sm:py-28">
        <div class="mx-auto max-w-2xl text-center">
          <p class="text-xs font-bold uppercase tracking-[0.22em] text-brand">Built around your work</p>
          <h2 class="mt-4 text-3xl font-semibold tracking-tight sm:text-4xl">Everything starts with a clearer structure.</h2>
          <p class="mt-4 text-base leading-7 text-muted">A practical foundation for the people, content, and workflows behind great learning experiences.</p>
        </div>

        <div class="mt-12 grid gap-5 md:grid-cols-3">
          <article class="group rounded-3xl border border-line bg-white p-7 transition hover:-translate-y-1 hover:border-brand/25 hover:shadow-xl hover:shadow-brand/5">
            <div class="grid h-12 w-12 place-items-center rounded-2xl bg-brand-soft text-brand transition group-hover:bg-brand group-hover:text-white">
              <svg viewBox="0 0 24 24" fill="none" class="h-6 w-6" aria-hidden="true"><path d="M4 6.5A2.5 2.5 0 0 1 6.5 4H20v15H6.5A2.5 2.5 0 0 0 4 21V6.5Zm0 12A2.5 2.5 0 0 1 6.5 16H20M8 8h7m-7 3h9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
            </div>
            <h3 class="mt-6 text-lg font-semibold">Courses with a purpose</h3>
            <p class="mt-3 text-sm leading-7 text-muted">Start with a course, shape its learning content, and control when it is ready to publish.</p>
          </article>
          <article class="group rounded-3xl border border-line bg-white p-7 transition hover:-translate-y-1 hover:border-brand/25 hover:shadow-xl hover:shadow-brand/5">
            <div class="grid h-12 w-12 place-items-center rounded-2xl bg-emerald-100 text-emerald-700 transition group-hover:bg-emerald-600 group-hover:text-white">
              <svg viewBox="0 0 24 24" fill="none" class="h-6 w-6" aria-hidden="true"><path d="M16 20v-1.5a4.5 4.5 0 0 0-4.5-4.5h-5A4.5 4.5 0 0 0 2 18.5V20m7-10a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm7-6.5a4 4 0 0 1 0 7.75M22 20v-1.5a4.5 4.5 0 0 0-3.5-4.38" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
            </div>
            <h3 class="mt-6 text-lg font-semibold">A space for your people</h3>
            <p class="mt-3 text-sm leading-7 text-muted">Keep each learning business in its own workspace with role-aware access to its data.</p>
          </article>
          <article class="group rounded-3xl border border-line bg-white p-7 transition hover:-translate-y-1 hover:border-brand/25 hover:shadow-xl hover:shadow-brand/5">
            <div class="grid h-12 w-12 place-items-center rounded-2xl bg-amber-100 text-amber-700 transition group-hover:bg-amber-500 group-hover:text-white">
              <svg viewBox="0 0 24 24" fill="none" class="h-6 w-6" aria-hidden="true"><path d="M4 19V5m0 14h16M7 15l4-4 3 2 5-6m-4 0h4v4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
            </div>
            <h3 class="mt-6 text-lg font-semibold">Progress you can follow</h3>
            <p class="mt-3 text-sm leading-7 text-muted">Bring lesson progress and completion into the same API foundation as your course library.</p>
          </article>
        </div>
      </section>

      <section id="workflow" class="border-y border-line bg-canvas">
        <div class="mx-auto grid max-w-7xl items-center gap-12 px-5 py-20 sm:px-8 sm:py-24 lg:grid-cols-2">
          <div>
            <p class="text-xs font-bold uppercase tracking-[0.22em] text-brand">A simple place to begin</p>
            <h2 class="mt-4 max-w-xl text-3xl font-semibold tracking-tight sm:text-4xl">From first idea to your own learning workspace.</h2>
            <p class="mt-5 max-w-xl text-base leading-7 text-muted">Create your company workspace, sign in securely, and start organising courses. The foundation is ready to extend as your needs grow.</p>
          </div>
          <div class="space-y-4">
            <div class="flex gap-4 rounded-2xl border border-line bg-white p-5">
              <span class="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-brand text-sm font-bold text-white">01</span>
              <div><h3 class="font-semibold">Create your workspace</h3><p class="mt-1 text-sm leading-6 text-muted">Set up your organisation and owner account in a few steps.</p></div>
            </div>
            <div class="flex gap-4 rounded-2xl border border-line bg-white p-5">
              <span class="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-emerald-100 text-sm font-bold text-emerald-800">02</span>
              <div><h3 class="font-semibold">Build your course library</h3><p class="mt-1 text-sm leading-6 text-muted">Create courses and publish them when they are ready.</p></div>
            </div>
            <div class="flex gap-4 rounded-2xl border border-line bg-white p-5">
              <span class="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-amber-100 text-sm font-bold text-amber-800">03</span>
              <div><h3 class="font-semibold">Keep learning moving</h3><p class="mt-1 text-sm leading-6 text-muted">Extend the workspace with people, lessons, media, and progress workflows.</p></div>
            </div>
          </div>
        </div>
      </section>

      <section class="mx-auto max-w-7xl px-5 py-20 sm:px-8 sm:py-24">
        <div class="relative overflow-hidden rounded-[32px] bg-ink px-7 py-12 text-white sm:px-12 sm:py-16 lg:px-16">
          <div class="absolute -right-10 -top-32 h-80 w-80 rounded-full bg-brand/50 blur-3xl" />
          <div class="absolute -bottom-40 left-1/3 h-72 w-72 rounded-full bg-emerald-300/15 blur-3xl" />
          <div class="relative flex flex-col items-start justify-between gap-8 md:flex-row md:items-center">
            <div class="max-w-2xl">
              <p class="text-xs font-bold uppercase tracking-[0.22em] text-emerald-200">Your next chapter</p>
              <h2 class="mt-4 text-3xl font-semibold tracking-tight sm:text-4xl">Make room for better learning.</h2>
              <p class="mt-4 max-w-xl text-sm leading-7 text-white/65 sm:text-base">Start with the essentials. Build the learning experience you want to see.</p>
            </div>
            <NuxtLink to="/register" class="inline-flex shrink-0 items-center gap-2 rounded-2xl bg-white px-6 py-4 text-sm font-semibold text-ink transition hover:-translate-y-0.5 hover:bg-emerald-100">
              Get started
              <svg viewBox="0 0 20 20" fill="none" class="h-4 w-4" aria-hidden="true"><path d="M4 10h12m-5-5 5 5-5 5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" /></svg>
            </NuxtLink>
          </div>
        </div>
      </section>
    </main>

    <footer class="border-t border-line bg-white">
      <div class="mx-auto flex max-w-7xl flex-col gap-4 px-5 py-7 text-xs text-muted sm:flex-row sm:items-center sm:justify-between sm:px-8">
        <NuxtLink to="/" class="inline-flex items-center gap-2 font-semibold text-ink"><span class="grid h-7 w-7 place-items-center rounded-lg bg-brand text-xs font-black text-white">N</span>newage.</NuxtLink>
        <p>A thoughtful foundation for learning businesses.</p>
        <div class="flex items-center gap-5"><NuxtLink to="/login" class="hover:text-ink">Sign in</NuxtLink><NuxtLink to="/register" class="hover:text-ink">Create workspace</NuxtLink></div>
      </div>
    </footer>
  </div>
</template>
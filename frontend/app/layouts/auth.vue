<template>
  <div class="grid min-h-screen bg-white lg:grid-cols-[0.92fr_1.08fr]">
    <aside class="relative hidden min-h-screen overflow-hidden bg-ink px-10 py-10 text-white lg:flex lg:flex-col xl:px-16">
      <div class="absolute -right-32 top-24 h-96 w-96 rounded-full bg-brand/35 blur-3xl" />
      <div class="absolute -bottom-40 -left-20 h-96 w-96 rounded-full bg-emerald-400/15 blur-3xl" />
      <NuxtLink to="/" class="relative z-10 inline-flex w-fit items-center gap-3 font-semibold tracking-tight text-white">
        <span class="grid h-10 w-10 place-items-center rounded-2xl bg-white text-lg font-black text-brand">N</span>
        <span class="text-xl">newage<span class="text-white/50">.</span></span>
      </NuxtLink>

      <div class="relative z-10 my-auto max-w-lg pb-10 pt-16">
        <div class="mb-6 inline-flex items-center gap-2 rounded-full border border-white/15 bg-white/5 px-3 py-1.5 text-xs font-medium text-white/80">
          <span class="h-2 w-2 rounded-full bg-emerald-300" />
          Your next chapter starts here
        </div>
        <h1 class="text-4xl font-semibold leading-tight tracking-tight xl:text-5xl">
          Learning should feel
          <span class="text-emerald-200">clear, connected,</span>
          and full of possibility.
        </h1>
        <p class="mt-5 max-w-md text-base leading-7 text-white/65">
          Bring your courses, learners, and everyday progress into one thoughtful workspace.
        </p>

        <div class="mt-10 rounded-3xl border border-white/10 bg-white/[0.07] p-5 backdrop-blur-sm">
          <div class="flex items-center justify-between">
            <p class="text-sm font-medium text-white/75">Designed for focus</p>
            <span class="rounded-full bg-emerald-300/15 px-2.5 py-1 text-xs font-medium text-emerald-100">One workspace</span>
          </div>
          <div class="mt-5 flex items-end gap-2">
            <div class="h-12 flex-1 rounded-t-xl bg-white/15" />
            <div class="h-20 flex-1 rounded-t-xl bg-white/20" />
            <div class="h-16 flex-1 rounded-t-xl bg-brand/80" />
            <div class="h-28 flex-1 rounded-t-xl bg-emerald-200" />
            <div class="h-24 flex-1 rounded-t-xl bg-white/25" />
            <div class="h-32 flex-1 rounded-t-xl bg-white/15" />
          </div>
          <div class="mt-4 flex items-center justify-between border-t border-white/10 pt-4 text-xs text-white/55">
            <span>Build at your own pace</span>
            <span>Newage LMS</span>
          </div>
        </div>
      </div>

      <p class="relative z-10 text-xs text-white/40">A calmer place to teach, learn, and grow.</p>
    </aside>

    <main class="flex min-h-screen items-center justify-center px-5 py-10 sm:px-8">
      <div class="w-full max-w-md">
        <div class="mb-8 flex items-center justify-between lg:hidden">
          <NuxtLink to="/" class="inline-flex items-center gap-2.5 font-semibold tracking-tight">
            <span class="grid h-9 w-9 place-items-center rounded-xl bg-brand text-lg font-black text-white">N</span>
            <span class="text-xl">newage<span class="text-brand">.</span></span>
          </NuxtLink>
          <NuxtLink to="/" class="text-sm font-medium text-muted hover:text-ink">Home</NuxtLink>
        </div>
        <slot />
      </div>
    </main>
  </div>
</template>
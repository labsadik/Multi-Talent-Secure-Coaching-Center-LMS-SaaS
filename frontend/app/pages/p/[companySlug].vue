<script setup lang="ts">
import type { PublicCompanyPage } from '~/types/api'

definePageMeta({ layout: 'marketing' })

const route = useRoute()
const companySlug = computed(() => String(route.params.companySlug || ''))
const { currentUser, loadCurrentUser } = useAuth()
const { data: page, pending, error } = await useAsyncData<PublicCompanyPage>(
  () => 'public-company-' + companySlug.value,
  () => $fetch<PublicCompanyPage>('/api/public/companies/' + encodeURIComponent(companySlug.value)),
)
const brandStyle = computed(() => ({
  '--site-brand': page.value?.site.brandColor || '#2563eb',
  '--site-accent': page.value?.site.accentColor || '#14b8a6',
}))
function courseAction(courseId: string) {
  if (currentUser.value?.role === 'learner') return '/dashboard/learn?checkoutCourse=' + encodeURIComponent(courseId)
  return '/learn/register?companySlug=' + encodeURIComponent(companySlug.value) + '&courseId=' + encodeURIComponent(courseId)
}
function formatPrice(paise: number, currency = 'INR') {
  return paise === 0 ? 'Free' : new Intl.NumberFormat('en-IN', {
    style: 'currency', currency, maximumFractionDigits: 2,
  }).format(paise / 100)
}
onMounted(async () => {
  try { await loadCurrentUser() } catch { /* public storefronts are accessible without a session */ }
})
</script>

<template>
  <div class="min-h-screen bg-white" :style="brandStyle">
    <div v-if="pending" class="grid min-h-screen place-items-center bg-canvas px-6 text-center"><div><div class="mx-auto h-10 w-10 animate-spin rounded-full border-4 border-line border-t-brand" /><p class="mt-4 text-sm text-muted">Loading company courses…</p></div></div>
    <div v-else-if="error || !page" class="grid min-h-screen place-items-center bg-canvas px-6 text-center">
      <div class="max-w-lg rounded-3xl border border-line bg-white p-8 shadow-sm sm:p-10"><div class="mx-auto grid h-14 w-14 place-items-center rounded-2xl bg-brand-soft text-brand"><span class="text-2xl font-black">N</span></div><h1 class="mt-5 text-2xl font-semibold tracking-tight">This public page is not available yet.</h1><p class="mt-3 text-sm leading-6 text-muted">The company may not have published its page, or the link may be incorrect. Ask the company for its public learning link.</p><NuxtLink to="/" class="mt-6 inline-flex rounded-xl px-5 py-3 text-sm font-semibold text-white" :style="{ backgroundColor: '#2563eb' }">Visit Newage</NuxtLink></div>
    </div>
    <template v-else>
      <header class="sticky top-0 z-30 border-b border-white/30 bg-white/90 backdrop-blur-xl">
        <div class="mx-auto flex h-[76px] max-w-7xl items-center justify-between gap-4 px-4 sm:px-6 lg:px-8">
          <NuxtLink :to="'/p/' + page.company.slug" class="flex min-w-0 items-center gap-3">
            <img v-if="page.site.logoUrl" :src="page.site.logoUrl" :alt="page.site.displayName + ' logo'" class="h-10 w-10 shrink-0 rounded-xl border border-line bg-white object-contain p-1" />
            <span v-else class="grid h-10 w-10 shrink-0 place-items-center rounded-xl text-lg font-black text-white" :style="{ backgroundColor: page.site.brandColor }">{{ page.site.displayName.slice(0,1).toUpperCase() }}</span>
            <span class="truncate text-base font-semibold tracking-tight sm:text-lg">{{ page.site.displayName }}</span>
          </NuxtLink>
          <nav class="flex shrink-0 items-center gap-2 sm:gap-3" aria-label="Company page navigation">
            <a href="#courses" class="hidden rounded-xl px-4 py-2.5 text-sm font-semibold text-muted hover:bg-canvas sm:inline-flex">Courses</a>
            <NuxtLink :to="'/workspace/' + encodeURIComponent(page.company.slug) + '/login'" class="rounded-xl border border-line px-3 py-2.5 text-xs font-semibold sm:px-4 sm:text-sm">Company sign in</NuxtLink>
            <NuxtLink :to="'/learn/register?companySlug=' + encodeURIComponent(page.company.slug)" class="rounded-xl px-3 py-2.5 text-xs font-semibold text-white sm:px-4 sm:text-sm" :style="{ backgroundColor: page.site.brandColor }">Join to learn</NuxtLink>
          </nav>
        </div>
      </header>

      <main>
        <section class="relative isolate overflow-hidden bg-slate-950">
          <div v-if="page.site.heroImageUrl" class="absolute inset-0 -z-20 bg-cover bg-center" :style="{ backgroundImage: 'url(' + JSON.stringify(page.site.heroImageUrl) + ')' }" />
          <div class="absolute inset-0 -z-10" :style="{ background: 'linear-gradient(105deg, ' + page.site.brandColor + 'ef 0%, ' + page.site.brandColor + 'b8 48%, #101827d9 100%)' }" />
          <div class="absolute -right-32 -top-32 -z-10 h-96 w-96 rounded-full border-[48px] border-white/10" />
          <div class="mx-auto grid max-w-7xl items-center gap-10 px-4 py-16 sm:px-6 sm:py-24 lg:grid-cols-[1.05fr_.95fr] lg:px-8 lg:py-28">
            <div>
              <div class="inline-flex items-center gap-2 rounded-full border border-white/20 bg-white/10 px-3 py-1.5 text-xs font-semibold text-white/85"><span class="h-2 w-2 rounded-full" :style="{ backgroundColor: page.site.accentColor }" /> Company learning hub</div>
              <h1 class="mt-6 max-w-3xl text-4xl font-semibold leading-[1.08] tracking-tight text-white sm:text-5xl lg:text-6xl">{{ page.site.headline }}</h1>
              <p class="mt-5 max-w-2xl text-base leading-7 text-white/80 sm:text-lg">{{ page.site.tagline }}</p>
              <p v-if="page.site.about" class="mt-5 max-w-2xl text-sm leading-7 text-white/70">{{ page.site.about }}</p>
              <div class="mt-8 flex flex-wrap gap-3"><a href="#courses" class="inline-flex items-center gap-2 rounded-xl px-5 py-3.5 text-sm font-semibold text-white shadow-lg transition hover:-translate-y-0.5" :style="{ backgroundColor: page.site.accentColor }">Explore courses <span aria-hidden="true">→</span></a><NuxtLink :to="'/learn/register?companySlug=' + encodeURIComponent(page.company.slug)" class="inline-flex rounded-xl border border-white/25 bg-white/10 px-5 py-3.5 text-sm font-semibold text-white hover:bg-white/15">Create learner account</NuxtLink></div>
              <div class="mt-10 flex flex-wrap items-center gap-5 text-xs font-medium text-white/65"><span>{{ page.courses.length }} published courses</span><span class="h-1 w-1 rounded-full bg-white/40" /><span>Secure course access</span><span v-if="page.site.contactEmail" class="h-1 w-1 rounded-full bg-white/40" /><a v-if="page.site.contactEmail" :href="'mailto:' + page.site.contactEmail" class="underline underline-offset-4 hover:text-white">Contact us</a></div>
            </div>
            <div class="relative mx-auto w-full max-w-lg">
              <div class="absolute -inset-3 rounded-[2rem] blur-2xl opacity-30" :style="{ backgroundColor: page.site.accentColor }" />
              <div class="relative overflow-hidden rounded-[1.7rem] border border-white/20 bg-white p-4 shadow-2xl sm:p-5">
                <div class="flex items-center justify-between gap-3 border-b border-line pb-4"><div><p class="text-[10px] font-bold uppercase tracking-[0.18em]" :style="{ color: page.site.brandColor }">Featured learning</p><p class="mt-1 text-sm font-semibold text-ink">Explore your next skill</p></div><span class="grid h-10 w-10 place-items-center rounded-xl text-white" :style="{ backgroundColor: page.site.brandColor }">↗</span></div>
                <div v-if="page.courses[0]" class="mt-4 overflow-hidden rounded-2xl bg-canvas">
                  <img v-if="page.courses[0].thumbnailUrl" :src="page.courses[0].thumbnailUrl" :alt="page.courses[0].title + ' thumbnail'" class="h-44 w-full object-cover sm:h-52" />
                  <div v-else class="grid h-44 place-items-center text-4xl font-black text-white sm:h-52" :style="{ background: 'linear-gradient(135deg, ' + page.site.brandColor + ', ' + page.site.accentColor + ')' }">{{ page.courses[0].title.slice(0,1).toUpperCase() }}</div>
                  <div class="p-4 sm:p-5"><div class="flex items-start justify-between gap-3"><h2 class="text-lg font-semibold leading-6 text-ink">{{ page.courses[0].title }}</h2><span class="shrink-0 rounded-lg bg-white px-2.5 py-1.5 text-xs font-bold text-ink ring-1 ring-line">{{ formatPrice(page.courses[0].priceInPaise, page.courses[0].currency) }}</span></div><p class="mt-2 line-clamp-2 text-sm leading-6 text-muted">{{ page.courses[0].description || 'A learning experience from ' + page.site.displayName + '.' }}</p><NuxtLink :to="courseAction(page.courses[0].id)" class="mt-4 flex items-center justify-center rounded-xl px-4 py-3 text-sm font-semibold text-white" :style="{ backgroundColor: page.site.brandColor }">{{ page.courses[0].priceInPaise ? 'Enrol & pay securely' : 'Enrol for free' }}</NuxtLink></div>
                </div>
                <div v-else class="mt-4 rounded-2xl bg-canvas px-5 py-12 text-center"><p class="text-sm font-semibold text-ink">Courses are coming soon</p><p class="mt-2 text-xs leading-5 text-muted">This company will show published courses here.</p></div>
              </div>
            </div>
          </div>
        </section>

        <section id="courses" class="mx-auto max-w-7xl px-4 py-14 sm:px-6 sm:py-20 lg:px-8">
          <div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-end"><div><p class="text-xs font-bold uppercase tracking-[0.18em]" :style="{ color: page.site.brandColor }">Course catalogue</p><h2 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Find your next course.</h2><p class="mt-3 max-w-2xl text-sm leading-6 text-muted">Browse programmes published by {{ page.site.displayName }}. Create a learner account to enrol and check out.</p></div><span class="w-fit rounded-full bg-canvas px-3 py-2 text-xs font-semibold text-muted">{{ page.courses.length }} available</span></div>
          <div v-if="page.courses.length" class="mt-8 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            <article v-for="course in page.courses" :key="course.id" class="group flex min-w-0 flex-col overflow-hidden rounded-2xl border border-line bg-white transition hover:-translate-y-1 hover:shadow-xl hover:shadow-ink/5">
              <img v-if="course.thumbnailUrl" :src="course.thumbnailUrl" :alt="course.title + ' thumbnail'" class="h-48 w-full object-cover" loading="lazy" />
              <div v-else class="grid h-48 place-items-center text-5xl font-black text-white" :style="{ background: 'linear-gradient(135deg, ' + page.site.brandColor + ', ' + page.site.accentColor + ')' }">{{ course.title.slice(0,1).toUpperCase() }}</div>
              <div class="flex flex-1 flex-col p-5"><div class="flex items-start justify-between gap-3"><h3 class="text-lg font-semibold leading-6">{{ course.title }}</h3><span class="shrink-0 rounded-lg bg-canvas px-2.5 py-1.5 text-xs font-bold">{{ formatPrice(course.priceInPaise, course.currency) }}</span></div><p class="mt-3 line-clamp-3 flex-1 text-sm leading-6 text-muted">{{ course.description || 'Explore this programme from ' + page.site.displayName + '.' }}</p><NuxtLink :to="courseAction(course.id)" class="mt-5 flex w-full items-center justify-center rounded-xl px-4 py-3 text-sm font-semibold text-white transition hover:opacity-90" :style="{ backgroundColor: page.site.brandColor }">{{ course.priceInPaise ? 'Enrol & pay securely' : 'Enrol for free' }} <span class="ml-2">→</span></NuxtLink></div>
            </article>
          </div>
          <div v-else class="mt-8 rounded-3xl border border-dashed border-slate-300 bg-canvas px-6 py-16 text-center"><h3 class="text-lg font-semibold">No courses published yet</h3><p class="mt-2 text-sm leading-6 text-muted">Please check back soon. The course catalogue will appear here after the company publishes courses.</p></div>
        </section>
      </main>
      <footer class="border-t border-line bg-canvas"><div class="mx-auto flex max-w-7xl flex-col gap-3 px-4 py-7 text-xs text-muted sm:flex-row sm:items-center sm:justify-between sm:px-6 lg:px-8"><p>© {{ new Date().getFullYear() }} {{ page.site.displayName }} · Learning that moves you forward.</p><p>Powered by Newage learning platform</p></div></footer>
    </template>
  </div>
</template>

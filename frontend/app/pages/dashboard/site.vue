<script setup lang="ts">
import type { CompanySiteReply, SiteSettings } from '~/types/api'
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: ['auth', 'company-dashboard'] })

const route = useRoute()
function workspacePath(suffix = '') {
  const slug = String(route.params.companySlug || currentCompany.value?.slug || '').trim()
  return slug ? '/workspace/' + encodeURIComponent(slug) + '/dashboard' + suffix : '/dashboard' + suffix
}
const { currentUser, currentCompany, getCompanySite, updateCompanySite } = useAuth()
const loading = ref(true)
const saving = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const companySlug = ref('')
const publicPath = ref('')
const site = ref<SiteSettings>({
  displayName: '',
  headline: '',
  tagline: 'Practical learning, built around you.',
  about: '',
  theme: 'ocean',
  brandColor: '#2563eb',
  accentColor: '#14b8a6',
  logoUrl: '',
  heroImageUrl: '',
  contactEmail: '',
  isPublished: false,
})
const presets = [
  { id: 'ocean', name: 'Ocean', brand: '#2563eb', accent: '#14b8a6', description: 'Clean and professional' },
  { id: 'forest', name: 'Forest', brand: '#166534', accent: '#d97706', description: 'Natural and grounded' },
  { id: 'sunset', name: 'Sunset', brand: '#c2410c', accent: '#db2777', description: 'Warm and creative' },
  { id: 'midnight', name: 'Midnight', brand: '#7c3aed', accent: '#38bdf8', description: 'Bold and modern' },
  { id: 'minimal', name: 'Minimal', brand: '#18181b', accent: '#64748b', description: 'Quiet and understated' },
]
const canConfigure = computed(() => ['owner', 'admin'].includes(currentUser.value?.role ?? ''))
const publicUrl = computed(() => publicPath.value || (companySlug.value ? '/p/' + companySlug.value : '/'))

async function load() {
  loading.value = true
  errorMessage.value = ''
  try {
    const reply: CompanySiteReply = await getCompanySite()
    companySlug.value = reply.company.slug
    publicPath.value = reply.publicPath
    site.value = { ...site.value, ...reply.site }
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Public site settings could not be loaded.')
  } finally {
    loading.value = false
  }
}
function choosePreset(preset: typeof presets[number]) {
  site.value.theme = preset.id
  site.value.brandColor = preset.brand
  site.value.accentColor = preset.accent
}
async function save() {
  errorMessage.value = ''
  successMessage.value = ''
  saving.value = true
  try {
    const reply = await updateCompanySite({
      ...site.value,
      logoUrl: site.value.logoUrl?.trim() || null,
      heroImageUrl: site.value.heroImageUrl?.trim() || null,
      contactEmail: site.value.contactEmail?.trim() || null,
    })
    site.value = { ...site.value, ...reply.site }
    publicPath.value = reply.publicPath
    successMessage.value = site.value.isPublished
      ? 'Your public company page is published and updated.'
      : 'Settings saved. Publish the page when you are ready.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Site settings could not be saved. Use secure HTTPS image URLs.')
  } finally {
    saving.value = false
  }
}
function openPublicPage() {
  if (!import.meta.client) return
  window.open(publicUrl.value, '_blank', 'noopener,noreferrer')
}
onMounted(() => { void load() })
</script>

<template>
  <div class="min-h-screen bg-canvas">
    <header class="sticky top-0 z-30 border-b border-line bg-white/95 backdrop-blur-xl">
      <div class="mx-auto flex h-[72px] max-w-7xl items-center justify-between gap-4 px-4 sm:px-6 lg:px-8">
        <NuxtLink :to="workspacePath()" class="inline-flex items-center gap-3 font-semibold"><span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white">N</span>newage<span class="text-brand">.</span></NuxtLink>
        <div class="flex items-center gap-2">
          <button type="button" class="rounded-xl border border-line px-3 py-2.5 text-xs font-semibold text-muted hover:bg-canvas sm:px-4 sm:text-sm" :disabled="!companySlug" @click="openPublicPage">View public page ↗</button>
          <NuxtLink :to="workspacePath()" class="rounded-xl bg-brand px-3 py-2.5 text-xs font-semibold text-white hover:bg-brand-dark sm:px-4 sm:text-sm">Workspace</NuxtLink>
        </div>
      </div>
      <nav class="mx-auto flex max-w-7xl items-center gap-2 overflow-x-auto px-4 pb-3 sm:px-6 lg:px-8" aria-label="Workspace sections">
        <NuxtLink :to="workspacePath()" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-muted hover:bg-canvas">Courses</NuxtLink>
        <NuxtLink :to="workspacePath('/site')" class="shrink-0 rounded-lg bg-brand-soft px-3 py-2 text-xs font-semibold text-brand">Public site & theme</NuxtLink>
        <NuxtLink :to="workspacePath('/settings')" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-muted hover:bg-canvas">Payments & plans</NuxtLink>
        <NuxtLink :to="workspacePath('/users')" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-muted hover:bg-canvas">People</NuxtLink>
      </nav>
    </header>

    <main class="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
      <div class="mb-8">
        <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Company storefront</p>
        <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Your public page, your brand.</h1>
        <p class="mt-3 max-w-3xl text-sm leading-6 text-muted">Give learners a public home for your company, publish your course catalogue, set your colours, and share one company link.</p>
        <div class="mt-4 flex flex-wrap items-center gap-2">
          <span class="inline-flex items-center gap-2 rounded-full border border-line bg-white px-3 py-2 text-xs font-medium text-muted"><span class="h-2 w-2 rounded-full" :class="site.isPublished ? 'bg-emerald-500' : 'bg-amber-500'" />{{ site.isPublished ? 'Public page is published' : 'Not public yet' }}</span>
          <code class="rounded-lg bg-white px-3 py-2 text-xs text-brand ring-1 ring-line">{{ publicUrl }}</code>
        </div>
      </div>

      <div v-if="errorMessage" role="alert" class="mb-5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{{ errorMessage }}</div>
      <div v-if="successMessage" role="status" class="mb-5 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">{{ successMessage }}</div>
      <div v-if="loading" class="rounded-2xl border border-line bg-white p-8 text-sm text-muted">Loading your company site…</div>

      <div v-else class="grid items-start gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(360px,0.92fr)]">
        <form class="space-y-6" @submit.prevent="save">
          <section class="rounded-2xl border border-line bg-white p-5 sm:p-6">
            <div><p class="text-xs font-bold uppercase tracking-[0.16em] text-brand">01 / Identity</p><h2 class="mt-2 text-lg font-semibold">Company page content</h2><p class="mt-1 text-sm text-muted">This copy appears on your public company page.</p></div>
            <div class="mt-5 grid gap-4 sm:grid-cols-2">
              <div><label for="displayName" class="mb-2 block text-sm font-semibold">Public company name</label><input id="displayName" v-model="site.displayName" required minlength="2" maxlength="100" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
              <div><label for="contactEmail" class="mb-2 block text-sm font-semibold">Contact email</label><input id="contactEmail" v-model="site.contactEmail" type="email" maxlength="254" placeholder="learn@example.com" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
              <div class="sm:col-span-2"><label for="headline" class="mb-2 block text-sm font-semibold">Main headline</label><input id="headline" v-model="site.headline" required minlength="2" maxlength="160" placeholder="Turn your team's knowledge into skills" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
              <div class="sm:col-span-2"><label for="tagline" class="mb-2 block text-sm font-semibold">Short tagline</label><input id="tagline" v-model="site.tagline" maxlength="240" placeholder="Practical learning, built around you." class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
              <div class="sm:col-span-2"><label for="about" class="mb-2 block text-sm font-semibold">About your company / learning programme</label><textarea id="about" v-model="site.about" rows="4" maxlength="10000" placeholder="Tell learners what you teach and why it matters…" class="w-full resize-y rounded-xl border border-line px-3.5 py-3 text-sm leading-6 outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
              <div><label for="logoUrl" class="mb-2 block text-sm font-semibold">Logo image URL <span class="font-normal text-muted">(HTTPS)</span></label><input id="logoUrl" v-model="site.logoUrl" type="url" placeholder="https://…" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /><p class="mt-1.5 text-xs text-muted">Use a public HTTPS image URL.</p></div>
              <div><label for="heroImageUrl" class="mb-2 block text-sm font-semibold">Hero / banner image URL <span class="font-normal text-muted">(HTTPS)</span></label><input id="heroImageUrl" v-model="site.heroImageUrl" type="url" placeholder="https://…" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
            </div>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5 sm:p-6">
            <div><p class="text-xs font-bold uppercase tracking-[0.16em] text-brand">02 / Theme</p><h2 class="mt-2 text-lg font-semibold">Choose the look and feel</h2><p class="mt-1 text-sm text-muted">Pick a starter palette, then adjust the two colours.</p></div>
            <div class="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <button v-for="preset in presets" :key="preset.id" type="button" class="rounded-xl border p-3 text-left transition hover:-translate-y-0.5" :class="site.theme === preset.id ? 'border-brand ring-2 ring-brand/15' : 'border-line'" @click="choosePreset(preset)">
                <div class="flex items-center gap-2"><span class="h-7 w-7 rounded-lg" :style="{ backgroundColor: preset.brand }" /><span class="h-7 w-7 rounded-lg" :style="{ backgroundColor: preset.accent }" /><span class="ml-auto text-[10px] font-bold uppercase tracking-wider text-muted">{{ site.theme === preset.id ? 'Selected' : 'Theme' }}</span></div>
                <p class="mt-3 text-sm font-semibold">{{ preset.name }}</p><p class="mt-1 text-xs text-muted">{{ preset.description }}</p>
              </button>
            </div>
            <div class="mt-5 grid gap-4 sm:grid-cols-2">
              <div class="flex items-center gap-3 rounded-xl border border-line p-3"><input v-model="site.brandColor" type="color" aria-label="Brand colour" class="h-10 w-12 cursor-pointer rounded-lg border-0 bg-transparent" /><div class="min-w-0"><p class="text-sm font-semibold">Brand colour</p><p class="font-mono text-xs text-muted">{{ site.brandColor }}</p></div></div>
              <div class="flex items-center gap-3 rounded-xl border border-line p-3"><input v-model="site.accentColor" type="color" aria-label="Accent colour" class="h-10 w-12 cursor-pointer rounded-lg border-0 bg-transparent" /><div class="min-w-0"><p class="text-sm font-semibold">Accent colour</p><p class="font-mono text-xs text-muted">{{ site.accentColor }}</p></div></div>
            </div>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5 sm:p-6">
            <p class="text-xs font-bold uppercase tracking-[0.16em] text-brand">03 / Publishing</p><h2 class="mt-2 text-lg font-semibold">Make the public page live</h2>
            <p class="mt-1 text-sm leading-6 text-muted">Visitors can see the page and published courses. Draft courses stay private. Paid course checkout goes through the learner account and the company's Razorpay setup.</p>
            <label class="mt-5 flex items-start gap-3 rounded-xl border border-line bg-canvas p-4"><input v-model="site.isPublished" type="checkbox" class="mt-1 h-4 w-4 rounded accent-brand" /><span><span class="block text-sm font-semibold">Publish public company page</span><span class="mt-1 block text-xs leading-5 text-muted">Turn this off to hide the page from visitors while you keep editing.</span></span></label>
            <div class="mt-5 flex flex-col gap-3 sm:flex-row"><button type="submit" :disabled="saving || !canConfigure" class="flex-1 rounded-xl px-5 py-3.5 text-sm font-semibold text-white transition disabled:opacity-50" :style="{ backgroundColor: site.brandColor }">{{ saving ? 'Saving…' : 'Save & update public page' }}</button><button type="button" class="rounded-xl border border-line px-5 py-3.5 text-sm font-semibold text-muted hover:bg-canvas" @click="load">Discard changes</button></div>
            <p v-if="!canConfigure" class="mt-3 text-xs text-amber-700">Only company owners and admins can change public page settings.</p>
          </section>
        </form>

        <aside class="xl:sticky xl:top-32">
          <div class="mb-3 flex items-center justify-between gap-3"><div><p class="text-xs font-bold uppercase tracking-[0.16em] text-brand">Live preview</p><h2 class="mt-1 text-lg font-semibold">Your public page</h2></div><span class="rounded-full bg-white px-3 py-1.5 text-[10px] font-semibold text-muted ring-1 ring-line">Preview</span></div>
          <div class="overflow-hidden rounded-3xl border border-line bg-white shadow-xl shadow-ink/5">
            <div class="relative flex min-h-52 flex-col justify-between overflow-hidden p-5 sm:p-7" :style="{ backgroundColor: site.brandColor, backgroundImage: site.heroImageUrl ? 'linear-gradient(110deg, rgba(10,10,20,.66), rgba(10,10,20,.12)), url(' + JSON.stringify(site.heroImageUrl) + ')' : undefined, backgroundSize: 'cover', backgroundPosition: 'center' }">
              <div class="relative flex items-center gap-3"><img v-if="site.logoUrl" :src="site.logoUrl" alt="Company logo" class="h-10 w-10 rounded-xl bg-white object-contain p-1" /><div v-else class="grid h-10 w-10 place-items-center rounded-xl bg-white/15 font-black text-white">{{ (site.displayName || currentCompany?.name || 'C').slice(0,1).toUpperCase() }}</div><p class="truncate text-sm font-semibold text-white">{{ site.displayName || currentCompany?.name || 'Your company' }}</p></div>
              <div class="relative mt-8"><p class="text-[10px] font-bold uppercase tracking-[0.2em] text-white/70">Learn with us</p><h3 class="mt-2 max-w-md text-2xl font-semibold leading-tight text-white">{{ site.headline || 'Your company learning hub' }}</h3><p class="mt-2 text-sm leading-6 text-white/80">{{ site.tagline || 'Practical learning, built around you.' }}</p><span class="mt-5 inline-flex rounded-xl px-4 py-2.5 text-xs font-semibold text-white" :style="{ backgroundColor: site.accentColor }">Explore courses</span></div>
            </div>
            <div class="p-5 sm:p-6"><div class="flex items-end justify-between gap-3"><div><p class="text-[10px] font-bold uppercase tracking-[0.16em]" :style="{ color: site.brandColor }">Course catalogue</p><h3 class="mt-1 text-lg font-semibold">Learn something new</h3></div><span class="text-[10px] text-muted">Published courses</span></div>
              <p v-if="site.about" class="mt-3 line-clamp-3 text-xs leading-5 text-muted">{{ site.about }}</p>
              <div class="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-1"><div class="overflow-hidden rounded-xl border border-line"><div class="grid h-20 place-items-center bg-canvas text-xs font-semibold text-muted">Course thumbnail</div><div class="p-3"><p class="text-sm font-semibold">Your published course</p><p class="mt-1 text-xs text-muted">Price and enrolment are shown here</p><span class="mt-3 inline-flex rounded-lg px-3 py-2 text-[10px] font-semibold text-white" :style="{ backgroundColor: site.brandColor }">View course</span></div></div></div>
              <p v-if="site.contactEmail" class="mt-4 border-t border-line pt-4 text-xs text-muted">Contact: {{ site.contactEmail }}</p>
            </div>
          </div>
          <p class="mt-3 text-xs leading-5 text-muted">This is a design preview. Save settings, publish the page, and open the public URL to view real published courses.</p>
        </aside>
      </div>
    </main>
  </div>
</template>

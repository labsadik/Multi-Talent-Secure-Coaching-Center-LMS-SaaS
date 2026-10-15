<script setup lang="ts">
import type { Course, CreateCourseInput } from '~/types/api'
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: ['auth', 'company-dashboard'], alias: ['/workspace/:companySlug/dashboard'] })

const {
  currentUser,
  currentCompany,
  loadCurrentUser,
  getCourses,
  createCourse,
  updateCourse,
  deleteCourse,
  publishCourse,
  logout,
  clearSession,
} = useAuth()

const route = useRoute()
const courses = ref<Course[]>([])

function workspacePath(suffix = '') {
  const slug = String(route.params.companySlug || currentCompany.value?.slug || '').trim()
  return slug ? '/workspace/' + encodeURIComponent(slug) + '/dashboard' + suffix : '/dashboard' + suffix
}
const loading = ref(true)
const busy = ref(false)
const createOpen = ref(false)
const editingCourseId = ref('')
const pageError = ref('')
const successMessage = ref('')
const title = ref('')
const slug = ref('')
const description = ref('')
const thumbnailUrl = ref('')
const priceRupees = ref(0)
const slugTouched = ref(false)

const canManageContent = computed(() =>
  ['owner', 'admin', 'instructor'].includes(currentUser.value?.role ?? ''),
)
const publishedCount = computed(() => courses.value.filter((course) => course.status === 'published').length)
const draftCount = computed(() => courses.value.filter((course) => course.status === 'draft').length)
const userInitials = computed(() => {
  const parts = (currentUser.value?.name ?? 'N').trim().split(/\s+/).filter(Boolean)
  return parts.slice(0, 2).map((part) => part.charAt(0).toUpperCase()).join('')
})

watch(title, (value) => {
  if (!slugTouched.value) {
    slug.value = slugify(value)
  }
})

function slugify(value: string) {
  return value.trim().toLowerCase().normalize('NFKD').replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 80)
}

function formatPrice(paise: number, currency = 'INR') {
  if (paise === 0) return 'Free'
  return new Intl.NumberFormat('en-IN', {
    style: 'currency',
    currency,
    maximumFractionDigits: 2,
  }).format(paise / 100)
}

async function loadDashboard() {
  loading.value = true
  pageError.value = ''
  try {
    await loadCurrentUser()
    courses.value = await getCourses()
  } catch (error) {
    const statusCode = (error as { statusCode?: number } | null)?.statusCode
    if (statusCode === 401) {
      clearSession()
      await navigateTo('/login')
      return
    }
    pageError.value = apiErrorMessage(error, 'The workspace could not be loaded. Check that the API is running.')
  } finally {
    loading.value = false
  }
}

async function submitCourse() {
  successMessage.value = ''
  pageError.value = ''
  busy.value = true
  try {
    const payload: CreateCourseInput = {
      title: title.value.trim(),
      slug: slug.value.trim().toLowerCase(),
      description: description.value.trim() || undefined,
      thumbnailUrl: thumbnailUrl.value.trim() || undefined,
      priceInPaise: Math.round(Number(priceRupees.value || 0) * 100),
    }
    if (editingCourseId.value) {
      const updated = await updateCourse(editingCourseId.value, payload)
      courses.value = courses.value.map((item) => item.id === updated.id ? updated : item)
      successMessage.value = 'Course details saved.'
    } else {
      const created = await createCourse(payload)
      courses.value = [created, ...courses.value]
      successMessage.value = 'Course created as a draft.'
    }
    editingCourseId.value = ''
    title.value = ''
    slug.value = ''
    description.value = ''
    thumbnailUrl.value = ''
    priceRupees.value = 0
    slugTouched.value = false
    createOpen.value = false
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'The course could not be saved. Check the fields and try again.')
  } finally {
    busy.value = false
  }
}

function editCourse(course: Course) {
  editingCourseId.value = course.id
  title.value = course.title
  slug.value = course.slug
  description.value = course.description ?? ''
  thumbnailUrl.value = course.thumbnailUrl ?? ''
  priceRupees.value = course.priceInPaise / 100
  slugTouched.value = true
  createOpen.value = true
  pageError.value = ''
  successMessage.value = ''
  if (import.meta.client) window.scrollTo({ top: 0, behavior: 'smooth' })
}

async function archiveCourse(course: Course) {
  if (!import.meta.client || !window.confirm('Archive "' + course.title + '" and hide it from the course library? This keeps historical enrolment and payment records.')) return
  pageError.value = ''
  successMessage.value = ''
  try {
    await deleteCourse(course.id)
    courses.value = courses.value.filter((item) => item.id !== course.id)
    if (editingCourseId.value === course.id) {
      editingCourseId.value = ''
      createOpen.value = false
    }
    successMessage.value = 'Course archived.'
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'The course could not be archived.')
  }
}

async function publish(course: Course) {
  pageError.value = ''
  successMessage.value = ''
  try {
    const updated = await publishCourse(course.id)
    courses.value = courses.value.map((item) => item.id === updated.id ? updated : item)
    successMessage.value = 'Course published successfully.'
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'This course could not be published.')
  }
}

async function signOut() {
  await logout()
  await navigateTo('/login')
}

onMounted(() => {
  void loadDashboard()
})
</script>

<template>
  <div class="min-h-screen bg-canvas">
    <header class="sticky top-0 z-30 border-b border-line bg-white/95 backdrop-blur-xl">
      <div class="mx-auto flex h-[72px] max-w-7xl items-center justify-between gap-4 px-4 sm:px-6 lg:px-8">
        <NuxtLink to="/" class="inline-flex items-center gap-3 font-semibold tracking-tight">
          <span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white">N</span>
          <span class="text-xl">newage<span class="text-brand">.</span></span>
        </NuxtLink>

        <div class="hidden min-w-0 flex-1 sm:block sm:px-8">
          <div class="mx-auto flex max-w-md items-center gap-3 rounded-xl border border-line bg-canvas px-3.5 py-2.5 text-xs text-muted">
            <svg viewBox="0 0 20 20" fill="none" class="h-4 w-4 shrink-0" aria-hidden="true"><path d="M9 15.5a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13ZM14 14l4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
            <span class="truncate">{{ currentCompany?.name || 'Your workspace' }}</span>
            <span class="ml-auto shrink-0 rounded-md bg-white px-2 py-1 text-[10px] font-medium text-muted ring-1 ring-line">Workspace</span>
          </div>
        </div>

        <div class="flex items-center gap-3">
          <div class="hidden text-right sm:block">
            <p class="max-w-36 truncate text-sm font-semibold">{{ currentUser?.name || 'Loading…' }}</p>
            <p class="mt-0.5 text-[11px] capitalize text-muted">{{ currentUser?.role || 'Member' }}</p>
          </div>
          <div class="grid h-10 w-10 place-items-center rounded-full bg-brand-soft text-sm font-bold text-brand">{{ userInitials }}</div>
          <button type="button" class="rounded-xl border border-line px-3 py-2.5 text-xs font-semibold text-muted transition hover:border-red-200 hover:bg-red-50 hover:text-red-700" @click="signOut">Sign out</button>
        </div>
      </div>
      <nav class="mx-auto flex max-w-7xl items-center gap-1 overflow-x-auto px-4 pb-3 sm:px-6 lg:px-8" aria-label="Workspace sections">
        <NuxtLink  :to="workspacePath()" class="shrink-0 rounded-lg bg-brand-soft px-3 py-2 text-xs font-semibold text-brand">Overview</NuxtLink>
        <NuxtLink v-if="currentUser?.role === 'learner'"  :to="workspacePath('/learn')" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-muted hover:bg-canvas hover:text-ink">My learning</NuxtLink>
        <NuxtLink v-if="['owner','admin','instructor'].includes(currentUser?.role || '')" :to="workspacePath('/users')" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-muted hover:bg-canvas hover:text-ink">People</NuxtLink>
        <NuxtLink v-if="['owner','admin'].includes(currentUser?.role || '')" :to="workspacePath('/site')" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-muted hover:bg-canvas hover:text-ink">Public site & theme</NuxtLink>
        <NuxtLink v-if="['owner','admin'].includes(currentUser?.role || '')" :to="workspacePath('/settings')" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-muted hover:bg-canvas hover:text-ink">Payments & plans</NuxtLink>
        <NuxtLink v-if="['owner','admin'].includes(currentUser?.role || '') && currentCompany?.slug" :to="'/p/' + currentCompany.slug" target="_blank" class="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-brand hover:bg-brand-soft">View public page ↗</NuxtLink>
      </nav>
    </header>

    <main class="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
      <div class="mb-8 flex flex-col justify-between gap-5 sm:flex-row sm:items-end">
        <div>
          <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Workspace overview</p>
          <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Good things start here<span class="text-brand">.</span></h1>
          <p class="mt-3 text-sm leading-6 text-muted">A clear view of your course library and what is ready for learners.</p>
        </div>
        <button v-if="canManageContent" type="button" class="inline-flex items-center justify-center gap-2 rounded-xl bg-brand px-5 py-3.5 text-sm font-semibold text-white shadow-lg shadow-brand/20 transition hover:-translate-y-0.5 hover:bg-brand-dark" @click="createOpen = !createOpen">
          <svg viewBox="0 0 20 20" fill="none" class="h-4 w-4" aria-hidden="true"><path d="M10 3.5v13M3.5 10h13" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
          {{ createOpen ? 'Close form' : 'Create a course' }}
        </button>
      </div>

      <div v-if="pageError" role="alert" class="mb-5 flex items-start justify-between gap-4 rounded-2xl border border-red-200 bg-red-50 px-4 py-3.5 text-sm text-red-700">
        <span>{{ pageError }}</span>
        <button type="button" class="shrink-0 font-semibold underline underline-offset-2" @click="loadDashboard">Retry</button>
      </div>
      <div v-if="successMessage" role="status" class="mb-5 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3.5 text-sm text-emerald-800">{{ successMessage }}</div>

      <section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4" aria-label="Workspace statistics">
        <article class="rounded-2xl border border-line bg-white p-5 shadow-sm shadow-ink/[0.02]">
          <div class="flex items-center justify-between"><span class="text-sm font-medium text-muted">Total courses</span><span class="grid h-9 w-9 place-items-center rounded-xl bg-brand-soft text-brand"><svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="M4 6.5A2.5 2.5 0 0 1 6.5 4H20v15H6.5A2.5 2.5 0 0 0 4 21V6.5ZM4 18.5A2.5 2.5 0 0 1 6.5 16H20M8 8h7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg></span></div>
          <p class="mt-5 text-3xl font-semibold tracking-tight">{{ loading ? '—' : courses.length }}</p><p class="mt-1 text-xs text-muted">In this workspace</p>
        </article>
        <article class="rounded-2xl border border-line bg-white p-5 shadow-sm shadow-ink/[0.02]">
          <div class="flex items-center justify-between"><span class="text-sm font-medium text-muted">Published</span><span class="grid h-9 w-9 place-items-center rounded-xl bg-emerald-100 text-emerald-700"><svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="m5 12 4 4L19 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg></span></div>
          <p class="mt-5 text-3xl font-semibold tracking-tight">{{ loading ? '—' : publishedCount }}</p><p class="mt-1 text-xs text-muted">Ready for learners</p>
        </article>
        <article class="rounded-2xl border border-line bg-white p-5 shadow-sm shadow-ink/[0.02]">
          <div class="flex items-center justify-between"><span class="text-sm font-medium text-muted">Drafts</span><span class="grid h-9 w-9 place-items-center rounded-xl bg-amber-100 text-amber-700"><svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="m14 3 7 7-11 11H3v-7L14 3Zm-3 3 7 7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg></span></div>
          <p class="mt-5 text-3xl font-semibold tracking-tight">{{ loading ? '—' : draftCount }}</p><p class="mt-1 text-xs text-muted">Still being shaped</p>
        </article>
        <article class="rounded-2xl border border-line bg-ink p-5 text-white shadow-sm shadow-ink/[0.04]">
          <div class="flex items-center justify-between"><span class="text-sm font-medium text-white/70">Workspace</span><span class="grid h-9 w-9 place-items-center rounded-xl bg-white/10 text-emerald-200"><svg viewBox="0 0 24 24" fill="none" class="h-5 w-5" aria-hidden="true"><path d="M12 3 3.5 7.5 12 12l8.5-4.5L12 3ZM5 12l7 3.8 7-3.8M5 16.5l7 3.8 7-3.8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg></span></div>
          <p class="mt-5 truncate text-xl font-semibold tracking-tight">{{ currentCompany?.name || 'Loading…' }}</p><p class="mt-1 text-xs text-white/55">{{ currentCompany?.slug || 'Secure company space' }}</p>
        </article>
      </section>

      <section v-if="canManageContent" class="mt-7 grid gap-4 md:grid-cols-2">
        <NuxtLink :to="workspacePath('/site')" class="group flex items-center gap-4 rounded-2xl border border-line bg-white p-5 transition hover:-translate-y-0.5 hover:shadow-lg"><span class="grid h-12 w-12 shrink-0 place-items-center rounded-2xl bg-brand-soft text-xl text-brand">✦</span><span class="min-w-0 flex-1"><strong class="block text-sm font-semibold">Public company page & theme</strong><span class="mt-1 block text-xs leading-5 text-muted">Set your branding, colours, logo, banner and company story.</span></span><span class="text-lg text-muted transition group-hover:translate-x-1">→</span></NuxtLink>
        <NuxtLink v-if="currentCompany?.slug" :to="'/p/' + currentCompany.slug" target="_blank" class="group flex items-center gap-4 rounded-2xl border border-line bg-white p-5 transition hover:-translate-y-0.5 hover:shadow-lg"><span class="grid h-12 w-12 shrink-0 place-items-center rounded-2xl bg-emerald-100 text-xl text-emerald-700">↗</span><span class="min-w-0 flex-1"><strong class="block text-sm font-semibold">View public course catalogue</strong><span class="mt-1 block text-xs leading-5 text-muted">Open your company’s public learning page to share with learners.</span></span><span class="text-lg text-muted transition group-hover:translate-x-1">→</span></NuxtLink>
      </section>

      <section v-if="createOpen && canManageContent" class="mt-7 rounded-3xl border border-brand/20 bg-white p-5 shadow-xl shadow-brand/5 sm:p-7">
        <div class="mb-6">
          <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Course builder</p>
          <h2 class="mt-2 text-xl font-semibold tracking-tight">{{ editingCourseId ? 'Edit course details' : 'Create a course' }}</h2>
          <p class="mt-1 text-sm text-muted">{{ editingCourseId ? 'Update the title, URL, description, or price. Existing course content stays in place.' : 'New courses start as drafts. Publish them when they are ready.' }}</p>
        </div>
        <form class="grid gap-5 md:grid-cols-2" @submit.prevent="submitCourse">
          <div>
            <label for="courseTitle" class="mb-2 block text-sm font-semibold">Course title</label>
            <input id="courseTitle" v-model="title" required minlength="3" maxlength="140" placeholder="e.g. Product design fundamentals" class="w-full rounded-xl border border-line px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
          </div>
          <div>
            <label for="courseSlug" class="mb-2 block text-sm font-semibold">URL slug</label>
            <input id="courseSlug" v-model="slug" @input="slugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="product-design-fundamentals" class="w-full rounded-xl border border-line px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
          </div>
          <div class="md:col-span-2">
            <label for="courseDescription" class="mb-2 block text-sm font-semibold">Description <span class="font-normal text-muted">(optional)</span></label>
            <textarea id="courseDescription" v-model="description" rows="3" maxlength="10000" placeholder="What will learners take away from this course?" class="w-full resize-y rounded-xl border border-line px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
          </div>
          <div>
            <label for="coursePrice" class="mb-2 block text-sm font-semibold">Price (INR)</label>
            <div class="flex items-center rounded-xl border border-line px-4 focus-within:border-brand focus-within:ring-4 focus-within:ring-brand/10">
              <span class="text-sm text-muted">₹</span>
              <input id="coursePrice" v-model.number="priceRupees" type="number" min="0" max="1000000" step="0.01" required class="w-full border-0 bg-transparent px-2 py-3 text-sm outline-none" />
            </div>
            <p class="mt-1.5 text-xs text-muted">Set to 0 for a free course.</p>
          </div>
          <div class="md:col-span-2"><label for="courseThumbnail" class="mb-2 block text-sm font-semibold">Course thumbnail URL <span class="font-normal text-muted">(optional, HTTPS)</span></label><input id="courseThumbnail" v-model="thumbnailUrl" type="url" maxlength="2048" placeholder="https://cdn.example.com/course-cover.jpg" class="w-full rounded-xl border border-line px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" /><p class="mt-1.5 text-xs text-muted">This cover appears on your public page and course cards. Use a publicly reachable HTTPS image.</p></div>
          <div class="flex items-end gap-3">
            <button type="submit" :disabled="busy" class="inline-flex min-h-11 flex-1 items-center justify-center rounded-xl bg-brand px-4 py-3 text-sm font-semibold text-white transition hover:bg-brand-dark disabled:opacity-60">{{ busy ? 'Saving…' : (editingCourseId ? 'Save changes' : 'Save as draft') }}</button>
            <button type="button" class="min-h-11 rounded-xl border border-line px-4 py-3 text-sm font-semibold text-muted hover:bg-canvas" @click="createOpen = false; editingCourseId = ''">Cancel</button>
          </div>
        </form>
      </section>

      <section class="mt-8">
        <div class="mb-5 flex items-center justify-between gap-3">
          <div><h2 class="text-xl font-semibold tracking-tight">Your courses</h2><p class="mt-1 text-sm text-muted">Manage course visibility from one place.</p></div>
          <span class="rounded-full border border-line bg-white px-3 py-1.5 text-xs font-semibold text-muted">{{ courses.length }} total</span>
        </div>

        <div v-if="loading" class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          <div v-for="item in 3" :key="item" class="h-48 animate-pulse rounded-2xl border border-line bg-white" />
        </div>
        <div v-else-if="courses.length" class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          <article v-for="(course, index) in courses" :key="course.id" class="group flex min-w-0 flex-col overflow-hidden rounded-2xl border border-line bg-white transition hover:-translate-y-0.5 hover:shadow-xl hover:shadow-ink/5">
            <div class="relative flex h-28 items-end overflow-hidden p-5" :class="index % 3 === 0 ? 'bg-gradient-to-br from-[#e6e2ff] to-[#f4f2ff]' : index % 3 === 1 ? 'bg-gradient-to-br from-[#dff8e7] to-[#f0fbf3]' : 'bg-gradient-to-br from-[#ffedda] to-[#fff7ef]'">
              <img v-if="course.thumbnailUrl" :src="course.thumbnailUrl" :alt="course.title + ' thumbnail'" class="absolute inset-0 h-full w-full object-cover" />
              <div v-if="course.thumbnailUrl" class="absolute inset-0 bg-gradient-to-t from-black/45 to-transparent" />
              <div class="absolute -right-4 -top-10 h-32 w-32 rounded-full border-[18px] border-white/35" />
              <div class="absolute right-12 top-5 h-10 w-10 rotate-12 rounded-xl border border-white/70 bg-white/30" />
              <span class="relative rounded-lg bg-white/75 px-2.5 py-1 text-[10px] font-bold uppercase tracking-wider text-ink/65">Course</span>
              <span class="absolute bottom-4 right-4 rounded-full px-2.5 py-1 text-[10px] font-semibold capitalize" :class="course.status === 'published' ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'">{{ course.status }}</span>
            </div>
            <div class="flex flex-1 flex-col p-5">
              <NuxtLink :to="workspacePath('/courses/' + course.id)" class="line-clamp-2 text-base font-semibold leading-6 transition hover:text-brand">{{ course.title }}</NuxtLink>
              <p class="mt-2 line-clamp-2 min-h-10 text-sm leading-5 text-muted">{{ course.description || 'A new learning experience, ready to take shape.' }}</p>
              <div class="mt-5 flex flex-col gap-3 border-t border-line pt-4">
                <div class="flex items-center justify-between gap-3">
                  <span class="text-sm font-semibold">{{ formatPrice(course.priceInPaise, course.currency) }}</span>
                  <span class="rounded-full px-2.5 py-1 text-[10px] font-semibold capitalize" :class="course.status === 'published' ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'">{{ course.status }}</span>
                </div>
                <div class="flex flex-wrap items-center gap-2">
                  <NuxtLink :to="workspacePath('/courses/' + course.id)" class="rounded-lg bg-ink px-3 py-2 text-xs font-semibold text-white transition hover:bg-ink/80">Manage content</NuxtLink>
                  <button v-if="canManageContent" type="button" class="rounded-lg border border-line px-3 py-2 text-xs font-semibold text-muted hover:border-brand hover:text-brand" @click="editCourse(course)">Edit details</button>
                  <button v-if="canManageContent && course.status !== 'published'" type="button" class="rounded-lg bg-brand-soft px-3 py-2 text-xs font-semibold text-brand transition hover:bg-brand hover:text-white" @click="publish(course)">Publish</button>
                  <button v-if="canManageContent" type="button" class="ml-auto rounded-lg border border-red-200 px-3 py-2 text-xs font-semibold text-red-700 transition hover:bg-red-50" @click="archiveCourse(course)">Archive</button>
                </div>
              </div>
            </div>
          </article>
        </div>
        <div v-else class="rounded-3xl border border-dashed border-slate-300 bg-white px-6 py-14 text-center">
          <div class="mx-auto grid h-14 w-14 place-items-center rounded-2xl bg-brand-soft text-brand"><svg viewBox="0 0 24 24" fill="none" class="h-7 w-7" aria-hidden="true"><path d="M4 6.5A2.5 2.5 0 0 1 6.5 4H20v15H6.5A2.5 2.5 0 0 0 4 21V6.5ZM4 18.5A2.5 2.5 0 0 1 6.5 16H20" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg></div>
          <h3 class="mt-4 text-lg font-semibold">Your course library starts here</h3>
          <p class="mx-auto mt-2 max-w-md text-sm leading-6 text-muted">{{ canManageContent ? 'Create your first course draft, add the learning content, then publish it when you are ready.' : 'There are no published courses available in this workspace yet.' }}</p>
          <button v-if="canManageContent" type="button" class="mt-6 rounded-xl bg-brand px-5 py-3 text-sm font-semibold text-white transition hover:bg-brand-dark" @click="createOpen = true">Create your first course</button>
        </div>
      </section>

      <footer class="mt-12 flex flex-col gap-2 border-t border-line py-6 text-xs text-muted sm:flex-row sm:items-center sm:justify-between">
        <p>Newage workspace · Keep learning moving.</p>
        <p>API-backed starter dashboard</p>
      </footer>
    </main>
  </div>
</template>
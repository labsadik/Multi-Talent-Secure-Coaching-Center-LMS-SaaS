<script setup lang="ts">
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ layout: 'auth', middleware: 'company-guest' })

const route = useRoute()
const { login, currentUser, currentCompany, logout } = useAuth()
const companySlug = computed(() => String(route.params.companySlug || '').trim().toLowerCase())
const email = ref('')
const password = ref('')
const pending = ref(false)
const switchingWorkspace = ref(false)
const errorMessage = ref('')

const isDifferentCompany = computed(() => Boolean(
  currentUser.value && currentCompany.value?.slug?.trim().toLowerCase() !== companySlug.value,
))
const courseId = computed(() => typeof route.query.courseId === 'string' ? route.query.courseId.trim() : '')
const registerPath = computed(() => ({
  path: '/workspace/' + encodeURIComponent(companySlug.value) + '/register',
  query: courseId.value ? { courseId: courseId.value } : {},
}))

function loginResponsePath() {
  if (currentUser.value?.role === 'learner') {
    return courseId.value
      ? { path: '/dashboard/learn', query: { checkoutCourse: courseId.value } }
      : '/dashboard/learn'
  }
  const requested = typeof route.query.redirect === 'string' ? route.query.redirect : '/dashboard'
  return requested.startsWith('/') && !requested.startsWith('//') ? requested : '/dashboard'
}

async function switchWorkspace() {
  switchingWorkspace.value = true
  try {
    await logout()
  } catch {
    // logout() clears the local session even if the API is temporarily unavailable.
  } finally {
    switchingWorkspace.value = false
  }
}

async function submitLogin() {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(companySlug.value)) {
    errorMessage.value = 'This company login link is invalid. Ask the company for the correct link.'
    return
  }

  pending.value = true
  errorMessage.value = ''
  try {
    await login({
      companySlug: companySlug.value,
      email: email.value.trim(),
      password: password.value,
    })
    await navigateTo(loginResponsePath())
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'We could not sign you in. Check your email and password for this company.')
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <div>
    <NuxtLink to="/" class="mb-8 hidden text-sm font-medium text-muted transition hover:text-ink lg:inline-flex">← Back to Newage</NuxtLink>
    <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Company workspace</p>
    <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Sign in to your company.</h1>
    <p class="mt-3 text-sm leading-6 text-muted">Use an active account that belongs to this company workspace.</p>

    <div class="mt-5 flex items-center gap-3 rounded-xl border border-line bg-canvas px-4 py-3">
      <span class="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-brand text-sm font-black text-white">N</span>
      <div class="min-w-0">
        <p class="text-xs text-muted">Company workspace</p>
        <p class="break-all text-sm font-semibold">{{ companySlug }}</p>
      </div>
    </div>

    <p v-if="courseId && !isDifferentCompany" class="mt-4 rounded-xl border border-brand/20 bg-brand-soft px-4 py-3 text-sm leading-6 text-ink">Sign in to continue to the course you selected. After sign-in, you'll return to that course to enrol or pay.</p>

    <div v-if="isDifferentCompany" class="mt-6 rounded-2xl border border-amber-200 bg-amber-50 p-5">
      <h2 class="font-semibold text-amber-950">You are signed in to another company</h2>
      <p class="mt-2 text-sm leading-6 text-amber-900">Your current account belongs to {{ currentCompany?.name || currentCompany?.slug || 'another workspace' }}. It cannot be used in {{ companySlug }}. Sign out first, then enter an account that belongs to this company.</p>
      <button type="button" :disabled="switchingWorkspace" class="mt-4 w-full rounded-xl bg-amber-950 px-4 py-3 text-sm font-semibold text-white disabled:opacity-60" @click="switchWorkspace">{{ switchingWorkspace ? 'Signing out…' : 'Sign out and continue' }}</button>
    </div>

    <template v-else>
      <div v-if="errorMessage" role="alert" class="mt-5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div>
      <form class="mt-5 space-y-5" @submit.prevent="submitLogin">
        <div>
          <label for="email" class="mb-2 block text-sm font-semibold">Email address</label>
          <input id="email" v-model="email" name="email" type="email" autocomplete="username" required maxlength="254" placeholder="you@company.com" class="w-full rounded-xl border border-line bg-white px-4 py-3.5 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
        </div>
        <div>
          <div class="mb-2 flex items-center justify-between gap-3"><label for="password" class="block text-sm font-semibold">Password</label><span class="text-xs text-muted">Keep it private</span></div>
          <input id="password" v-model="password" name="password" type="password" autocomplete="current-password" required maxlength="128" placeholder="Enter your password" class="w-full rounded-xl border border-line bg-white px-4 py-3.5 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
        </div>
        <button type="submit" :disabled="pending" class="inline-flex w-full items-center justify-center gap-2 rounded-xl bg-brand px-5 py-3.5 text-sm font-semibold text-white shadow-lg shadow-brand/20 transition hover:bg-brand-dark disabled:cursor-not-allowed disabled:opacity-60">
          {{ pending ? 'Signing you in…' : 'Sign in' }}
        </button>
      </form>

      <p class="mt-7 text-center text-sm text-muted">Buying a course or joining as a learner? <NuxtLink :to="registerPath" class="font-semibold text-brand hover:text-brand-dark">Create your learner account</NuxtLink></p>
      <p class="mt-3 text-center text-sm text-muted">Employee or instructor? Your company administrator must create or activate your account.</p>
      <p class="mt-3 text-center text-xs leading-5 text-muted">The API checks your active membership in this exact company before signing you in.</p>
    </template>
  </div>
</template>

<script setup lang="ts">
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ layout: 'auth', middleware: 'company-guest' })

const route = useRoute()
const { registerLearner, currentUser, currentCompany, logout } = useAuth()
const companySlug = computed(() => String(route.params.companySlug || '').trim().toLowerCase())
const name = ref('')
const email = ref('')
const password = ref('')
const pending = ref(false)
const switchingWorkspace = ref(false)
const errorMessage = ref('')

const isDifferentCompany = computed(() => Boolean(
  currentUser.value && currentCompany.value?.slug?.trim().toLowerCase() !== companySlug.value,
))
const courseId = computed(() => typeof route.query.courseId === 'string' ? route.query.courseId.trim() : '')
const loginPath = computed(() => ({
  path: '/workspace/' + encodeURIComponent(companySlug.value) + '/login',
  query: courseId.value ? { courseId: courseId.value } : {},
}))

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

async function submit() {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(companySlug.value)) {
    errorMessage.value = 'This company registration link is invalid. Ask the company for its learning link.'
    return
  }

  pending.value = true
  errorMessage.value = ''
  try {
    await registerLearner({
      companySlug: companySlug.value,
      name: name.value.trim(),
      email: email.value.trim(),
      password: password.value,
    })
    const dashboardPath = '/workspace/' + encodeURIComponent(companySlug.value) + '/dashboard/learn'
    await navigateTo(courseId.value
      ? { path: dashboardPath, query: { checkoutCourse: courseId.value } }
      : dashboardPath)
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Learner account could not be created. Check your details or contact the company.')
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <div>
    <NuxtLink to="/" class="mb-8 hidden text-sm font-medium text-muted transition hover:text-ink lg:inline-flex">← Back to Newage</NuxtLink>
    <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Company learning hub</p>
    <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Create your learner account.</h1>
    <p class="mt-3 text-sm leading-6 text-muted">Your account will belong to this company workspace only.</p>

    <div class="mt-5 flex items-center gap-3 rounded-xl border border-line bg-canvas px-4 py-3">
      <span class="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-brand text-sm font-black text-white">N</span>
      <div class="min-w-0">
        <p class="text-xs text-muted">Registration for company</p>
        <p class="break-all text-sm font-semibold">{{ companySlug }}</p>
      </div>
    </div>

    <div v-if="isDifferentCompany" class="mt-6 rounded-2xl border border-amber-200 bg-amber-50 p-5">
      <h2 class="font-semibold text-amber-950">You are signed in to another company</h2>
      <p class="mt-2 text-sm leading-6 text-amber-900">Your current account belongs to {{ currentCompany?.name || currentCompany?.slug || 'another workspace' }}. Sign out before creating a learner account for {{ companySlug }}. Your current account will not be added to this company.</p>
      <button type="button" :disabled="switchingWorkspace" class="mt-4 w-full rounded-xl bg-amber-950 px-4 py-3 text-sm font-semibold text-white disabled:opacity-60" @click="switchWorkspace">{{ switchingWorkspace ? 'Signing out…' : 'Sign out and continue' }}</button>
    </div>

    <template v-else>
      <div v-if="errorMessage" role="alert" class="mt-6 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div>
      <form class="mt-7 space-y-4" @submit.prevent="submit">
        <div>
          <label for="name" class="mb-2 block text-sm font-semibold">Your name</label>
          <input id="name" v-model="name" required minlength="2" maxlength="80" autocomplete="name" placeholder="Alex Morgan" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
        </div>
        <div>
          <label for="email" class="mb-2 block text-sm font-semibold">Email address</label>
          <input id="email" v-model="email" required type="email" maxlength="254" autocomplete="email" placeholder="you@example.com" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
        </div>
        <div>
          <label for="password" class="mb-2 block text-sm font-semibold">Password</label>
          <input id="password" v-model="password" required type="password" minlength="12" maxlength="128" autocomplete="new-password" placeholder="At least 12 characters" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
          <p class="mt-1.5 text-xs text-muted">Use 12–128 characters.</p>
        </div>
        <button :disabled="pending" class="w-full rounded-xl bg-brand px-5 py-3.5 text-sm font-semibold text-white hover:bg-brand-dark disabled:opacity-50">{{ pending ? 'Creating account…' : 'Create learner account' }}</button>
      </form>

      <p class="mt-6 text-center text-sm text-muted">Already have an account? <NuxtLink :to="loginPath" class="font-semibold text-brand">Sign in to {{ companySlug }}</NuxtLink></p>
      <p class="mt-5 text-xs leading-5 text-muted">Learner signup is available only when this company has enabled registration. Employee and instructor accounts are managed by the company administrator.</p>
    </template>
  </div>
</template>

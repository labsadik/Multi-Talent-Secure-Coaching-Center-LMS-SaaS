<script setup lang="ts">
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ layout: 'auth', middleware: 'guest' })

const route = useRoute()
const { login } = useAuth()
const companySlug = ref('')
const email = ref('')
const password = ref('')
const pending = ref(false)
const errorMessage = ref('')

function loginResponsePath() {
  if (useAuth().currentUser.value?.role === 'learner') return '/dashboard/learn'
  const requested = typeof route.query.redirect === 'string' ? route.query.redirect : '/dashboard'
  return requested.startsWith('/') && !requested.startsWith('//') ? requested : '/dashboard'
}

onMounted(() => {
  if (typeof route.query.companySlug === 'string') companySlug.value = route.query.companySlug
})

async function submitLogin() {
  pending.value = true
  errorMessage.value = ''

  try {
    await login({
      companySlug: companySlug.value.trim().toLowerCase(),
      email: email.value.trim(),
      password: password.value,
    })

    await navigateTo(loginResponsePath())
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'We could not sign you in. Check your details and try again.')
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <div>
    <NuxtLink to="/" class="mb-8 hidden text-sm font-medium text-muted transition hover:text-ink lg:inline-flex">← Back to Newage</NuxtLink>
    <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Welcome back</p>
    <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Sign in to your space.</h1>
    <p class="mt-3 text-sm leading-6 text-muted">Pick up where you left off in your learning workspace.</p>

    <div v-if="errorMessage" role="alert" class="mt-6 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div>

    <form class="mt-8 space-y-5" @submit.prevent="submitLogin">
      <div>
        <label for="companySlug" class="mb-2 block text-sm font-semibold">Workspace slug</label>
        <input id="companySlug" v-model="companySlug" name="companySlug" autocomplete="organization" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="your-company" class="w-full rounded-xl border border-line bg-white px-4 py-3.5 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
        <p class="mt-2 text-xs text-muted">The short name used when your workspace was created.</p>
      </div>
      <div>
        <label for="email" class="mb-2 block text-sm font-semibold">Email address</label>
        <input id="email" v-model="email" name="email" type="email" autocomplete="username" required maxlength="254" placeholder="you@company.com" class="w-full rounded-xl border border-line bg-white px-4 py-3.5 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
      </div>
      <div>
        <div class="mb-2 flex items-center justify-between gap-3"><label for="password" class="block text-sm font-semibold">Password</label><span class="text-xs text-muted">Keep it private</span></div>
        <input id="password" v-model="password" name="password" type="password" autocomplete="current-password" required maxlength="128" placeholder="Enter your password" class="w-full rounded-xl border border-line bg-white px-4 py-3.5 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
      </div>

      <button type="submit" :disabled="pending" class="inline-flex w-full items-center justify-center gap-2 rounded-xl bg-brand px-5 py-3.5 text-sm font-semibold text-white shadow-lg shadow-brand/20 transition hover:bg-brand-dark disabled:cursor-not-allowed disabled:opacity-60">
        <svg v-if="pending" viewBox="0 0 24 24" fill="none" class="h-4 w-4 animate-spin" aria-hidden="true"><circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity=".25" stroke-width="3" /><path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" /></svg>
        {{ pending ? 'Signing you in…' : 'Sign in' }}
      </button>
    </form>

    <p class="mt-7 text-center text-sm text-muted">New to Newage? <NuxtLink to="/register" class="font-semibold text-brand hover:text-brand-dark">Create a workspace</NuxtLink></p>
    <p class="mt-3 text-center text-sm text-muted">Joining a workspace to learn? <NuxtLink to="/learn/register" class="font-semibold text-brand hover:text-brand-dark">Create a learner account</NuxtLink></p>
    <p class="mt-8 text-center text-xs leading-5 text-muted">Your access is checked by the API on each authenticated request.</p>
  </div>
</template>
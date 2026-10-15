<script setup lang="ts">
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ layout: 'auth' })

const { register } = useAuth()
const companyName = ref('')
const companySlug = ref('')
const slugTouched = ref(false)
const name = ref('')
const email = ref('')
const password = ref('')
const pending = ref(false)
const errorMessage = ref('')

watch(companyName, (value) => {
  if (!slugTouched.value) {
    companySlug.value = slugify(value)
  }
})

function slugify(value: string) {
  return value.trim().toLowerCase().normalize('NFKD').replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 80)
}

async function submitRegister() {
  pending.value = true
  errorMessage.value = ''

  try {
    await register({
      companyName: companyName.value.trim(),
      companySlug: companySlug.value.trim().toLowerCase(),
      name: name.value.trim(),
      email: email.value.trim(),
      password: password.value,
    })
    await navigateTo('/dashboard')
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'We could not create your workspace. Review the details and try again.')
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <div>
    <NuxtLink to="/" class="mb-8 hidden text-sm font-medium text-muted transition hover:text-ink lg:inline-flex">← Back to Newage</NuxtLink>
    <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">A fresh start</p>
    <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Create your workspace.</h1>
    <p class="mt-3 text-sm leading-6 text-muted">Set up your organisation and owner account. You can invite people later.</p>

    <div v-if="errorMessage" role="alert" class="mt-6 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div>

    <form class="mt-7 space-y-4" @submit.prevent="submitRegister">
      <div>
        <label for="companyName" class="mb-2 block text-sm font-semibold">Workspace name</label>
        <input id="companyName" v-model="companyName" name="companyName" autocomplete="organization" required minlength="2" maxlength="120" placeholder="Acme Learning" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
      </div>
      <div>
        <label for="companySlug" class="mb-2 block text-sm font-semibold">Workspace slug</label>
        <input id="companySlug" v-model="companySlug" @input="slugTouched = true" name="companySlug" autocomplete="off" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="acme-learning" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
        <p class="mt-1.5 text-xs text-muted">Lowercase letters, numbers, and hyphens.</p>
      </div>
      <div>
        <label for="name" class="mb-2 block text-sm font-semibold">Your name</label>
        <input id="name" v-model="name" name="name" autocomplete="name" required minlength="2" maxlength="80" placeholder="Alex Morgan" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
      </div>
      <div>
        <label for="email" class="mb-2 block text-sm font-semibold">Work email</label>
        <input id="email" v-model="email" name="email" type="email" autocomplete="email" required maxlength="254" placeholder="you@company.com" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
      </div>
      <div>
        <label for="password" class="mb-2 block text-sm font-semibold">Password</label>
        <input id="password" v-model="password" name="password" type="password" autocomplete="new-password" required minlength="12" maxlength="128" placeholder="At least 12 characters" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none transition placeholder:text-slate-400 focus:border-brand focus:ring-4 focus:ring-brand/10" />
        <p class="mt-1.5 text-xs text-muted">Use 12–128 characters. A password manager is recommended.</p>
      </div>

      <button type="submit" :disabled="pending" class="inline-flex w-full items-center justify-center gap-2 rounded-xl bg-brand px-5 py-3.5 text-sm font-semibold text-white shadow-lg shadow-brand/20 transition hover:bg-brand-dark disabled:cursor-not-allowed disabled:opacity-60">
        <svg v-if="pending" viewBox="0 0 24 24" fill="none" class="h-4 w-4 animate-spin" aria-hidden="true"><circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity=".25" stroke-width="3" /><path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" /></svg>
        {{ pending ? 'Creating workspace…' : 'Create workspace' }}
      </button>
    </form>

    <p class="mt-6 text-center text-sm text-muted">Already have a workspace? <NuxtLink to="/login" class="font-semibold text-brand hover:text-brand-dark">Sign in</NuxtLink></p>
    <p class="mt-5 text-center text-xs leading-5 text-muted">Creating a workspace makes you its owner. Registration requires the backend to be running.</p>
  </div>
</template>
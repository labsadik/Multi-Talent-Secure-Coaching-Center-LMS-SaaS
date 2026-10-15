<script setup lang="ts">
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ layout: 'auth', middleware: 'guest' })

const { registerLearner } = useAuth()
const route = useRoute()
const companySlug = ref('')
const name = ref('')
const email = ref('')
const password = ref('')
const pending = ref(false)
const errorMessage = ref('')

onMounted(() => {
  if (typeof route.query.companySlug === 'string') companySlug.value = route.query.companySlug
})

async function submit() {
  pending.value = true
  errorMessage.value = ''
  try {
    await registerLearner({
      companySlug: companySlug.value.trim().toLowerCase(),
      name: name.value.trim(),
      email: email.value.trim(),
      password: password.value,
    })
    const courseId = typeof route.query.courseId === 'string' ? route.query.courseId : ''
    await navigateTo(courseId ? { path: '/dashboard/learn', query: { checkoutCourse: courseId } } : '/dashboard/learn')
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Learner account could not be created. Check the workspace slug and details.')
  } finally { pending.value = false }
}
</script>

<template>
  <div>
    <NuxtLink to="/" class="mb-8 hidden text-sm font-medium text-muted transition hover:text-ink lg:inline-flex">← Back to Newage</NuxtLink>
    <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Start learning</p>
    <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Create your learner account.</h1>
    <p class="mt-3 text-sm leading-6 text-muted">Join a workspace and find the courses your team has published.</p>
    <div v-if="errorMessage" role="alert" class="mt-6 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div>
    <form class="mt-7 space-y-4" @submit.prevent="submit">
      <div><label for="companySlug" class="mb-2 block text-sm font-semibold">Workspace slug</label><input id="companySlug" v-model="companySlug" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" autocomplete="organization" placeholder="your-company" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
      <div><label for="name" class="mb-2 block text-sm font-semibold">Your name</label><input id="name" v-model="name" required minlength="2" maxlength="80" autocomplete="name" placeholder="Alex Morgan" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
      <div><label for="email" class="mb-2 block text-sm font-semibold">Email address</label><input id="email" v-model="email" required type="email" maxlength="254" autocomplete="email" placeholder="you@example.com" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
      <div><label for="password" class="mb-2 block text-sm font-semibold">Password</label><input id="password" v-model="password" required type="password" minlength="12" maxlength="128" autocomplete="new-password" placeholder="At least 12 characters" class="w-full rounded-xl border border-line bg-white px-4 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /><p class="mt-1.5 text-xs text-muted">Use 12–128 characters.</p></div>
      <button :disabled="pending" class="w-full rounded-xl bg-brand px-5 py-3.5 text-sm font-semibold text-white hover:bg-brand-dark disabled:opacity-50">{{ pending ? 'Creating account…' : 'Create learner account' }}</button>
    </form>
    <p class="mt-6 text-center text-sm text-muted">Already have an account? <NuxtLink to="/login" class="font-semibold text-brand">Sign in</NuxtLink></p>
    <p class="mt-5 text-xs leading-5 text-muted">Learner registration must be enabled for the selected workspace.</p>
  </div>
</template>
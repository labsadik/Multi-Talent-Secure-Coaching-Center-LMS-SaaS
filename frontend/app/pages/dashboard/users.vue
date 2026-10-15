<script setup lang="ts">
import type { CompanyUser } from '~/types/api'
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: ['auth', 'company-dashboard'] })

const route = useRoute()
function workspacePath(suffix = '') {
  const slug = String(route.params.companySlug || currentCompany.value?.slug || '').trim()
  return slug ? '/workspace/' + encodeURIComponent(slug) + '/dashboard' + suffix : '/dashboard' + suffix
}

const { currentUser, currentCompany, getCompanyUsers, createCompanyUser, updateCompanyUser } = useAuth()
const { confirm } = useConfirmDialog()
const users = ref<CompanyUser[]>([])
const loading = ref(true)
const saving = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const formOpen = ref(false)
const name = ref('')
const email = ref('')
const password = ref('')
const role = ref<'instructor' | 'learner' | 'admin'>('instructor')
const isInstructor = computed(() => currentUser.value?.role === 'instructor')
const canManageUsers = computed(() => ['owner', 'admin', 'instructor'].includes(currentUser.value?.role || ''))
const employeeLoginPath = computed(() => currentCompany.value?.slug
  ? '/workspace/' + encodeURIComponent(currentCompany.value.slug) + '/login'
  : '')

async function copyEmployeeLoginLink() {
  if (!import.meta.client || !employeeLoginPath.value) return
  try {
    await navigator.clipboard.writeText(window.location.origin + employeeLoginPath.value)
    successMessage.value = 'Company login link copied. Share it with your employees.'
    errorMessage.value = ''
  } catch {
    errorMessage.value = 'Clipboard access was blocked. Open the company login panel and copy its URL from the browser address bar.'
  }
}

async function load() {
  loading.value = true
  errorMessage.value = ''
  try { users.value = await getCompanyUsers() }
  catch (error) { errorMessage.value = apiErrorMessage(error, 'Users could not be loaded. Owners, admins, and instructors can access this page.') }
  finally { loading.value = false }
}
async function addUser() {
  saving.value = true
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const createdRole = isInstructor.value ? 'learner' : role.value
    await createCompanyUser({ name: name.value.trim(), email: email.value.trim(), password: password.value, role: createdRole })
    name.value = ''
    email.value = ''
    password.value = ''
    role.value = isInstructor.value ? 'learner' : 'instructor'
    formOpen.value = false
    successMessage.value = isInstructor.value ? 'Learner added to this workspace.' : 'User added to this workspace.'
    await load()
  } catch (error) { errorMessage.value = apiErrorMessage(error, 'User could not be created.') }
  finally { saving.value = false }
}
async function changeRole(user: CompanyUser, event: Event) {
  const roleSelect = event.target as HTMLSelectElement
  const nextRole = roleSelect.value
  if (nextRole === user.role) return
  const accepted = await confirm({
    title: 'Change member role?',
    message: 'Change ' + user.name + ' from ' + user.role + ' to ' + nextRole + '? Their workspace permissions will change immediately.',
    confirmLabel: 'Update role',
    tone: 'primary',
  })
  if (!accepted) {
    roleSelect.value = user.role
    return
  }
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const updated = await updateCompanyUser(user.id, { role: nextRole })
    users.value = users.value.map((item) => item.id === user.id ? { ...item, ...updated } : item)
    successMessage.value = 'Role updated.'
  } catch (error) { errorMessage.value = apiErrorMessage(error, 'Role could not be updated.') }
}
async function toggleStatus(user: CompanyUser) {
  const disabling = user.status === 'active'
  const accepted = await confirm({
    title: disabling ? 'Disable this account?' : 'Reactivate this account?',
    message: disabling
      ? 'Disable ' + user.name + '? They will lose access to this workspace immediately.'
      : 'Restore workspace access for ' + user.name + '?',
    confirmLabel: disabling ? 'Disable account' : 'Reactivate account',
    tone: disabling ? 'danger' : 'primary',
  })
  if (!accepted) return
  errorMessage.value = ''
  successMessage.value = ''
  try {
    const updated = await updateCompanyUser(user.id, { status: user.status === 'active' ? 'disabled' : 'active' })
    users.value = users.value.map((item) => item.id === user.id ? { ...item, ...updated } : item)
    successMessage.value = updated.status === 'active' ? 'User reactivated.' : 'User disabled. Their existing access token will no longer authorize API requests.'
  } catch (error) { errorMessage.value = apiErrorMessage(error, 'User status could not be changed.') }
}
onMounted(() => { void load() })
</script>

<template>
  <div class="min-h-screen bg-canvas">
    <header class="border-b border-line bg-white"><div class="mx-auto flex h-[72px] max-w-7xl items-center justify-between px-4 sm:px-6"><NuxtLink :to="workspacePath()" class="inline-flex items-center gap-3 font-semibold"><span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white">N</span>newage<span class="text-brand">.</span></NuxtLink><NuxtLink :to="workspacePath()" class="rounded-xl border border-line px-4 py-2.5 text-sm font-semibold text-muted hover:bg-canvas">Workspace</NuxtLink></div></header>
    <main class="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-10">
      <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">{{ isInstructor ? 'Learner management' : 'People & access' }}</p><div class="mt-3 flex flex-col justify-between gap-4 sm:flex-row sm:items-end"><div><h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">{{ isInstructor ? 'Manage your learners.' : 'Your workspace team.' }}</h1><p class="mt-3 max-w-2xl text-sm leading-6 text-muted">{{ isInstructor ? 'Create learner accounts and enable or disable learner access. Staff roles and workspace settings are reserved for admins and the owner.' : 'Manage team roles and disable access when someone no longer needs it. All changes are enforced by the Rust API.' }}</p></div><button v-if="canManageUsers" type="button" class="rounded-xl bg-brand px-5 py-3.5 text-sm font-semibold text-white hover:bg-brand-dark" @click="formOpen = true">{{ isInstructor ? 'Add a learner' : 'Add a teammate' }}</button></div>
      <section v-if="employeeLoginPath && ['owner', 'admin'].includes(currentUser?.role || '')" class="mt-6 flex flex-col justify-between gap-4 rounded-2xl border border-line bg-white p-5 sm:flex-row sm:items-center">
        <div class="min-w-0">
          <p class="text-sm font-semibold">Employee sign-in panel</p>
          <p class="mt-1 text-xs leading-5 text-muted">Share this company-specific link with your students and employees. They can sign in with their company account.</p>
          <code class="mt-2 block break-all text-xs text-brand">{{ employeeLoginPath }}</code>
        </div>
        <div class="flex shrink-0 flex-wrap gap-2">
          <NuxtLink :to="employeeLoginPath" target="_blank" class="rounded-xl border border-line px-4 py-2.5 text-sm font-semibold text-ink hover:bg-canvas">Open panel ↗</NuxtLink>
          <button type="button" class="rounded-xl bg-brand px-4 py-2.5 text-sm font-semibold text-white hover:bg-brand-dark" @click="copyEmployeeLoginLink">Copy login link</button>
        </div>
      </section>
      <div v-if="errorMessage" role="alert" class="mt-5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{{ errorMessage }}</div><div v-if="successMessage" role="status" class="mt-5 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">{{ successMessage }}</div>
      <UiModal
        :open="formOpen && canManageUsers"
        :title="isInstructor ? 'Add a learner' : 'Add a teammate'"
        description="Create a workspace account and set a temporary password to share securely."
        size="lg"
        @update:open="value => { if (!value) formOpen = false }"
      >
        <form class="grid gap-4 sm:grid-cols-2" @submit.prevent="addUser"><div><label for="name" class="mb-2 block text-sm font-semibold">Full name</label><input id="name" v-model="name" required minlength="2" maxlength="80" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /></div><div><label for="email" class="mb-2 block text-sm font-semibold">Email</label><input id="email" v-model="email" required type="email" maxlength="254" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /></div><div><label for="password" class="mb-2 block text-sm font-semibold">Temporary password</label><input id="password" v-model="password" required type="password" minlength="12" maxlength="128" autocomplete="new-password" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /><p class="mt-1.5 text-xs text-muted">At least 12 characters.</p></div><div><label for="role" class="mb-2 block text-sm font-semibold">Role</label><select v-if="!isInstructor" id="role" v-model="role" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand"><option value="instructor">Instructor</option><option value="learner">Learner</option><option value="admin">Admin</option></select><p v-else class="rounded-xl border border-line bg-canvas px-3.5 py-3 text-sm text-muted">Learner</p></div><div class="sm:col-span-2"><button :disabled="saving" class="rounded-xl bg-brand px-5 py-3 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Creating user…' : (isInstructor ? 'Create learner' : 'Create teammate') }}</button></div></form>
      </UiModal>
      <section class="mt-8 overflow-hidden rounded-2xl border border-line bg-white">
        <div class="flex items-center justify-between border-b border-line px-5 py-4"><div><h2 class="font-semibold">{{ isInstructor ? 'Learners' : 'Workspace members' }}</h2><p class="mt-1 text-xs text-muted">{{ isInstructor ? 'Only learner accounts are shown here.' : 'Password hashes are never returned by the API.' }}</p></div><span class="rounded-full bg-canvas px-3 py-1.5 text-xs font-semibold text-muted">{{ users.length }} {{ isInstructor ? 'learners' : 'members' }}</span></div>
        <div v-if="loading" class="space-y-3 p-5"><div v-for="n in 4" :key="n" class="h-12 animate-pulse rounded-xl bg-canvas" /></div>
        <div v-else-if="!users.length" class="p-10 text-center text-sm text-muted">{{ isInstructor ? 'No learners found yet. Add a learner to get started.' : 'No users were returned for this workspace.' }}</div>
        <div v-else class="overflow-x-auto"><table class="w-full min-w-[760px] text-left text-sm"><thead class="bg-canvas text-xs font-semibold uppercase tracking-wider text-muted"><tr><th class="px-5 py-3">Member</th><th class="px-5 py-3">Role</th><th class="px-5 py-3">Status</th><th class="px-5 py-3">Created</th><th class="px-5 py-3 text-right">Access</th></tr></thead><tbody><tr v-for="user in users" :key="user.id" class="border-t border-line"><td class="px-5 py-4"><p class="font-semibold">{{ user.name }}<span v-if="user.id === currentUser?.id" class="ml-2 rounded bg-brand-soft px-1.5 py-0.5 text-[10px] font-semibold text-brand">You</span></p><p class="mt-1 text-xs text-muted">{{ user.email }}</p></td><td class="px-5 py-4"><select v-if="user.role !== 'owner' && user.id !== currentUser?.id && ['owner','admin'].includes(currentUser?.role || '')" :value="user.role" class="rounded-lg border border-line bg-white px-2.5 py-2 text-xs capitalize outline-none focus:border-brand" @change="changeRole(user, $event)"><option value="admin">Admin</option><option value="instructor">Instructor</option><option value="learner">Learner</option></select><span v-else class="capitalize text-muted">{{ user.role }}</span></td><td class="px-5 py-4"><span class="rounded-full px-2.5 py-1 text-[10px] font-semibold capitalize" :class="user.status === 'active' ? 'bg-emerald-100 text-emerald-800' : 'bg-red-100 text-red-800'">{{ user.status }}</span></td><td class="px-5 py-4 text-xs text-muted">{{ user.createdAtMs ? new Date(user.createdAtMs).toLocaleDateString() : '—' }}</td><td class="px-5 py-4 text-right"><button v-if="user.role !== 'owner' && user.id !== currentUser?.id && ['owner','admin','instructor'].includes(currentUser?.role || '')" type="button" class="rounded-lg border px-3 py-2 text-xs font-semibold" :class="user.status === 'active' ? 'border-red-200 text-red-700 hover:bg-red-50' : 'border-emerald-200 text-emerald-700 hover:bg-emerald-50'" @click="toggleStatus(user)">{{ user.status === 'active' ? 'Disable' : 'Reactivate' }}</button><span v-else class="text-xs text-muted">Protected</span></td></tr></tbody></table></div>
      </section>
      <p class="mt-4 text-xs leading-5 text-muted">{{ isInstructor ? 'Instructors can create or disable learner accounts but cannot change staff roles or workspace settings.' : 'The owner account is protected from role and status changes. Disabled users are revalidated by the backend on every authenticated request.' }}</p>
    </main>
  </div>
</template>
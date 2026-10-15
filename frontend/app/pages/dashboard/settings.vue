<script setup lang="ts">
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: ['auth', 'company-dashboard'] })

const route = useRoute()
const { currentCompany, currentUser, getRazorpaySettings, configureRazorpay } = useAuth()
function workspacePath(suffix = '') {
  const slug = String(route.params.companySlug || currentCompany.value?.slug || '').trim()
  return slug ? '/workspace/' + encodeURIComponent(slug) + '/dashboard' + suffix : '/dashboard' + suffix
}

const loading = ref(true)
const saving = ref(false)
const configured = ref(false)
const existingKeyId = ref('')
const keyId = ref('')
const keySecret = ref('')
const webhookSecret = ref('')
const message = ref('')
const errorMessage = ref('')
const canConfigure = computed(() => ['owner', 'admin'].includes(currentUser.value?.role || ''))

async function loadSettings() {
  loading.value = true
  errorMessage.value = ''
  try {
    const result = await getRazorpaySettings()
    configured.value = result.configured
    existingKeyId.value = result.keyId || ''
    keyId.value = result.keyId || ''
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Course payment settings could not be loaded.')
  } finally {
    loading.value = false
  }
}

async function saveSettings() {
  saving.value = true
  errorMessage.value = ''
  message.value = ''
  try {
    const result = await configureRazorpay({
      keyId: keyId.value.trim(),
      keySecret: keySecret.value,
      webhookSecret: webhookSecret.value,
    })
    configured.value = result.configured
    existingKeyId.value = result.keyId || ''
    keySecret.value = ''
    webhookSecret.value = ''
    message.value = 'Company Razorpay credentials saved securely. Paid course checkout can now use this company account.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Could not save Razorpay credentials. Check the key format and encryption configuration.')
  } finally {
    saving.value = false
  }
}

onMounted(() => { void loadSettings() })
</script>

<template>
  <div class="min-h-screen bg-canvas">
    <header class="border-b border-line bg-white">
      <div class="mx-auto flex h-[72px] max-w-7xl items-center justify-between px-4 sm:px-6">
        <NuxtLink :to="workspacePath()" class="inline-flex items-center gap-3 font-semibold">
          <span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white">N</span>
          newage<span class="text-brand">.</span>
        </NuxtLink>
        <NuxtLink :to="workspacePath()" class="rounded-xl border border-line px-4 py-2.5 text-sm font-semibold text-muted hover:bg-canvas">Workspace</NuxtLink>
      </div>
    </header>
    <main class="mx-auto max-w-3xl px-4 py-8 sm:px-6 sm:py-10">
      <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Company payment settings</p>
      <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Sell your courses.</h1>
      <p class="mt-3 max-w-2xl text-sm leading-6 text-muted">Connect this company’s own Razorpay account. Learners pay for your published courses; payments are verified before course access is granted. Newage subscription plans are not used for learner checkout.</p>

      <div v-if="errorMessage" role="alert" class="mt-5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div>
      <div v-if="message" role="status" class="mt-5 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm leading-6 text-emerald-800">{{ message }}</div>

      <section class="mt-8 rounded-2xl border border-line bg-white p-5 sm:p-7">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h2 class="text-lg font-semibold">Company Razorpay account</h2>
            <p class="mt-1 text-sm leading-6 text-muted">Every company has isolated credentials. These credentials are used only for courses belonging to this workspace.</p>
          </div>
          <span class="shrink-0 rounded-full px-2.5 py-1 text-[10px] font-semibold" :class="configured ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'">{{ configured ? 'Configured' : 'Not configured' }}</span>
        </div>

        <div v-if="loading" class="mt-6 h-36 animate-pulse rounded-xl bg-canvas" />
        <form v-else-if="canConfigure" class="mt-6 space-y-4" @submit.prevent="saveSettings">
          <div>
            <label for="keyId" class="mb-2 block text-sm font-semibold">Razorpay Key ID</label>
            <input id="keyId" v-model="keyId" required minlength="12" maxlength="100" placeholder="rzp_test_…" autocomplete="off" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
            <p v-if="existingKeyId" class="mt-1.5 text-xs text-muted">Currently saved: {{ existingKeyId }}</p>
          </div>
          <div>
            <label for="keySecret" class="mb-2 block text-sm font-semibold">API Key Secret</label>
            <input id="keySecret" v-model="keySecret" required minlength="8" maxlength="256" type="password" autocomplete="new-password" placeholder="Enter the company API secret" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
          </div>
          <div>
            <label for="webhookSecret" class="mb-2 block text-sm font-semibold">Webhook secret</label>
            <input id="webhookSecret" v-model="webhookSecret" required minlength="8" maxlength="256" type="password" autocomplete="new-password" placeholder="Enter the webhook secret for this company" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
          </div>
          <button :disabled="saving" class="rounded-xl bg-brand px-5 py-3 text-sm font-semibold text-white hover:bg-brand-dark disabled:opacity-50">{{ saving ? 'Encrypting and saving…' : 'Save course payment credentials' }}</button>
        </form>
        <p v-else class="mt-6 rounded-xl bg-canvas p-4 text-sm text-muted">Only workspace owners or admins can change payment configuration.</p>

        <div class="mt-6 rounded-xl bg-canvas p-4">
          <h3 class="text-sm font-semibold">Razorpay webhook setup</h3>
          <p class="mt-1.5 text-sm leading-6 text-muted">In this company’s Razorpay Dashboard, add a webhook that points to your deployed API’s <code class="rounded bg-white px-1.5 py-0.5 text-xs">/api/v1/webhooks/razorpay</code> endpoint and subscribe to <code class="rounded bg-white px-1.5 py-0.5 text-xs">payment.captured</code>. Use the same webhook secret entered above. Keep test keys for sandbox testing and use live keys only after verifying the full purchase flow.</p>
        </div>
        <p class="mt-5 text-xs leading-5 text-muted">API and webhook secrets are encrypted by the backend and never shown again. Course prices are calculated on the server; access is granted only after Razorpay payment verification succeeds.</p>
      </section>
    </main>
  </div>
</template>

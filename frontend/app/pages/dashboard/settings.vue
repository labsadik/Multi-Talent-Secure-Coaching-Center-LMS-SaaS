<script setup lang="ts">
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: 'auth' })

type RazorpaySettings = { configured: boolean; keyId?: string | null }
type OrderResponse = {
  subscriptionOrderId: string
  razorpayOrderId: string
  planCode: string
  amount: number
  currency: string
  keyId: string
  status: string
}
declare global {
  interface Window {
    Razorpay?: new (options: Record<string, unknown>) => { open: () => void }
  }
}

const { currentUser, getRazorpaySettings, configureRazorpay, createSubscriptionOrder } = useAuth()
const loading = ref(true)
const saving = ref(false)
const buyingPlan = ref('')
const configured = ref(false)
const existingKeyId = ref('')
const keyId = ref('')
const keySecret = ref('')
const webhookSecret = ref('')
const message = ref('')
const errorMessage = ref('')

const canConfigure = computed(() => ['owner', 'admin'].includes(currentUser.value?.role || ''))
const plans = [
  { code: 'starter', name: 'Starter', description: 'A structured home for a growing course library', features: ['More active courses', 'More learner seats', 'Core course workflows'] },
  { code: 'pro', name: 'Pro', description: 'Room for multiple programmes and a larger audience', features: ['Higher course limits', 'More active learners', 'All starter capabilities'] },
  { code: 'business', name: 'Business', description: 'More space for teams and a larger learning operation', features: ['Largest configured limits', 'High learner capacity', 'All pro capabilities'] },
]

async function loadSettings() {
  loading.value = true
  errorMessage.value = ''
  try {
    const result = await getRazorpaySettings()
    configured.value = result.configured
    existingKeyId.value = result.keyId || ''
    keyId.value = result.keyId || ''
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Payment settings could not be loaded.')
  } finally { loading.value = false }
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
    message.value = 'Payment credentials saved. Secret values are encrypted by the backend and are not shown again.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Could not save payment credentials. Check the key format and encryption configuration.')
  } finally { saving.value = false }
}
async function ensureCheckoutScript() {
  if (window.Razorpay) return
  await new Promise<void>((resolve, reject) => {
    const script = document.createElement('script')
    script.src = 'https://checkout.razorpay.com/v1/checkout.js'
    script.async = true
    script.onload = () => resolve()
    script.onerror = () => reject(new Error('Razorpay Checkout could not be loaded.'))
    document.head.appendChild(script)
  })
}
async function buyPlan(planCode: 'starter' | 'pro' | 'business') {
  errorMessage.value = ''
  message.value = ''
  buyingPlan.value = planCode
  try {
    const order = await createSubscriptionOrder(planCode) as unknown as OrderResponse
    await ensureCheckoutScript()
    if (!window.Razorpay || !order.keyId || !order.razorpayOrderId) throw new Error('Platform Razorpay checkout is not configured.')
    const amountDisplay = new Intl.NumberFormat('en-IN', { style: 'currency', currency: order.currency || 'INR' }).format(order.amount / 100)
    const checkout = new window.Razorpay({
      key: order.keyId,
      amount: order.amount,
      currency: order.currency || 'INR',
      name: 'Newage',
      description: planCode.charAt(0).toUpperCase() + planCode.slice(1) + ' monthly plan · ' + amountDisplay,
      order_id: order.razorpayOrderId,
      prefill: { name: currentUser.value?.name, email: currentUser.value?.email },
      theme: { color: '#635bdf' },
      handler: () => {
        message.value = 'Checkout completed. The platform webhook is the source of truth for activating your plan; confirmation appears after Razorpay delivers a valid captured-payment event.'
      },
    })
    checkout.open()
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Could not create a subscription order. Check that platform Razorpay credentials are configured on the backend.')
  } finally { buyingPlan.value = '' }
}
onMounted(() => { void loadSettings() })
</script>

<template>
  <div class="min-h-screen bg-canvas">
    <header class="border-b border-line bg-white"><div class="mx-auto flex h-[72px] max-w-7xl items-center justify-between px-4 sm:px-6"><NuxtLink to="/dashboard" class="inline-flex items-center gap-3 font-semibold"><span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white">N</span>newage<span class="text-brand">.</span></NuxtLink><NuxtLink to="/dashboard" class="rounded-xl border border-line px-4 py-2.5 text-sm font-semibold text-muted hover:bg-canvas">Workspace</NuxtLink></div></header>
    <main class="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-10">
      <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Workspace settings</p><h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Payments and plans.</h1><p class="mt-3 max-w-2xl text-sm leading-6 text-muted">Configure course checkout for your workspace and choose a Newage SaaS plan. Payment secrets are sent only to the Rust API.</p>
      <div v-if="errorMessage" role="alert" class="mt-5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div><div v-if="message" role="status" class="mt-5 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm leading-6 text-emerald-800">{{ message }}</div>

      <section class="mt-8 grid gap-6 lg:grid-cols-[1fr_0.85fr]">
        <article class="rounded-2xl border border-line bg-white p-5 sm:p-7">
          <div class="flex items-start justify-between gap-4"><div><h2 class="text-lg font-semibold">Course payment gateway</h2><p class="mt-1 text-sm leading-6 text-muted">These credentials are used for payments inside your courses, separate from Newage platform billing.</p></div><span class="rounded-full px-2.5 py-1 text-[10px] font-semibold" :class="configured ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'">{{ configured ? 'Configured' : 'Not configured' }}</span></div>
          <div v-if="loading" class="mt-6 h-36 animate-pulse rounded-xl bg-canvas" />
          <form v-else-if="canConfigure" class="mt-6 space-y-4" @submit.prevent="saveSettings">
            <div><label for="keyId" class="mb-2 block text-sm font-semibold">Razorpay Key ID</label><input id="keyId" v-model="keyId" required minlength="12" maxlength="100" placeholder="rzp_test_…" autocomplete="off" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /><p v-if="existingKeyId" class="mt-1.5 text-xs text-muted">Currently saved: {{ existingKeyId }}</p></div>
            <div><label for="keySecret" class="mb-2 block text-sm font-semibold">API Key Secret</label><input id="keySecret" v-model="keySecret" required minlength="8" maxlength="256" type="password" autocomplete="new-password" placeholder="Enter the API secret" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
            <div><label for="webhookSecret" class="mb-2 block text-sm font-semibold">Webhook secret</label><input id="webhookSecret" v-model="webhookSecret" required minlength="8" maxlength="256" type="password" autocomplete="new-password" placeholder="Enter the course payment webhook secret" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
            <button :disabled="saving" class="rounded-xl bg-brand px-5 py-3 text-sm font-semibold text-white hover:bg-brand-dark disabled:opacity-50">{{ saving ? 'Encrypting and saving…' : 'Save credentials' }}</button>
          </form>
          <p v-else class="mt-6 rounded-xl bg-canvas p-4 text-sm text-muted">Only workspace owners or admins can change payment configuration.</p>
          <p class="mt-5 text-xs leading-5 text-muted">Use Razorpay test keys in development. Store the matching webhook URL and secret in the Razorpay dashboard. Secret fields are never read back into the browser.</p>
        </article>

        <article class="rounded-2xl border border-line bg-ink p-5 text-white sm:p-7">
          <p class="text-xs font-bold uppercase tracking-[0.2em] text-emerald-200">Platform subscription</p><h2 class="mt-3 text-xl font-semibold">Scale your learning workspace.</h2><p class="mt-2 text-sm leading-6 text-white/60">Choose a plan. The backend determines the final price and activates it only after a verified captured-payment webhook.</p>
          <div class="mt-6 space-y-3">
            <article v-for="plan in plans" :key="plan.code" class="rounded-xl border border-white/10 bg-white/[0.06] p-4">
              <div class="flex items-start justify-between gap-3"><div><h3 class="font-semibold">{{ plan.name }}</h3><p class="mt-1 text-xs leading-5 text-white/55">{{ plan.description }}</p></div><span class="rounded-full bg-white/10 px-2.5 py-1 text-[10px] capitalize">{{ plan.code }}</span></div>
              <p class="mt-3 text-xs leading-6 text-white/70">{{ plan.features.join(' · ') }}</p>
              <button type="button" :disabled="!canConfigure || buyingPlan !== ''" class="mt-4 w-full rounded-xl bg-white px-4 py-2.5 text-sm font-semibold text-ink hover:bg-emerald-100 disabled:opacity-50" @click="buyPlan(plan.code as 'starter' | 'pro' | 'business')">{{ buyingPlan === plan.code ? 'Opening checkout…' : 'Choose ' + plan.name }}</button>
            </article>
          </div>
          <p class="mt-5 text-xs leading-5 text-white/45">Requires platform RAZORPAY_KEY_ID, RAZORPAY_KEY_SECRET and RAZORPAY_WEBHOOK_SECRET in the backend environment.</p>
        </article>
      </section>
    </main>
  </div>
</template>
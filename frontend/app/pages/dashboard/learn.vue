<script setup lang="ts">
import type { Course, LearnerCourse } from '~/types/api'
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: ['auth', 'company-dashboard'], alias: ['/workspace/:companySlug/dashboard/learn'] })

type OrderResponse = {
  enrolled?: boolean
  paymentRequired?: boolean
  orderId?: string
  razorpayOrderId?: string
  amount?: number
  currency?: string
  keyId?: string
}
type CheckoutReply = { razorpay_payment_id: string; razorpay_order_id: string; razorpay_signature: string }
declare global {
  interface Window {
    Razorpay?: new (options: Record<string, unknown>) => { open: () => void }
  }
}

const { currentUser, currentCompany, getCourses, getMyCourses, getCourseProgress, createCourseOrder, verifyCoursePayment } = useAuth()
const route = useRoute()
function workspacePath(suffix = '') {
  const slug = String(route.params.companySlug || currentCompany.value?.slug || '').trim()
  return slug ? '/workspace/' + encodeURIComponent(slug) + '/dashboard' + suffix : '/dashboard' + suffix
}
const courses = ref<Course[]>([])
const enrolled = ref<LearnerCourse[]>([])
const progressByCourse = ref<Record<string, number>>({})
const loading = ref(true)
const busyCourse = ref('')
const errorMessage = ref('')
const successMessage = ref('')
const checkoutBusy = ref(false)
const requestedCheckoutCourse = ref<Course | null>(null)

const isLearner = computed(() => currentUser.value?.role === 'learner')
const enrolledIds = computed(() => new Set(enrolled.value.map((item) => item.courseId)))
const availableCourses = computed(() => courses.value.filter((course) => !enrolledIds.value.has(course.id)))

function formatPrice(paise: number) {
  if (!paise) return 'Free'
  return new Intl.NumberFormat('en-IN', { style: 'currency', currency: 'INR', maximumFractionDigits: 2 }).format(paise / 100)
}
async function load() {
  loading.value = true
  errorMessage.value = ''
  try {
    courses.value = await getCourses()
    if (isLearner.value) {
      enrolled.value = await getMyCourses()
      const entries = await Promise.all(enrolled.value.map(async (item) => {
        try {
          const result = await getCourseProgress(item.courseId)
          return [item.courseId, Number(result.progressPercent || 0)] as const
        } catch { return [item.courseId, 0] as const }
      }))
      progressByCourse.value = Object.fromEntries(entries)
    }
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Learning courses could not be loaded.')
  } finally { loading.value = false }
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
async function enroll(course: Course) {
  errorMessage.value = ''
  successMessage.value = ''
  busyCourse.value = course.id
  try {
    const order = await createCourseOrder(course.id) as OrderResponse
    if (order.enrolled || !order.paymentRequired) {
      successMessage.value = 'You are enrolled. The course is now in your learning list.'
      await load()
      return
    }
    await ensureCheckoutScript()
    if (!window.Razorpay || !order.keyId || !order.razorpayOrderId) throw new Error('The payment checkout is not fully configured.')
    const checkout = new window.Razorpay({
      key: order.keyId,
      amount: order.amount,
      currency: order.currency || 'INR',
      name: 'Newage',
      description: course.title,
      order_id: order.razorpayOrderId,
      prefill: { name: currentUser.value?.name, email: currentUser.value?.email },
      theme: { color: '#635bdf' },
      handler: async (reply: CheckoutReply) => {
        checkoutBusy.value = true
        try {
          await verifyCoursePayment({
            razorpayOrderId: reply.razorpay_order_id,
            razorpayPaymentId: reply.razorpay_payment_id,
            razorpaySignature: reply.razorpay_signature,
          })
          successMessage.value = 'Payment verified and course access granted.'
          await load()
        } catch (error) {
          errorMessage.value = apiErrorMessage(error, 'Payment was submitted. Course access is granted only after the API confirms captured payment; it may take a moment.')
          await load()
        } finally {
          checkoutBusy.value = false
        }
      },
      modal: { ondismiss: () => { busyCourse.value = '' } },
    })
    checkout.open()
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Enrolment could not be started. Paid courses require the company payment settings to be configured.')
  } finally {
    busyCourse.value = ''
  }
}

onMounted(async () => {
  if (!isLearner.value) {
    await navigateTo(currentCompany.value?.slug ? '/workspace/' + encodeURIComponent(currentCompany.value.slug) + '/dashboard' : '/dashboard')
    return
  }
  await load()
  const requestedCourseId = typeof route.query.checkoutCourse === 'string' ? route.query.checkoutCourse : ''
  if (requestedCourseId) {
    requestedCheckoutCourse.value = courses.value.find((course) => course.id === requestedCourseId && !enrolledIds.value.has(course.id)) ?? null
    if (!requestedCheckoutCourse.value && enrolledIds.value.has(requestedCourseId)) {
      successMessage.value = 'You already have access to this course.'
    }
  }
})

async function continueRequestedCheckout() {
  const course = requestedCheckoutCourse.value
  if (!course) return
  await enroll(course)
  if (!errorMessage.value) requestedCheckoutCourse.value = null
}
</script>

<template>
  <div class="min-h-screen bg-canvas">
    <header class="border-b border-line bg-white">
      <div class="mx-auto flex h-[72px] max-w-7xl items-center justify-between px-4 sm:px-6">
        <NuxtLink :to="currentCompany?.slug ? '/p/' + currentCompany.slug : '/'" class="inline-flex min-w-0 items-center gap-3 font-semibold"><span class="grid h-10 w-10 shrink-0 place-items-center rounded-2xl bg-brand text-lg font-black text-white">{{ (currentCompany?.name || 'N').slice(0, 1).toUpperCase() }}</span><span class="min-w-0"><span class="block truncate">{{ currentCompany?.name || 'newage' }}</span><span class="block text-[10px] font-medium uppercase tracking-wider text-muted">Learning dashboard</span></span></NuxtLink>
        <div class="flex items-center gap-3"><span class="hidden text-sm text-muted sm:block">{{ currentUser?.name }}</span><NuxtLink :to="currentCompany?.slug ? '/workspace/' + encodeURIComponent(currentCompany.slug) + '/dashboard' : '/dashboard'" class="rounded-xl border border-line px-4 py-2.5 text-sm font-semibold text-muted hover:bg-canvas">Workspace</NuxtLink></div>
      </div>
    </header>
    <main class="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-10">
      <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Learner space</p>
      <h1 class="mt-3 text-3xl font-semibold tracking-tight sm:text-4xl">Your learning, in one place.</h1>
      <p class="mt-3 max-w-2xl text-sm leading-6 text-muted">Pick up an enrolled course or find the next thing you want to learn. Payments are verified by the backend before access is granted.</p>

      <div v-if="errorMessage" role="alert" class="mt-5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700">{{ errorMessage }}</div>
      <div v-if="successMessage" role="status" class="mt-5 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm leading-6 text-emerald-800">{{ successMessage }}</div>

      <section v-if="requestedCheckoutCourse" class="mt-6 flex flex-col justify-between gap-4 rounded-2xl border border-brand/20 bg-white p-5 shadow-sm sm:flex-row sm:items-center sm:p-6">
        <div class="min-w-0"><p class="text-xs font-bold uppercase tracking-[0.16em] text-brand">Selected from the company public page</p><h2 class="mt-1 text-lg font-semibold">{{ requestedCheckoutCourse.title }}</h2><p class="mt-1 text-sm leading-6 text-muted">{{ requestedCheckoutCourse.priceInPaise === 0 ? 'This course is free. Confirm to add it to your learning list.' : 'Continue to the company payment checkout. Payment is verified by the API before access is granted.' }}</p></div>
        <button type="button" :disabled="busyCourse === requestedCheckoutCourse.id || checkoutBusy" class="shrink-0 rounded-xl bg-brand px-5 py-3 text-sm font-semibold text-white hover:bg-brand-dark disabled:opacity-60" @click="continueRequestedCheckout">{{ busyCourse === requestedCheckoutCourse.id || checkoutBusy ? 'Processing…' : (requestedCheckoutCourse.priceInPaise === 0 ? 'Confirm free enrolment' : 'Continue to secure payment') }}</button>
      </section>

      <section class="mt-10">
        <div class="mb-5 flex items-end justify-between gap-3"><div><h2 class="text-xl font-semibold">Continue learning</h2><p class="mt-1 text-sm text-muted">Courses you already have access to.</p></div><span class="rounded-full border border-line bg-white px-3 py-1.5 text-xs font-semibold text-muted">{{ enrolled.length }} enrolled</span></div>
        <div v-if="loading" class="grid gap-4 md:grid-cols-2 xl:grid-cols-3"><div v-for="n in 3" :key="n" class="h-52 animate-pulse rounded-2xl border border-line bg-white" /></div>
        <div v-else-if="enrolled.length" class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          <article v-for="item in enrolled" :key="item.courseId" class="rounded-2xl border border-line bg-white p-5 shadow-sm shadow-ink/[0.02]">
            <div class="flex items-start justify-between gap-3"><div class="grid h-11 w-11 place-items-center rounded-xl bg-brand-soft text-brand"><svg viewBox="0 0 24 24" fill="none" class="h-6 w-6" aria-hidden="true"><path d="M4 6.5A2.5 2.5 0 0 1 6.5 4H20v15H6.5A2.5 2.5 0 0 0 4 21V6.5ZM4 18.5A2.5 2.5 0 0 1 6.5 16H20" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg></div><span class="rounded-full bg-emerald-100 px-2.5 py-1 text-[10px] font-semibold text-emerald-800">Enrolled</span></div>
            <h3 class="mt-5 text-base font-semibold">{{ item.title }}</h3><p class="mt-2 line-clamp-2 min-h-10 text-sm leading-6 text-muted">{{ item.description || 'Ready when you are.' }}</p>
            <div class="mt-5"><div class="mb-2 flex justify-between text-xs"><span class="font-medium text-muted">Course progress</span><span class="font-semibold">{{ progressByCourse[item.courseId] || 0 }}%</span></div><div class="h-2 overflow-hidden rounded-full bg-canvas"><div class="h-full rounded-full bg-brand transition-all" :style="{ width: (progressByCourse[item.courseId] || 0) + '%' }" /></div></div>
            <NuxtLink :to="workspacePath('/courses/' + item.courseId)" class="mt-5 inline-flex w-full justify-center rounded-xl bg-brand px-4 py-3 text-sm font-semibold text-white hover:bg-brand-dark">Open course</NuxtLink>
          </article>
        </div>
        <div v-else-if="!loading" class="rounded-2xl border border-dashed border-slate-300 bg-white px-5 py-10 text-center"><p class="font-semibold">Your learning list is empty</p><p class="mt-2 text-sm text-muted">Enrol in a course below to get started.</p></div>
      </section>

      <section class="mt-12">
        <div class="mb-5 flex items-end justify-between gap-3"><div><h2 class="text-xl font-semibold">Discover courses</h2><p class="mt-1 text-sm text-muted">Published courses from your workspace.</p></div><span class="rounded-full border border-line bg-white px-3 py-1.5 text-xs font-semibold text-muted">{{ availableCourses.length }} available</span></div>
        <div v-if="!loading && !availableCourses.length" class="rounded-2xl border border-line bg-white px-5 py-10 text-center text-sm text-muted">You have explored every published course available in this workspace.</div>
        <div v-else class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          <article v-for="(course,index) in availableCourses" :key="course.id" class="overflow-hidden rounded-2xl border border-line bg-white">
            <div class="flex h-24 items-end p-4" :class="index % 3 === 0 ? 'bg-gradient-to-br from-[#e6e2ff] to-[#f4f2ff]' : index % 3 === 1 ? 'bg-gradient-to-br from-[#dff8e7] to-[#f0fbf3]' : 'bg-gradient-to-br from-[#ffedda] to-[#fff7ef]'"><span class="rounded-full bg-white/80 px-2.5 py-1 text-[10px] font-bold uppercase tracking-wider text-ink/65">Published course</span></div>
            <div class="p-5"><h3 class="font-semibold">{{ course.title }}</h3><p class="mt-2 line-clamp-2 min-h-10 text-sm leading-6 text-muted">{{ course.description || 'A new learning experience.' }}</p><div class="mt-5 flex items-center justify-between gap-3"><span class="text-sm font-semibold">{{ formatPrice(course.priceInPaise) }}</span><button type="button" :disabled="busyCourse === course.id || checkoutBusy" class="rounded-xl bg-brand px-4 py-2.5 text-xs font-semibold text-white hover:bg-brand-dark disabled:opacity-50" @click="enroll(course)">{{ busyCourse === course.id ? 'Starting…' : 'Enrol now' }}</button></div></div>
          </article>
        </div>
      </section>
    </main>
  </div>
</template>
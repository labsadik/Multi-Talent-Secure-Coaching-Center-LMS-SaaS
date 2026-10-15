<script setup lang="ts">
import type { ContentItem, Course, PlaybackInfo } from '~/types/api'
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: 'auth' })

type TopicOutline = ContentItem & { lessons: ContentItem[] }
type SubjectOutline = ContentItem & { topics: TopicOutline[] }
type VideoUploadTicket = {
  assetId: string
  status: string
  upload: {
    endpoint: string
    videoId: string
    libraryId: number
    authorizationSignature: string
    authorizationExpire: number
  }
}

const route = useRoute()
const courseId = computed(() => String(route.params.courseId || ''))
const {
  currentUser,
  getCourses,
  getSubjects,
  createSubject,
  getTopics,
  createTopic,
  getLessons,
  createLesson,
  uploadAsset,
  createVideoTicket,
  refreshAssetStatus,
  getPlayback,
  updateLessonProgress,
} = useAuth()

const course = ref<Course | null>(null)
const outline = ref<SubjectOutline[]>([])
const loading = ref(true)
const saving = ref(false)
const uploading = ref(false)
const uploadProgress = ref(0)
const selectedSubjectId = ref('')
const selectedTopicId = ref('')
const selectedAssetId = ref('')
const selectedAssetTopicId = ref('')
const pendingVideoAssetId = ref('')
const errorMessage = ref('')
const successMessage = ref('')
const subjectTitle = ref('')
const subjectSlug = ref('')
const topicTitle = ref('')
const topicSlug = ref('')
const lessonTitle = ref('')
const lessonSlug = ref('')
const lessonType = ref<'text' | 'video' | 'file'>('text')
const lessonContent = ref('')
const lessonPreview = ref(false)
const lessonDuration = ref(0)
const playback = ref<PlaybackInfo | null>(null)
const playbackLesson = ref<ContentItem | null>(null)
const busyLessonId = ref('')

const canManageContent = computed(() => ['owner', 'admin', 'instructor'].includes(currentUser.value?.role ?? ''))

watch(subjectTitle, (value) => { if (!subjectSlug.value) subjectSlug.value = slugify(value) })
watch(topicTitle, (value) => { if (!topicSlug.value) topicSlug.value = slugify(value) })
watch(lessonTitle, (value) => { if (!lessonSlug.value) lessonSlug.value = slugify(value) })

function slugify(value: string) {
  return value.trim().toLowerCase().normalize('NFKD').replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 80)
}
function protectedDownloadUrl(downloadUrl: string) {
  return downloadUrl.startsWith('/api/v1/')
    ? '/api/backend/' + downloadUrl.replace(/^\/api\/v1\//, '')
    : downloadUrl
}
function resetMessages() {
  errorMessage.value = ''
  successMessage.value = ''
}
async function loadOutline() {
  loading.value = true
  errorMessage.value = ''
  try {
    const allCourses = await getCourses()
    course.value = allCourses.find((item) => item.id === courseId.value) ?? null
    if (!course.value) throw new Error('The course was not found in this workspace.')
    const subjects = await getSubjects(courseId.value)
    outline.value = await Promise.all(subjects.map(async (subject) => {
      const topics = await getTopics(subject.id)
      return {
        ...subject,
        topics: await Promise.all(topics.map(async (topic) => ({
          ...topic,
          lessons: await getLessons(topic.id),
        }))),
      }
    }))
    if (!selectedSubjectId.value && outline.value[0]) selectedSubjectId.value = outline.value[0].id
    if (!selectedSubjectId.value && outline.value[0]) selectedSubjectId.value = outline.value[0].id
    if (!selectedTopicId.value && outline.value[0]?.topics[0]) selectedTopicId.value = outline.value[0].topics[0].id
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Course content could not be loaded.')
  } finally {
    loading.value = false
  }
}
async function saveSubject() {
  resetMessages()
  saving.value = true
  try {
    await createSubject(courseId.value, { title: subjectTitle.value.trim(), slug: subjectSlug.value.trim().toLowerCase() })
    subjectTitle.value = ''
    subjectSlug.value = ''
    await loadOutline()
    successMessage.value = 'Subject added.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Subject could not be created.')
  } finally { saving.value = false }
}
async function saveTopic(subjectId: string) {
  resetMessages()
  saving.value = true
  try {
    const topic = await createTopic(subjectId, { title: topicTitle.value.trim(), slug: topicSlug.value.trim().toLowerCase() })
    selectedTopicId.value = topic.id
    topicTitle.value = ''
    topicSlug.value = ''
    await loadOutline()
    successMessage.value = 'Topic added.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Topic could not be created.')
  } finally { saving.value = false }
}
async function saveLesson(topicId: string) {
  resetMessages()
  if (lessonType.value !== 'text' && selectedAssetTopicId.value !== topicId) {
    errorMessage.value = 'Upload or select a ready media asset from the same topic as this lesson.'
    return
  }
  saving.value = true
  try {
    const payload: {
      title: string; slug: string; lessonType: string; content?: string; mediaAssetId?: string
      durationSeconds?: number; isPreview?: boolean
    } = {
      title: lessonTitle.value.trim(),
      slug: lessonSlug.value.trim().toLowerCase(),
      lessonType: lessonType.value,
      durationSeconds: Number(lessonDuration.value || 0),
      isPreview: lessonPreview.value,
    }
    if (lessonType.value === 'text') payload.content = lessonContent.value.trim()
    else payload.mediaAssetId = selectedAssetId.value
    await createLesson(topicId, payload)
    lessonTitle.value = ''
    lessonSlug.value = ''
    lessonContent.value = ''
    lessonDuration.value = 0
    lessonPreview.value = false
    selectedAssetId.value = ''
    selectedAssetTopicId.value = ''
    pendingVideoAssetId.value = ''
    await loadOutline()
    successMessage.value = 'Lesson added.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Lesson could not be created. Video/file lessons need a ready media asset first.')
  } finally { saving.value = false }
}
async function uploadStorageFile(topicId: string, event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  resetMessages()
  uploading.value = true
  try {
    const result = await uploadAsset(topicId, file)
    const assetId = String(result.assetId || '')
    if (!assetId) throw new Error('The upload finished without an asset ID.')
    pendingVideoAssetId.value = ''
    selectedAssetId.value = assetId
    selectedAssetTopicId.value = topicId
    successMessage.value = file.type === 'application/pdf'
      ? 'PDF uploaded and selected for the next file lesson.'
      : 'Image uploaded and selected for the next file lesson.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'File upload failed. Confirm Bunny Storage is configured and that the file is supported.')
  } finally { uploading.value = false }
}
async function uploadVideo(topicId: string, event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  resetMessages()
  uploading.value = true
  uploadProgress.value = 0
  try {
    const raw = await createVideoTicket(topicId, file.name.replace(/\.[^.]+$/, ''))
    const ticket = raw as unknown as VideoUploadTicket
    if (!ticket.assetId || !ticket.upload?.endpoint) throw new Error('Bunny Stream did not return a complete upload ticket.')
    selectedAssetId.value = ticket.assetId
    selectedAssetTopicId.value = topicId
    pendingVideoAssetId.value = ticket.assetId
    const tusModule = await import('tus-js-client')
    const Upload = tusModule.Upload
    await new Promise<void>((resolve, reject) => {
      const upload = new Upload(file, {
        endpoint: ticket.upload.endpoint,
        headers: {
          AuthorizationSignature: ticket.upload.authorizationSignature,
          AuthorizationExpire: String(ticket.upload.authorizationExpire),
          VideoId: ticket.upload.videoId,
          LibraryId: String(ticket.upload.libraryId),
        },
        metadata: { filename: file.name, filetype: file.type || 'application/octet-stream' },
        retryDelays: [0, 1000, 3000, 5000],
        onError: reject,
        onProgress: (sent, total) => { uploadProgress.value = total ? Math.round(sent / total * 100) : 0 },
        onSuccess: () => resolve(),
      })
      upload.start()
    })
    successMessage.value = 'Video uploaded. Bunny must finish processing before you can attach it to a lesson. Use Refresh status below.'
    await loadOutline()
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Video upload failed. Check Bunny Stream configuration and upload access.')
  } finally { uploading.value = false }
}
async function refreshVideo(assetId: string) {
  resetMessages()
  try {
    const result = await refreshAssetStatus(assetId)
    const status = String(result.status || 'unknown')
    successMessage.value = 'Video status: ' + status + '. Select the asset for a video lesson when it is ready.'
    if (status === 'ready') {
      selectedAssetId.value = assetId
      pendingVideoAssetId.value = ''
    }
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Could not refresh video status. Bunny Stream may not be configured.')
  }
}
async function openLesson(lesson: ContentItem) {
  resetMessages()
  playbackLesson.value = lesson
  try {
    playback.value = await getPlayback(lesson.id)
  } catch (error) {
    playback.value = null
    errorMessage.value = apiErrorMessage(error, 'This lesson is not available to your account. Check enrolment or preview access.')
  }
}
async function markComplete(lesson: ContentItem) {
  busyLessonId.value = lesson.id
  resetMessages()
  try {
    await updateLessonProgress(lesson.id, 'completed', Number(lesson.durationSeconds || 0))
    successMessage.value = 'Lesson marked as completed.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Progress could not be saved. Learners need an active course enrolment.')
  } finally { busyLessonId.value = '' }
}

onMounted(() => { void loadOutline() })
</script>

<template>
  <div class="min-h-screen bg-canvas">
    <header class="border-b border-line bg-white">
      <div class="mx-auto flex h-[72px] max-w-6xl items-center justify-between gap-4 px-4 sm:px-6">
        <NuxtLink to="/dashboard" class="inline-flex items-center gap-3 font-semibold"><span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white">N</span>newage<span class="text-brand">.</span></NuxtLink>
        <NuxtLink to="/dashboard" class="rounded-xl border border-line px-4 py-2.5 text-sm font-semibold text-muted hover:bg-canvas">← Workspace</NuxtLink>
      </div>
    </header>
    <main class="mx-auto max-w-6xl px-4 py-8 sm:px-6 sm:py-10">
      <p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Course builder</p>
      <div class="mt-3 flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div><h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">{{ course?.title || 'Loading course…' }}</h1><p class="mt-2 max-w-2xl text-sm leading-6 text-muted">{{ course?.description || 'Build a clear learning path with subjects, topics, and lessons.' }}</p></div>
        <span v-if="course" class="w-fit rounded-full px-3 py-1.5 text-xs font-semibold capitalize" :class="course.status === 'published' ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'">{{ course.status }}</span>
      </div>
      <div v-if="errorMessage" role="alert" class="mt-5 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{{ errorMessage }}</div>
      <div v-if="successMessage" role="status" class="mt-5 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">{{ successMessage }}</div>

      <div class="mt-8 grid gap-6 lg:grid-cols-[0.85fr_1.15fr]">
        <aside v-if="canManageContent" class="space-y-5">
          <section class="rounded-2xl border border-line bg-white p-5">
            <h2 class="font-semibold">1. Add a subject</h2><p class="mt-1 text-xs leading-5 text-muted">A module or main section of the course.</p>
            <form class="mt-4 space-y-3" @submit.prevent="saveSubject">
              <input v-model="subjectTitle" required minlength="2" maxlength="140" placeholder="Subject title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="subjectSlug" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="subject-slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <button :disabled="saving || !course" class="w-full rounded-xl bg-brand px-4 py-3 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Saving…' : 'Add subject' }}</button>
            </form>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5">
            <h2 class="font-semibold">2. Add a topic</h2><p class="mt-1 text-xs leading-5 text-muted">Choose the subject that should contain it.</p>
            <form class="mt-4 space-y-3" @submit.prevent="saveTopic(selectedSubjectId)">
              <select v-model="selectedSubjectId" required class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand">
                <option value="" disabled>Select a subject</option><option v-for="subject in outline" :key="subject.id" :value="subject.id">{{ subject.title }}</option>
              </select>
              <input v-model="topicTitle" required minlength="2" maxlength="140" placeholder="Topic title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="topicSlug" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="topic-slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <button :disabled="saving || !selectedSubjectId" class="w-full rounded-xl border border-brand/20 bg-brand-soft px-4 py-3 text-sm font-semibold text-brand disabled:opacity-50">{{ saving ? 'Saving…' : 'Add topic' }}</button>
            </form>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5">
            <h2 class="font-semibold">3. Add a lesson</h2><p class="mt-1 text-xs leading-5 text-muted">Text lessons work without media-provider configuration.</p>
            <form class="mt-4 space-y-3" @submit.prevent="saveLesson(selectedTopicId)">
              <select v-model="selectedTopicId" required class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand">
                <option value="" disabled>Select a topic</option><option v-for="subject in outline" :key="subject.id" disabled>— {{ subject.title }} —</option><option v-for="subject in outline" :key="subject.id + '-topics'" disabled></option>
                <template v-for="subject in outline" :key="subject.id + '-select'"><option v-for="topic in subject.topics" :key="topic.id" :value="topic.id">{{ subject.title }} / {{ topic.title }}</option></template>
              </select>
              <input v-model="lessonTitle" required minlength="2" maxlength="140" placeholder="Lesson title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="lessonSlug" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="lesson-slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <select v-model="lessonType" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand">
                <option value="text">Text lesson</option><option value="video">Video lesson (requires ready video asset)</option><option value="file">File lesson (requires uploaded asset)</option>
              </select>
              <textarea v-if="lessonType === 'text'" v-model="lessonContent" required maxlength="50000" rows="5" placeholder="Write the lesson content…" class="w-full resize-y rounded-xl border border-line px-3.5 py-3 text-sm leading-6 outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <div v-else class="rounded-xl border border-dashed border-brand/30 bg-brand-soft/40 p-3 text-xs leading-5 text-muted">Upload a file or video in the relevant topic panel, then select its asset ID:
                <input v-model="selectedAssetId" required placeholder="Uploaded asset ID" class="mt-2 w-full rounded-lg border border-line bg-white px-3 py-2.5 font-mono text-xs outline-none focus:border-brand" />
              </div>
              <div><label for="lessonDuration" class="mb-1.5 block text-xs font-semibold text-muted">Duration (seconds, optional)</label><input id="lessonDuration" v-model.number="lessonDuration" type="number" min="0" max="86400" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /></div>
              <label class="flex items-center gap-2 text-sm text-muted"><input v-model="lessonPreview" type="checkbox" class="h-4 w-4 rounded border-line accent-brand" /> Allow as a preview for non-enrolled learners</label>
              <button :disabled="saving || !selectedTopicId" class="w-full rounded-xl bg-ink px-4 py-3 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Saving…' : 'Add lesson' }}</button>
            </form>
          </section>
        </aside>

        <section class="min-w-0 rounded-2xl border border-line bg-white p-5 sm:p-6">
          <div class="flex items-center justify-between gap-3"><div><h2 class="text-lg font-semibold">Learning outline</h2><p class="mt-1 text-xs text-muted">Subjects, topics and lessons in order.</p></div><span class="rounded-full bg-canvas px-3 py-1 text-xs font-semibold text-muted">{{ outline.length }} subjects</span></div>
          <div v-if="loading" class="mt-6 space-y-3"><div v-for="n in 4" :key="n" class="h-16 animate-pulse rounded-xl bg-canvas" /></div>
          <div v-else-if="!outline.length" class="mt-6 rounded-2xl border border-dashed border-slate-300 px-5 py-12 text-center"><h3 class="font-semibold">No subjects yet</h3><p class="mt-2 text-sm leading-6 text-muted">{{ canManageContent ? 'Add your first subject to start mapping the learning path.' : 'This course has not published any learning content yet.' }}</p></div>
          <div v-else class="mt-6 space-y-4">
            <article v-for="(subject, subjectIndex) in outline" :key="subject.id" class="overflow-hidden rounded-2xl border border-line">
              <div class="flex items-start gap-3 bg-canvas px-4 py-4"><span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-brand-soft text-xs font-bold text-brand">{{ String(subjectIndex + 1).padStart(2, '0') }}</span><div class="min-w-0"><h3 class="font-semibold">{{ subject.title }}</h3><p class="mt-1 text-xs text-muted">{{ subject.topics.length }} topics</p></div></div>
              <div class="space-y-3 p-3 sm:p-4">
                <div v-for="(topic, topicIndex) in subject.topics" :key="topic.id" class="rounded-xl border border-line p-3 sm:p-4">
                  <div class="flex items-center justify-between gap-3"><div class="flex min-w-0 items-center gap-2"><span class="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-emerald-100 text-xs font-bold text-emerald-800">{{ subjectIndex + 1 }}.{{ topicIndex + 1 }}</span><div class="min-w-0"><h4 class="truncate text-sm font-semibold">{{ topic.title }}</h4><p class="mt-0.5 text-[11px] text-muted">{{ topic.lessons.length }} lessons</p></div></div><span v-if="canManageContent" class="text-[10px] font-semibold uppercase tracking-wider text-muted">Manage below</span></div>
                  <div v-if="canManageContent" class="mt-3 grid gap-3 rounded-xl bg-canvas p-3 sm:grid-cols-2">
                    <label class="block text-xs font-semibold text-muted">Upload image/PDF
                      <input type="file" accept="image/jpeg,image/png,image/webp,image/gif,application/pdf" :disabled="uploading" class="mt-2 block w-full text-xs file:mr-2 file:rounded-lg file:border-0 file:bg-white file:px-3 file:py-2 file:text-xs file:font-semibold file:text-brand" @change="uploadStorageFile(topic.id, $event)" />
                    </label>
                    <label class="block text-xs font-semibold text-muted">Upload video
                      <input type="file" accept="video/*" :disabled="uploading" class="mt-2 block w-full text-xs file:mr-2 file:rounded-lg file:border-0 file:bg-white file:px-3 file:py-2 file:text-xs file:font-semibold file:text-brand" @change="uploadVideo(topic.id, $event)" />
                    </label>
                    <div v-if="uploading" class="text-xs font-medium text-brand sm:col-span-2">Uploading… {{ uploadProgress }}%</div>
                    <div v-if="selectedAssetId" class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-emerald-200 bg-emerald-50 p-2.5 text-xs text-emerald-800 sm:col-span-2"><span class="min-w-0 truncate font-mono">{{ selectedAssetId }}</span><div class="flex gap-3"><button v-if="pendingVideoAssetId" type="button" class="shrink-0 font-semibold underline" @click="refreshVideo(pendingVideoAssetId)">Refresh video status</button><button type="button" class="shrink-0 underline" @click="selectedAssetId = ''; pendingVideoAssetId = ''">Clear</button></div></div>
                  </div>
                  <div v-if="topic.lessons.length" class="mt-3 space-y-2">
                    <div v-for="lesson in topic.lessons" :key="lesson.id" class="flex flex-col gap-2 rounded-xl border border-line/70 px-3 py-3 sm:flex-row sm:items-center">
                      <div class="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-white text-muted ring-1 ring-line"><svg v-if="lesson.lessonType === 'video'" viewBox="0 0 24 24" fill="none" class="h-4 w-4" aria-hidden="true"><path d="m8 5 11 7-11 7V5Z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" /></svg><svg v-else-if="lesson.lessonType === 'text'" viewBox="0 0 24 24" fill="none" class="h-4 w-4" aria-hidden="true"><path d="M5 4h14v16H5zM8 8h8M8 12h8M8 16h5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg><svg v-else viewBox="0 0 24 24" fill="none" class="h-4 w-4" aria-hidden="true"><path d="M6 3h8l4 4v14H6zM14 3v5h5" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" /></svg></div>
                      <div class="min-w-0 flex-1"><p class="truncate text-sm font-medium">{{ lesson.title }}</p><p class="mt-0.5 text-[11px] capitalize text-muted">{{ lesson.lessonType || 'lesson' }}<span v-if="lesson.isPreview"> · preview</span></p></div>
                      <div class="flex gap-2"><button type="button" class="rounded-lg border border-line px-3 py-2 text-xs font-semibold hover:bg-canvas" @click="openLesson(lesson)">Open</button><button v-if="currentUser?.role === 'learner'" type="button" :disabled="busyLessonId === lesson.id" class="rounded-lg bg-emerald-100 px-3 py-2 text-xs font-semibold text-emerald-800 disabled:opacity-50" @click="markComplete(lesson)">{{ busyLessonId === lesson.id ? 'Saving…' : 'Complete' }}</button></div>
                      <button v-if="lesson.mediaAssetId && canManageContent" type="button" class="rounded-lg border border-line px-2 py-2 text-[10px] font-semibold text-muted hover:bg-canvas" @click="refreshVideo(lesson.mediaAssetId)">Refresh video</button>
                    </div>
                  </div>
                  <p v-else class="mt-3 rounded-lg bg-canvas px-3 py-2.5 text-xs text-muted">No lessons in this topic yet.</p>
                </div>
                <p v-if="!subject.topics.length" class="rounded-xl border border-dashed border-slate-300 px-4 py-5 text-center text-xs text-muted">No topics yet. Add one using the course builder.</p>
              </div>
            </article>
          </div>
        </section>
      </div>

      <div v-if="playbackLesson" class="fixed inset-0 z-50 flex items-center justify-center bg-ink/60 p-4" @click.self="playbackLesson = null; playback = null">
        <section class="max-h-[90vh] w-full max-w-3xl overflow-auto rounded-3xl bg-white p-5 shadow-2xl sm:p-7" role="dialog" aria-modal="true" :aria-label="playbackLesson.title">
          <div class="flex items-start justify-between gap-4"><div><p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Lesson</p><h2 class="mt-2 text-xl font-semibold">{{ playbackLesson.title }}</h2></div><button type="button" class="grid h-9 w-9 place-items-center rounded-xl border border-line text-muted hover:bg-canvas" aria-label="Close lesson" @click="playbackLesson = null; playback = null">×</button></div>
          <div v-if="playback?.content" class="prose prose-sm mt-5 max-w-none whitespace-pre-wrap leading-7 text-ink">{{ playback.content }}</div>
          <div v-else-if="playback?.playbackUrl" class="mt-5 aspect-video overflow-hidden rounded-2xl bg-black"><iframe :src="playback.playbackUrl" class="h-full w-full" allow="accelerometer; gyroscope; autoplay; encrypted-media; picture-in-picture" allowfullscreen title="Lesson video" /></div>
          <img v-else-if="playback?.downloadUrl && playback.contentType?.startsWith('image/')" :src="protectedDownloadUrl(playback.downloadUrl)" :alt="playbackLesson?.title || 'Lesson image'" class="mt-5 max-h-[65vh] w-full rounded-2xl border border-line bg-canvas object-contain" />
          <a v-else-if="playback?.downloadUrl" :href="protectedDownloadUrl(playback.downloadUrl)" target="_blank" rel="noopener noreferrer" class="mt-5 inline-flex rounded-xl bg-brand px-4 py-3 text-sm font-semibold text-white">Open lesson file</a>
          <p v-else class="mt-5 rounded-xl bg-canvas p-4 text-sm leading-6 text-muted">The lesson is unavailable or the media is still processing. Refresh its status or confirm learner enrolment.</p>
        </section>
      </div>
    </main>
  </div>
</template>
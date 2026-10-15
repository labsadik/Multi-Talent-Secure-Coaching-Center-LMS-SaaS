<script setup lang="ts">
import type { ContentItem, Course, PlaybackInfo, TopicAsset } from '~/types/api'
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({
  middleware: ['auth', 'company-dashboard'],
  alias: ['/workspace/:companySlug/dashboard/courses/:courseId'],
})

interface ChapterView extends ContentItem {
  lessons: ContentItem[]
  assets: TopicAsset[]
}
interface SubjectView extends ContentItem {
  chapters: ChapterView[]
}
type VideoStatusReply = { status?: string; encodeProgress?: number; bunnyStatus?: number }

const route = useRoute()
const {
  currentUser, currentCompany,
  getCourses, getSubjects, createSubject, updateSubject, deleteSubject,
  getTopics, createTopic, updateTopic, deleteTopic,
  getLessons, createLesson, updateLesson, deleteLesson,
  getChapters, getChapterParts, getChapterPartLessons,
  getTopicAssets, uploadAsset, createVideoTicket, refreshAssetStatus,
  getMyCourses, getPlayback, updateLessonProgress,
} = useAuth()

const courseId = computed(() => String(route.params.courseId || ''))
const tenantSlug = computed(() => String(route.params.companySlug || currentCompany.value?.slug || '').trim().toLowerCase())
const isManager = computed(() => ['owner', 'admin', 'instructor'].includes(currentUser.value?.role || ''))
const isLearner = computed(() => currentUser.value?.role === 'learner')
const dashboardPath = computed(() => tenantSlug.value
  ? '/workspace/' + encodeURIComponent(tenantSlug.value) + '/dashboard'
  : '/dashboard')
const publicPagePath = computed(() => currentCompany.value?.slug ? '/p/' + currentCompany.value.slug : '/')

const course = ref<Course | null>(null)
const subjects = ref<SubjectView[]>([])
const loading = ref(true)
const saving = ref(false)
const pageError = ref('')
const successMessage = ref('')
const subjectFormOpen = ref(false)
const editingSubjectId = ref('')
const subjectTitle = ref('')
const subjectSlug = ref('')
const subjectSlugTouched = ref(false)
const expandedChapterId = ref('')
const chapterFormSubjectId = ref('')
const editingChapterId = ref('')
const chapterTitle = ref('')
const chapterSlug = ref('')
const chapterSlugTouched = ref(false)

const activeLessonChapterId = ref('')
const editingLesson = ref<ContentItem | null>(null)
const lessonTitle = ref('')
const lessonSlug = ref('')
const lessonSlugTouched = ref(false)
const lessonType = ref<'text' | 'video' | 'file'>('text')
const lessonContent = ref('')
const lessonAssetId = ref('')
const lessonDuration = ref(0)
const lessonPreview = ref(false)

const uploadActivity = ref<{ chapterId: string; kind: 'file' | 'video'; filename: string } | null>(null)
const uploadProgress = ref(0)
const processingVideoIds = ref<string[]>([])
const pollingVideos = ref(false)
const playbackLesson = ref<ContentItem | null>(null)
const playback = ref<PlaybackInfo | null>(null)
const playbackBusy = ref(false)
const busyLessonId = ref('')

function slugify(value: string) {
  const trimmed = value.trim()
  if (!trimmed) return ''
  const ascii = trimmed.toLowerCase().normalize('NFKD')
    .replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 80).replace(/-+$/g, '')
  if (ascii.length >= 2) return ascii
  const encoded = Array.from(trimmed, character => character.codePointAt(0)!.toString(16)).join('-')
  const fallback = ('u-' + encoded).slice(0, 80).replace(/-+$/g, '')
  return fallback.length >= 2 ? fallback : 'unit'
}
watch(subjectTitle, value => { if (!subjectSlugTouched.value) subjectSlug.value = slugify(value) })
watch(chapterTitle, value => { if (!chapterSlugTouched.value) chapterSlug.value = slugify(value) })
watch(lessonTitle, value => { if (!lessonSlugTouched.value) lessonSlug.value = slugify(value) })

function resetMessages() {
  pageError.value = ''
  successMessage.value = ''
}
function formatSize(bytes: number) {
  if (!bytes) return ''
  if (bytes < 1024 * 1024) return Math.max(1, Math.round(bytes / 1024)) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}
function statusLabel(asset: TopicAsset) {
  if (asset.status === 'ready') return 'Ready'
  if (asset.status === 'failed') return 'Failed'
  if (asset.assetType === 'video' && asset.status !== 'uploading') return 'Processing on Bunny'
  return 'Uploading'
}
function statusClass(status: string) {
  if (status === 'ready') return 'bg-emerald-100 text-emerald-800'
  if (status === 'failed') return 'bg-red-100 text-red-800'
  if (status === 'processing') return 'bg-violet-100 text-violet-800'
  return 'bg-amber-100 text-amber-900'
}
function availableAssets(chapter: ChapterView) {
  return chapter.assets.filter(asset => {
    if (asset.status !== 'ready') return false
    if (asset.chapterPartId) return false
    if (asset.lessonId && asset.lessonId !== editingLesson.value?.id) return false
    return lessonType.value === 'video'
      ? asset.assetType === 'video'
      : asset.assetType === 'image' || asset.assetType === 'file'
  })
}
function resetLessonForm() {
  activeLessonChapterId.value = ''
  editingLesson.value = null
  lessonTitle.value = ''
  lessonSlug.value = ''
  lessonSlugTouched.value = false
  lessonType.value = 'text'
  lessonContent.value = ''
  lessonAssetId.value = ''
  lessonDuration.value = 0
  lessonPreview.value = false
}
function openLessonForm(chapter: ChapterView) {
  resetMessages()
  resetLessonForm()
  activeLessonChapterId.value = chapter.id
}
function editSubject(subject: SubjectView) {
  resetMessages()
  editingSubjectId.value = subject.id
  subjectTitle.value = subject.title
  subjectSlug.value = subject.slug
  subjectSlugTouched.value = true
  subjectFormOpen.value = true
}
function editChapter(subjectId: string, chapter: ChapterView) {
  resetMessages()
  chapterFormSubjectId.value = subjectId
  editingChapterId.value = chapter.id
  chapterTitle.value = chapter.title
  chapterSlug.value = chapter.slug
  chapterSlugTouched.value = true
}
function editLesson(chapter: ChapterView, lesson: ContentItem) {
  resetMessages()
  activeLessonChapterId.value = chapter.id
  editingLesson.value = lesson
  lessonTitle.value = lesson.title
  lessonSlug.value = lesson.slug
  lessonSlugTouched.value = true
  lessonType.value = (lesson.lessonType || 'text') as 'text' | 'video' | 'file'
  lessonContent.value = lesson.content || ''
  lessonAssetId.value = lesson.mediaAssetId || ''
  lessonDuration.value = Number(lesson.durationSeconds || 0)
  lessonPreview.value = Boolean(lesson.isPreview)
  expandedChapterId.value = chapter.id
}
function cancelChapterForm() {
  chapterFormSubjectId.value = ''
  editingChapterId.value = ''
  chapterTitle.value = ''
  chapterSlug.value = ''
  chapterSlugTouched.value = false
}
async function loadOutline(showLoading = true) {
  if (showLoading) loading.value = true
  pageError.value = ''
  try {
    const allCourses = await getCourses()
    course.value = allCourses.find(item => item.id === courseId.value) ?? null
    if (!course.value) throw new Error('This course is not available in the current company workspace.')

    if (isLearner.value) {
      const enrolments = await getMyCourses()
      if (!enrolments.some(item => item.courseId === courseId.value)) {
        await navigateTo({ path: dashboardPath.value + '/learn', query: { checkoutCourse: courseId.value } })
        return
      }
    }

    const rawSubjects = await getSubjects(courseId.value)
    subjects.value = await Promise.all(rawSubjects.map(async subject => {
      const rawChapters = await getTopics(subject.id)
      const chapters = await Promise.all(rawChapters.map(async chapter => {
        const directLessons = await getLessons(chapter.id)
        // Keep lessons created before the simplified curriculum UI. Legacy chapter/part
        // levels are flattened into the same lesson list; no old records are deleted.
        let legacyLessons: ContentItem[] = []
        try {
          const legacyChapters = await getChapters(chapter.id)
          for (const legacyChapter of legacyChapters) {
            const parts = await getChapterParts(legacyChapter.id).catch(() => [])
            for (const part of parts) {
              const partLessons = await getChapterPartLessons(part.id).catch(() => [])
              legacyLessons.push(...partLessons)
            }
          }
        } catch { /* some older courses have no legacy chapter tree */ }
        const lessons = [...new Map([...directLessons, ...legacyLessons].map(item => [item.id, item])).values()]
          .sort((a, b) => (a.sortOrder || 0) - (b.sortOrder || 0))
        const assets = isManager.value ? await getTopicAssets(chapter.id).catch(() => []) : []
        return { ...chapter, lessons, assets }
      }))
      return { ...subject, chapters }
    }))
    if (!expandedChapterId.value || !subjects.value.some(subject => subject.chapters.some(chapter => chapter.id === expandedChapterId.value))) {
      expandedChapterId.value = subjects.value[0]?.chapters[0]?.id || ''
    }
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'Course content could not be loaded.')
  } finally {
    if (showLoading) loading.value = false
  }
}
async function saveSubject() {
  resetMessages()
  saving.value = true
  try {
    const payload = { title: subjectTitle.value.trim(), slug: subjectSlug.value.trim().toLowerCase(), sortOrder: subjects.value.length }
    if (editingSubjectId.value) {
      await updateSubject(editingSubjectId.value, payload)
      successMessage.value = 'Subject saved.'
    } else {
      await createSubject(courseId.value, payload)
      successMessage.value = 'Subject created.'
    }
    subjectFormOpen.value = false
    editingSubjectId.value = ''
    subjectTitle.value = ''
    subjectSlug.value = ''
    subjectSlugTouched.value = false
    await loadOutline(false)
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'Subject could not be saved.')
  } finally { saving.value = false }
}
async function removeSubject(subject: SubjectView) {
  if (!import.meta.client || !window.confirm('Archive "' + subject.title + '" and its curriculum? The content is kept in the database as archived data.')) return
  resetMessages()
  try {
    await deleteSubject(subject.id)
    if (chapterFormSubjectId.value === subject.id) cancelChapterForm()
    successMessage.value = 'Subject archived.'
    await loadOutline(false)
  } catch (error) { pageError.value = apiErrorMessage(error, 'Subject could not be archived.') }
}
function openChapterForm(subject: SubjectView) {
  cancelChapterForm()
  chapterFormSubjectId.value = subject.id
}
async function saveChapter(subject: SubjectView) {
  resetMessages()
  saving.value = true
  try {
    const payload = { title: chapterTitle.value.trim(), slug: chapterSlug.value.trim().toLowerCase(), sortOrder: subject.chapters.length }
    let chapterId = ''
    if (editingChapterId.value) {
      const updated = await updateTopic(editingChapterId.value, payload)
      chapterId = updated.id
      successMessage.value = 'Chapter saved.'
    } else {
      const created = await createTopic(subject.id, payload)
      chapterId = created.id
      successMessage.value = 'Chapter created.'
    }
    cancelChapterForm()
    await loadOutline(false)
    expandedChapterId.value = chapterId
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'Chapter could not be saved.')
  } finally { saving.value = false }
}
async function removeChapter(chapter: ChapterView) {
  if (!import.meta.client || !window.confirm('Archive "' + chapter.title + '" and its lessons?')) return
  resetMessages()
  try {
    await deleteTopic(chapter.id)
    if (expandedChapterId.value === chapter.id) expandedChapterId.value = ''
    if (activeLessonChapterId.value === chapter.id) resetLessonForm()
    successMessage.value = 'Chapter archived.'
    await loadOutline(false)
  } catch (error) { pageError.value = apiErrorMessage(error, 'Chapter could not be archived.') }
}
async function saveLesson(chapter: ChapterView) {
  resetMessages()
  saving.value = true
  try {
    const payload = {
      title: lessonTitle.value.trim(),
      slug: lessonSlug.value.trim().toLowerCase(),
      lessonType: lessonType.value,
      content: lessonType.value === 'text' ? lessonContent.value.trim() : undefined,
      mediaAssetId: lessonType.value === 'text' ? undefined : lessonAssetId.value || undefined,
      durationSeconds: Math.max(0, Math.round(Number(lessonDuration.value || 0))),
      isPreview: lessonPreview.value,
      sortOrder: editingLesson.value?.sortOrder ?? chapter.lessons.length,
    }
    if (editingLesson.value) {
      await updateLesson(editingLesson.value.id, payload)
      successMessage.value = 'Lesson saved.'
    } else {
      await createLesson(chapter.id, payload)
      successMessage.value = 'Lesson created.'
    }
    resetLessonForm()
    await loadOutline(false)
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'Lesson could not be saved. Media lessons can only use ready assets from this chapter.')
  } finally { saving.value = false }
}
async function removeLesson(lesson: ContentItem) {
  if (!import.meta.client || !window.confirm('Archive "' + lesson.title + '"?')) return
  resetMessages()
  try {
    await deleteLesson(lesson.id)
    successMessage.value = 'Lesson archived.'
    await loadOutline(false)
  } catch (error) { pageError.value = apiErrorMessage(error, 'Lesson could not be archived.') }
}
async function uploadStorageFile(chapter: ChapterView, event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  if (!['image/jpeg', 'image/png', 'image/webp', 'image/gif', 'application/pdf'].includes(file.type)) {
    pageError.value = 'Choose a JPG, PNG, WebP, GIF image or PDF file.'
    return
  }
  if (file.size > 20 * 1024 * 1024) {
    pageError.value = 'Images and PDFs must be 20 MB or smaller. Video uploads use Bunny Stream instead.'
    return
  }
  resetMessages()
  uploadActivity.value = { chapterId: chapter.id, kind: 'file', filename: file.name }
  uploadProgress.value = 0
  try {
    await uploadAsset(chapter.id, file, undefined, percent => { uploadProgress.value = percent })
    successMessage.value = file.name + ' uploaded to Bunny Storage and is ready to attach.'
    await loadOutline(false)
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'File upload failed. Check Bunny Storage configuration and try again.')
  } finally { uploadActivity.value = null }
}
async function uploadVideo(chapter: ChapterView, event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  if (!file.type.startsWith('video/')) {
    pageError.value = 'Choose a video file for Bunny Stream.'
    return
  }
  resetMessages()
  uploadActivity.value = { chapterId: chapter.id, kind: 'video', filename: file.name }
  uploadProgress.value = 0
  try {
    const ticket = await createVideoTicket(chapter.id, file.name.replace(/\.[^.]+$/, '')) as unknown as {
      assetId: string
      status: string
      upload: { endpoint: string; videoId: string; libraryId: number; authorizationSignature: string; authorizationExpire: number }
    }
    if (!ticket.assetId || !ticket.upload?.endpoint) throw new Error('Bunny Stream did not return a valid upload ticket.')
    const tusModule = await import('tus-js-client')
    await new Promise<void>((resolve, reject) => {
      const upload = new tusModule.Upload(file, {
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
    await loadOutline(false)
    successMessage.value = 'Upload finished. Bunny Stream is processing the video; it can be attached after it becomes Ready.'
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'Video upload failed. Confirm Bunny Stream settings and retry.')
  } finally {
    uploadActivity.value = null
  }
}
async function refreshAllVideoStatuses() {
  if (pollingVideos.value || !isManager.value) return
  pollingVideos.value = true
  try {
    let cycles = 0
    while (cycles < 360) {
      const pending = subjects.value.flatMap(subject => subject.chapters.flatMap(chapter =>
        chapter.assets.filter(asset => asset.assetType === 'video' && ['uploading', 'processing'].includes(asset.status))
          .map(asset => ({ chapterId: chapter.id, asset })),
      ))
      processingVideoIds.value = pending.map(item => item.asset.id)
      if (!pending.length) break
      cycles += 1
      for (const item of pending) {
        try {
          await refreshAssetStatus(item.asset.id)
        } catch {
          // Webhook delivery is the source of truth; polling is a safe fallback while this editor is open.
        }
      }
      const reloaded = await Promise.all(subjects.value.map(async subject => ({
        ...subject,
        chapters: await Promise.all(subject.chapters.map(async chapter => ({
          ...chapter,
          assets: await getTopicAssets(chapter.id).catch(() => chapter.assets),
        }))),
      })))
      subjects.value = reloaded
      await new Promise(resolve => setTimeout(resolve, 5000))
    }
  } finally {
    processingVideoIds.value = []
    pollingVideos.value = false
  }
}
async function openLesson(lesson: ContentItem) {
  resetMessages()
  playbackBusy.value = true
  playbackLesson.value = lesson
  playback.value = null
  try {
    playback.value = await getPlayback(lesson.id)
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'Lesson is not available. Check course enrolment or ask the company for access.')
    playbackLesson.value = null
  } finally { playbackBusy.value = false }
}
function protectedDownloadUrl(url: string) {
  return url.startsWith('/api/v1/')
    ? '/api/backend/' + url.replace(/^\/api\/v1\//, '')
    : url
}
async function markComplete(lesson: ContentItem) {
  busyLessonId.value = lesson.id
  resetMessages()
  try {
    await updateLessonProgress(lesson.id, 'completed', Number(lesson.durationSeconds || 0))
    successMessage.value = 'Lesson marked complete.'
  } catch (error) {
    pageError.value = apiErrorMessage(error, 'Progress could not be saved.')
  } finally { busyLessonId.value = '' }
}
onMounted(async () => {
  await loadOutline()
  if (isManager.value) void refreshAllVideoStatuses()
})
</script>

<template>
  <div class="min-h-screen bg-slate-50 text-slate-900">
    <header class="sticky top-0 z-30 border-b border-slate-200 bg-white/95 backdrop-blur">
      <div class="mx-auto flex h-16 max-w-6xl items-center justify-between gap-3 px-4 sm:px-6">
        <div class="min-w-0">
          <NuxtLink :to="dashboardPath" class="text-xs font-semibold text-slate-500 hover:text-blue-700">← {{ currentCompany?.name || 'Company dashboard' }}</NuxtLink>
          <h1 class="truncate text-sm font-semibold sm:text-base">{{ course?.title || 'Course curriculum' }}</h1>
        </div>
        <NuxtLink :to="publicPagePath" class="shrink-0 rounded-lg border border-slate-200 px-3 py-2 text-xs font-semibold text-slate-700 hover:bg-slate-50">Public page</NuxtLink>
      </div>
    </header>

    <main class="mx-auto max-w-6xl px-4 py-6 sm:px-6 sm:py-9">
      <div v-if="pageError" role="alert" class="mb-4 flex items-start justify-between gap-3 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
        <span>{{ pageError }}</span><button type="button" class="shrink-0 font-semibold underline" @click="loadOutline()">Retry</button>
      </div>
      <div v-if="successMessage" role="status" class="mb-4 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">{{ successMessage }}</div>

      <section v-if="loading" class="space-y-3">
        <div v-for="n in 4" :key="n" class="h-16 animate-pulse rounded-xl border border-slate-200 bg-white" />
      </section>

      <template v-else-if="course">
        <section class="mb-6 flex flex-col justify-between gap-4 rounded-2xl border border-slate-200 bg-white p-5 sm:flex-row sm:items-center sm:p-6">
          <div>
            <p class="text-xs font-semibold uppercase tracking-wider text-blue-700">{{ isManager ? 'Course editor' : 'Your learning' }}</p>
            <h2 class="mt-1 text-2xl font-semibold tracking-tight">{{ course.title }}</h2>
            <p class="mt-2 max-w-2xl text-sm leading-6 text-slate-600">{{ course.description || 'The course curriculum, organized into subjects, chapters and lessons.' }}</p>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <span class="rounded-full px-3 py-1.5 text-xs font-semibold" :class="course.status === 'published' ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-900'">{{ course.status }}</span>
            <span class="rounded-full bg-slate-100 px-3 py-1.5 text-xs font-semibold text-slate-700">{{ subjects.length }} subjects</span>
          </div>
        </section>

        <section v-if="isManager" class="mb-5 rounded-2xl border border-slate-200 bg-white p-4 sm:p-5">
          <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
            <div><h3 class="font-semibold">1. Subjects</h3><p class="mt-1 text-sm text-slate-500">Start with a subject, then add chapters and lessons inside it.</p></div>
            <button type="button" class="rounded-lg bg-blue-700 px-4 py-2.5 text-sm font-semibold text-white hover:bg-blue-800" @click="subjectFormOpen = !subjectFormOpen; editingSubjectId = ''; subjectTitle = ''; subjectSlug = ''; subjectSlugTouched = false">{{ subjectFormOpen && !editingSubjectId ? 'Close' : 'Add subject' }}</button>
          </div>
          <form v-if="subjectFormOpen" class="mt-4 grid gap-3 border-t border-slate-100 pt-4 sm:grid-cols-2" @submit.prevent="saveSubject">
            <div><label for="subjectTitle" class="mb-1.5 block text-xs font-semibold text-slate-600">Subject name</label><input id="subjectTitle" v-model="subjectTitle" required minlength="2" maxlength="140" placeholder="e.g. Mathematics" class="w-full rounded-lg border border-slate-200 px-3 py-2.5 text-sm outline-none focus:border-blue-600 focus:ring-2 focus:ring-blue-100" /></div>
            <div><label for="subjectSlug" class="mb-1.5 block text-xs font-semibold text-slate-600">URL slug</label><input id="subjectSlug" v-model="subjectSlug" @input="subjectSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" class="w-full rounded-lg border border-slate-200 px-3 py-2.5 text-sm outline-none focus:border-blue-600 focus:ring-2 focus:ring-blue-100" /></div>
            <div class="flex gap-2 sm:col-span-2"><button type="submit" :disabled="saving" class="rounded-lg bg-blue-700 px-4 py-2.5 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Saving…' : (editingSubjectId ? 'Save subject' : 'Create subject') }}</button><button type="button" class="rounded-lg border border-slate-200 px-4 py-2.5 text-sm font-semibold text-slate-600" @click="subjectFormOpen = false; editingSubjectId = ''">Cancel</button></div>
          </form>
        </section>

        <div v-if="subjects.length" class="space-y-4">
          <section v-for="(subject, subjectIndex) in subjects" :key="subject.id" class="overflow-hidden rounded-2xl border border-slate-200 bg-white">
            <div class="flex flex-wrap items-start gap-3 border-b border-slate-100 p-4 sm:p-5">
              <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-blue-50 text-sm font-bold text-blue-800">{{ subjectIndex + 1 }}</span>
              <div class="min-w-0 flex-1"><h3 class="text-base font-semibold">{{ subject.title }}</h3><p class="mt-1 text-xs text-slate-500">{{ subject.chapters.length }} chapters · {{ subject.chapters.reduce((sum, chapter) => sum + chapter.lessons.length, 0) }} lessons</p></div>
              <div v-if="isManager" class="flex gap-2"><button type="button" class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-semibold text-slate-700" @click="editSubject(subject)">Edit</button><button type="button" class="rounded-lg border border-red-200 px-3 py-1.5 text-xs font-semibold text-red-700" @click="removeSubject(subject)">Archive</button></div>
            </div>

            <div class="space-y-3 p-3 sm:p-5">
              <div v-for="(chapter, chapterIndex) in subject.chapters" :key="chapter.id" class="overflow-hidden rounded-xl border border-slate-200">
                <button type="button" class="flex w-full items-center gap-3 bg-slate-50 px-3 py-3 text-left hover:bg-slate-100 sm:px-4" :aria-expanded="expandedChapterId === chapter.id" @click="expandedChapterId = expandedChapterId === chapter.id ? '' : chapter.id">
                  <span class="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-white text-xs font-bold text-slate-600 ring-1 ring-slate-200">{{ subjectIndex + 1 }}.{{ chapterIndex + 1 }}</span>
                  <span class="min-w-0 flex-1"><span class="block truncate text-sm font-semibold">{{ chapter.title }}</span><span class="mt-0.5 block text-xs text-slate-500">{{ chapter.lessons.length }} lessons</span></span>
                  <span class="text-lg text-slate-400">{{ expandedChapterId === chapter.id ? '−' : '+' }}</span>
                </button>

                <div v-if="expandedChapterId === chapter.id" class="space-y-4 p-3 sm:p-4">
                  <div v-if="isManager" class="flex flex-wrap gap-2">
                    <button type="button" class="rounded-lg border border-slate-200 px-3 py-2 text-xs font-semibold text-slate-700 hover:border-blue-400" @click="editChapter(subject.id, chapter)">Edit chapter</button>
                    <button type="button" class="rounded-lg border border-red-200 px-3 py-2 text-xs font-semibold text-red-700" @click="removeChapter(chapter)">Archive chapter</button>
                    <button type="button" class="ml-auto rounded-lg bg-blue-700 px-3 py-2 text-xs font-semibold text-white hover:bg-blue-800" @click="openLessonForm(chapter)">+ Add lesson</button>
                  </div>

                  <div v-if="chapter.lessons.length" class="divide-y divide-slate-100 rounded-xl border border-slate-100">
                    <article v-for="(lesson, lessonIndex) in chapter.lessons" :key="lesson.id" class="flex flex-wrap items-center gap-3 p-3">
                      <span class="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-slate-100 text-[10px] font-bold text-slate-700">{{ lesson.lessonType === 'video' ? '▶' : lesson.lessonType === 'file' ? 'PDF' : 'TXT' }}</span>
                      <div class="min-w-0 flex-1"><p class="text-sm font-semibold">{{ lessonIndex + 1 }}. {{ lesson.title }}</p><p class="mt-1 text-xs text-slate-500">{{ lesson.lessonType || 'text' }}<span v-if="lesson.durationSeconds"> · {{ Math.ceil(lesson.durationSeconds / 60) }} min</span><span v-if="lesson.isPreview"> · Preview</span></p></div>
                      <div class="flex flex-wrap gap-2"><button type="button" class="rounded-lg border border-slate-200 px-3 py-2 text-xs font-semibold text-slate-700 hover:border-blue-400" @click="openLesson(lesson)">{{ isManager ? 'Preview' : 'Open lesson' }}</button><button v-if="isManager" type="button" class="rounded-lg border border-slate-200 px-3 py-2 text-xs font-semibold text-slate-700" @click="editLesson(chapter, lesson)">Edit</button><button v-if="isManager" type="button" class="rounded-lg border border-red-200 px-3 py-2 text-xs font-semibold text-red-700" @click="removeLesson(lesson)">Archive</button><button v-if="isLearner" type="button" :disabled="busyLessonId === lesson.id" class="rounded-lg bg-emerald-100 px-3 py-2 text-xs font-semibold text-emerald-800 disabled:opacity-50" @click="markComplete(lesson)">{{ busyLessonId === lesson.id ? 'Saving…' : 'Mark complete' }}</button></div>
                    </article>
                  </div>
                  <p v-else class="rounded-xl border border-dashed border-slate-200 px-4 py-7 text-center text-sm text-slate-500">No lessons yet. Add the first lesson to this chapter.</p>

                  <form v-if="isManager && activeLessonChapterId === chapter.id" class="space-y-3 rounded-xl border border-blue-100 bg-blue-50/50 p-4" @submit.prevent="saveLesson(chapter)">
                    <div class="flex items-center justify-between gap-3"><h4 class="text-sm font-semibold">{{ editingLesson ? 'Edit lesson' : 'New lesson' }}</h4><button type="button" class="text-xs font-semibold text-slate-500 underline" @click="resetLessonForm">Close</button></div>
                    <div class="grid gap-3 sm:grid-cols-2">
                      <div><label for="lessonTitle" class="mb-1.5 block text-xs font-semibold text-slate-600">Lesson title</label><input id="lessonTitle" v-model="lessonTitle" required minlength="2" maxlength="140" placeholder="Lesson name" class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm outline-none focus:border-blue-600 focus:ring-2 focus:ring-blue-100" /></div>
                      <div><label for="lessonSlug" class="mb-1.5 block text-xs font-semibold text-slate-600">URL slug</label><input id="lessonSlug" v-model="lessonSlug" @input="lessonSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm outline-none focus:border-blue-600 focus:ring-2 focus:ring-blue-100" /></div>
                      <div><label for="lessonType" class="mb-1.5 block text-xs font-semibold text-slate-600">Lesson type</label><select id="lessonType" v-model="lessonType" class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm"><option value="text">Text lesson</option><option value="video">Video lesson</option><option value="file">PDF or image</option></select></div>
                      <div><label for="lessonDuration" class="mb-1.5 block text-xs font-semibold text-slate-600">Duration (minutes, optional)</label><input id="lessonDuration" v-model.number="lessonDuration" type="number" min="0" max="1440" class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm" /></div>
                    </div>
                    <div v-if="lessonType === 'text'"><label for="lessonContent" class="mb-1.5 block text-xs font-semibold text-slate-600">Lesson content</label><textarea id="lessonContent" v-model="lessonContent" required rows="4" maxlength="50000" placeholder="Write the lesson content…" class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm leading-6 outline-none focus:border-blue-600 focus:ring-2 focus:ring-blue-100" /></div>
                    <template v-else>
                      <div><label :for="'asset-' + chapter.id" class="mb-1.5 block text-xs font-semibold text-slate-600">Ready media from Bunny</label><select :id="'asset-' + chapter.id" v-model="lessonAssetId" required class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm"><option value="" disabled>Select a ready {{ lessonType === 'video' ? 'video' : 'PDF/image' }}</option><option v-for="asset in availableAssets(chapter)" :key="asset.id" :value="asset.id">{{ asset.title }} · {{ asset.status }}</option></select><p v-if="!availableAssets(chapter).length" class="mt-1 text-xs text-slate-500">Upload the media below. It appears here when Ready.</p></div>
                    </template>
                    <label class="flex items-center gap-2 text-xs text-slate-600"><input v-model="lessonPreview" type="checkbox" class="h-4 w-4 rounded border-slate-300 accent-blue-700" /> Allow preview before enrolment</label>
                    <div class="flex flex-wrap gap-2"><button type="submit" :disabled="saving" class="rounded-lg bg-blue-700 px-4 py-2.5 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Saving…' : (editingLesson ? 'Save lesson' : 'Create lesson') }}</button><button type="button" class="rounded-lg border border-slate-200 bg-white px-4 py-2.5 text-sm font-semibold text-slate-700" @click="resetLessonForm">Cancel</button></div>
                  </form>

                  <section v-if="isManager" class="rounded-xl border border-slate-200 p-4">
                    <div class="mb-3"><h4 class="text-sm font-semibold">Chapter media</h4><p class="mt-1 text-xs leading-5 text-slate-500">Images and PDFs go to Bunny Storage. Videos upload to Bunny Stream and must finish processing before use.</p></div>
                    <div class="grid gap-3 sm:grid-cols-2">
                      <label class="flex cursor-pointer items-center gap-3 rounded-lg border border-slate-200 px-3 py-3 hover:border-blue-400"><span class="grid h-9 w-9 place-items-center rounded-lg bg-slate-100 text-xs font-bold">PDF</span><span class="min-w-0 flex-1"><span class="block text-sm font-semibold">Upload image or PDF</span><span class="block text-xs text-slate-500">JPG, PNG, WebP, GIF, PDF · max 20 MB</span></span><input type="file" accept="image/jpeg,image/png,image/webp,image/gif,application/pdf" class="sr-only" :disabled="!!uploadActivity" @change="uploadStorageFile(chapter, $event)" /></label>
                      <label class="flex cursor-pointer items-center gap-3 rounded-lg border border-slate-200 px-3 py-3 hover:border-blue-400"><span class="grid h-9 w-9 place-items-center rounded-lg bg-violet-100 text-xs font-bold text-violet-800">▶</span><span class="min-w-0 flex-1"><span class="block text-sm font-semibold">Upload video</span><span class="block text-xs text-slate-500">Bunny Stream · processing tracked</span></span><input type="file" accept="video/*" class="sr-only" :disabled="!!uploadActivity" @change="uploadVideo(chapter, $event)" /></label>
                    </div>
                    <div v-if="uploadActivity?.chapterId === chapter.id" class="mt-4 rounded-lg bg-slate-50 p-3">
                      <div class="flex items-center justify-between gap-3 text-xs"><span class="min-w-0 truncate font-medium">{{ uploadActivity.filename }}</span><span class="shrink-0 font-semibold">{{ uploadProgress }}%</span></div>
                      <div class="mt-2 h-2 overflow-hidden rounded-full bg-slate-200"><div class="h-full rounded-full bg-blue-700 transition-all" :style="{ width: uploadProgress + '%' }" /></div>
                      <p class="mt-2 text-xs text-slate-500">{{ uploadActivity.kind === 'video' ? 'Uploading video to Bunny Stream…' : 'Uploading file to Bunny Storage…' }}</p>
                    </div>
                    <div v-if="chapter.assets.length" class="mt-4 space-y-2">
                      <article v-for="asset in chapter.assets" :key="asset.id" class="rounded-lg border border-slate-100 p-3">
                        <div class="flex items-start gap-3"><span class="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-slate-100 text-[10px] font-bold">{{ asset.assetType === 'video' ? '▶' : asset.assetType === 'file' ? 'PDF' : 'IMG' }}</span><div class="min-w-0 flex-1"><p class="break-words text-xs font-semibold">{{ asset.title }}</p><p class="mt-1 text-[11px] text-slate-500">{{ asset.provider === 'bunny_stream' ? 'Bunny Stream' : 'Bunny Storage' }}<span v-if="asset.sizeBytes"> · {{ formatSize(asset.sizeBytes) }}</span><span v-if="asset.assetType === 'video' && asset.status === 'processing'"> · {{ asset.encodeProgress ?? 0 }}% encoded</span></p></div><span class="shrink-0 rounded-full px-2 py-1 text-[10px] font-semibold" :class="statusClass(asset.status)">{{ statusLabel(asset) }}</span></div>
                        <div v-if="asset.assetType === 'video' && ['uploading', 'processing'].includes(asset.status)" class="mt-2 h-1.5 overflow-hidden rounded-full bg-slate-100"><div class="h-full rounded-full bg-violet-600 transition-all" :style="{ width: (asset.status === 'processing' ? (asset.encodeProgress ?? 5) : 5) + '%' }" /></div>
                        <p v-if="asset.status === 'failed'" class="mt-2 text-xs text-red-700">Bunny reported a processing failure. Upload the video again after checking the source file.</p>
                      </article>
                    </div>
                    <p v-else class="mt-3 text-xs text-slate-500">No media uploaded to this chapter yet.</p>
                  </section>
                </div>
              </div>

              <form v-if="isManager && chapterFormSubjectId === subject.id" class="grid gap-3 rounded-xl border border-blue-100 bg-blue-50/50 p-4 sm:grid-cols-2" @submit.prevent="saveChapter(subject)">
                <div class="sm:col-span-2"><h4 class="text-sm font-semibold">{{ editingChapterId ? 'Edit chapter' : 'New chapter' }}</h4></div>
                <div><label for="chapterTitle" class="mb-1.5 block text-xs font-semibold text-slate-600">Chapter name</label><input id="chapterTitle" v-model="chapterTitle" required minlength="2" maxlength="140" placeholder="e.g. Algebra basics" class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm outline-none focus:border-blue-600 focus:ring-2 focus:ring-blue-100" /></div>
                <div><label for="chapterSlug" class="mb-1.5 block text-xs font-semibold text-slate-600">URL slug</label><input id="chapterSlug" v-model="chapterSlug" @input="chapterSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" class="w-full rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm outline-none focus:border-blue-600 focus:ring-2 focus:ring-blue-100" /></div>
                <div class="flex gap-2 sm:col-span-2"><button type="submit" :disabled="saving" class="rounded-lg bg-blue-700 px-4 py-2.5 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Saving…' : (editingChapterId ? 'Save chapter' : 'Create chapter') }}</button><button type="button" class="rounded-lg border border-slate-200 bg-white px-4 py-2.5 text-sm font-semibold text-slate-700" @click="cancelChapterForm">Cancel</button></div>
              </form>

              <div v-if="isManager" class="pt-1"><button v-if="chapterFormSubjectId !== subject.id" type="button" class="rounded-lg border border-dashed border-slate-300 px-3 py-2 text-xs font-semibold text-slate-600 hover:border-blue-400 hover:text-blue-700" @click="openChapterForm(subject)">+ Add chapter to {{ subject.title }}</button></div>
            </div>
          </section>
        </div>
        <section v-else class="rounded-2xl border border-dashed border-slate-300 bg-white px-6 py-12 text-center">
          <h3 class="font-semibold">No subjects yet</h3><p class="mx-auto mt-2 max-w-md text-sm leading-6 text-slate-500">{{ isManager ? 'Create a subject first. Add chapters under the subject, then place lessons inside each chapter.' : 'The company has not published a curriculum for this course yet.' }}</p>
          <button v-if="isManager" type="button" class="mt-5 rounded-lg bg-blue-700 px-4 py-2.5 text-sm font-semibold text-white" @click="subjectFormOpen = true">Create first subject</button>
        </section>
      </template>
    </main>

    <div v-if="playbackLesson" class="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/70 p-3 sm:p-6" @click.self="playbackLesson = null; playback = null">
      <section class="max-h-[92vh] w-full max-w-4xl overflow-auto rounded-2xl bg-white p-4 shadow-2xl sm:p-6" role="dialog" aria-modal="true" :aria-label="playbackLesson.title">
        <div class="flex items-start justify-between gap-4"><div><p class="text-xs font-semibold uppercase tracking-wider text-blue-700">Lesson</p><h2 class="mt-1 text-xl font-semibold">{{ playbackLesson.title }}</h2></div><button type="button" class="grid h-9 w-9 place-items-center rounded-lg border border-slate-200 text-slate-500" aria-label="Close lesson" @click="playbackLesson = null; playback = null">×</button></div>
        <div v-if="playbackBusy" class="grid min-h-40 place-items-center text-sm text-slate-500"><span class="animate-pulse">Loading lesson…</span></div>
        <div v-else-if="playback?.content" class="prose prose-sm mt-5 max-w-none whitespace-pre-wrap leading-7 text-slate-800">{{ playback.content }}</div>
        <div v-else-if="playback?.playbackUrl" class="mt-5 aspect-video overflow-hidden rounded-xl bg-black"><iframe :src="playback.playbackUrl" class="h-full w-full" allow="accelerometer; gyroscope; autoplay; encrypted-media; picture-in-picture" allowfullscreen title="Lesson video" /></div>
        <img v-else-if="playback?.downloadUrl && playback.contentType?.startsWith('image/')" :src="protectedDownloadUrl(playback.downloadUrl)" :alt="playbackLesson.title" class="mt-5 max-h-[70vh] w-full rounded-xl border border-slate-200 bg-slate-50 object-contain" />
        <a v-else-if="playback?.downloadUrl" :href="protectedDownloadUrl(playback.downloadUrl)" target="_blank" rel="noopener noreferrer" class="mt-5 inline-flex rounded-lg bg-blue-700 px-4 py-2.5 text-sm font-semibold text-white">Open PDF / file</a>
        <p v-else-if="!playbackBusy" class="mt-5 rounded-xl bg-slate-50 p-4 text-sm text-slate-600">This lesson is not ready or your account does not have access.</p>
      </section>
    </div>
  </div>
</template>

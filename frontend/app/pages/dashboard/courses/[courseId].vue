<script setup lang="ts">
import type { ContentItem, Course, PlaybackInfo, TopicAsset } from '~/types/api'
import { apiErrorMessage } from '~/utils/api-error'

definePageMeta({ middleware: 'auth' })

type ChapterPartOutline = ContentItem & { lessons: ContentItem[] }
type ChapterOutline = ContentItem & { parts: ChapterPartOutline[] }
type TopicOutline = ContentItem & { lessons: ContentItem[]; chapters: ChapterOutline[]; assets: TopicAsset[] }
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
type EditableKind = 'subject' | 'topic' | 'chapter' | 'chapterPart' | 'lesson'

const route = useRoute()
const courseId = computed(() => String(route.params.courseId || ''))
const {
  currentUser,
  getCourses,
  updateCourse,
  getSubjects,
  createSubject,
  updateSubject,
  deleteSubject,
  getTopics,
  createTopic,
  updateTopic,
  deleteTopic,
  getChapters,
  createChapter,
  updateChapter,
  deleteChapter,
  getChapterParts,
  createChapterPart,
  updateChapterPart,
  deleteChapterPart,
  getChapterPartLessons,
  createChapterPartLesson,
  getLessons,
  updateLesson,
  deleteLesson,
  getTopicAssets,
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
const selectedChapterId = ref('')
const selectedChapterPartId = ref('')
const selectedAssetId = ref('')
const selectedAssetTopicId = ref('')
const pendingVideoAssetId = ref('')
const errorMessage = ref('')
const successMessage = ref('')
const subjectTitle = ref('')
const subjectSlug = ref('')
const subjectSlugTouched = ref(false)
const topicTitle = ref('')
const topicSlug = ref('')
const topicSlugTouched = ref(false)
const chapterTitle = ref('')
const chapterSlug = ref('')
const chapterSlugTouched = ref(false)
const chapterPartTitle = ref('')
const chapterPartSlug = ref('')
const chapterPartSlugTouched = ref(false)
const lessonTitle = ref('')
const lessonSlug = ref('')
const lessonSlugTouched = ref(false)
const uploadChapterPartByTopic = ref<Record<string, string>>({})
const uploadMode = ref<'storage' | 'stream' | ''>('')
const uploadTargetTopicId = ref('')
const lessonType = ref<'text' | 'video' | 'file'>('text')
const lessonContent = ref('')
const lessonThumbnailUrl = ref('')
const lessonPreview = ref(false)
const lessonDuration = ref(0)
const playback = ref<PlaybackInfo | null>(null)
const playbackLesson = ref<ContentItem | null>(null)
const busyLessonId = ref('')
const editTarget = ref<{ kind: EditableKind; item: ContentItem } | null>(null)
const editTitle = ref('')
const editSlug = ref('')
const editDescription = ref('')
const editSortOrder = ref(0)
const editLessonType = ref<'text' | 'video' | 'file'>('text')
const editLessonContent = ref('')
const editMediaAssetId = ref('')
const editThumbnailUrl = ref('')
const editDuration = ref(0)
const editPreview = ref(false)

const canManageContent = computed(() => ['owner', 'admin', 'instructor'].includes(currentUser.value?.role ?? ''))

watch(subjectTitle, (value) => { if (!subjectSlugTouched.value) subjectSlug.value = slugify(value) })
watch(topicTitle, (value) => { if (!topicSlugTouched.value) topicSlug.value = slugify(value) })
watch(chapterTitle, (value) => { if (!chapterSlugTouched.value) chapterSlug.value = slugify(value) })
watch(chapterPartTitle, (value) => { if (!chapterPartSlugTouched.value) chapterPartSlug.value = slugify(value) })
watch(lessonTitle, (value) => { if (!lessonSlugTouched.value) lessonSlug.value = slugify(value) })

function slugify(value: string) {
  const trimmed = value.trim()
  if (!trimmed) return ''
  const ascii = trimmed.toLowerCase().normalize('NFKD')
    .replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 80).replace(/-+$/g, '')
  if (ascii.length >= 2) return ascii
  // Non-Latin titles (e.g. Bengali) still need a valid ASCII URL slug.
  const encoded = Array.from(trimmed, (character) => character.codePointAt(0)!.toString(16)).join('-')
  const fallback = ('u-' + encoded).slice(0, 80).replace(/-+$/g, '')
  return fallback.length >= 2 ? fallback : 'unit'
}

function uploadOptions(topic: TopicOutline) {
  return topic.chapters.flatMap((chapter) => chapter.parts.map((part) => ({ chapter, part })))
}
function findTopic(topicId: string) {
  return outline.value.flatMap((subject) => subject.topics).find((topic) => topic.id === topicId) ?? null
}
function getUploadContext(topicId: string) {
  const topic = findTopic(topicId)
  if (!topic) return null
  const options = uploadOptions(topic)
  const savedId = uploadChapterPartByTopic.value[topicId]
  const option = options.find((item) => item.part.id === savedId)
    ?? options.find((item) => item.part.id === selectedChapterPartId.value)
    ?? options[0]
  return option ? { chapterId: option.chapter.id, chapterPartId: option.part.id } : null
}
function onUploadContextChange(topicId: string, event: Event) {
  const target = event.target as HTMLSelectElement
  if (target.value) uploadChapterPartByTopic.value[topicId] = target.value
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
function bytesLabel(bytes: number) {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

async function copyUrl(value: string) {
  if (!import.meta.client) return
  try {
    await navigator.clipboard.writeText(value)
    successMessage.value = 'Public asset URL copied.'
    errorMessage.value = ''
  } catch {
    errorMessage.value = 'Clipboard access is unavailable. Copy the URL from the asset details manually.'
    successMessage.value = ''
  }
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
        topics: await Promise.all(topics.map(async (topic) => {
          const [topicLessons, chapters, assets] = await Promise.all([
            getLessons(topic.id),
            getChapters(topic.id),
            canManageContent.value ? getTopicAssets(topic.id) : Promise.resolve([] as TopicAsset[]),
          ])
          return {
            ...topic,
            lessons: topicLessons.filter((lesson) => !lesson.chapterPartId),
            assets,
            chapters: await Promise.all(chapters.map(async (chapter) => {
              const parts = await getChapterParts(chapter.id)
              return {
                ...chapter,
                parts: await Promise.all(parts.map(async (part) => ({
                  ...part,
                  lessons: await getChapterPartLessons(part.id),
                }))),
              }
            })),
          }
        })),
      }
    }))
    for (const subject of outline.value) {
      for (const topic of subject.topics) {
        if (!uploadChapterPartByTopic.value[topic.id]) {
          const options = uploadOptions(topic)
          const preferred = options.find((item) => item.part.id === selectedChapterPartId.value) ?? options[0]
          if (preferred) uploadChapterPartByTopic.value[topic.id] = preferred.part.id
        }
      }
    }
    if (!outline.value.some((subject) => subject.id === selectedSubjectId.value)) {
      selectedSubjectId.value = outline.value[0]?.id ?? ''
    }
    const allTopics = outline.value.flatMap((subject) => subject.topics)
    if (!allTopics.some((topic) => topic.id === selectedTopicId.value)) {
      selectedTopicId.value = allTopics[0]?.id ?? ''
    }
    const allChapters = allTopics.flatMap((topic) => topic.chapters)
    if (!allChapters.some((chapter) => chapter.id === selectedChapterId.value)) {
      selectedChapterId.value = allChapters[0]?.id ?? ''
    }
    const allChapterParts = allChapters.flatMap((chapter) => chapter.parts)
    if (!allChapterParts.some((part) => part.id === selectedChapterPartId.value)) {
      selectedChapterPartId.value = allChapterParts[0]?.id ?? ''
    }
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
    subjectSlugTouched.value = false
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
    topicSlugTouched.value = false
    await loadOutline()
    successMessage.value = 'Part added to subject.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Part could not be created.')
  } finally { saving.value = false }
}
async function saveChapter(topicId: string) {
  resetMessages()
  saving.value = true
  try {
    const chapter = await createChapter(topicId, { title: chapterTitle.value.trim(), slug: chapterSlug.value.trim().toLowerCase() })
    selectedChapterId.value = chapter.id
    chapterTitle.value = ''
    chapterSlug.value = ''
    chapterSlugTouched.value = false
    await loadOutline()
    successMessage.value = 'Chapter added to this part.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Chapter could not be created.')
  } finally { saving.value = false }
}
async function saveChapterPart(chapterId: string) {
  resetMessages()
  saving.value = true
  try {
    const part = await createChapterPart(chapterId, { title: chapterPartTitle.value.trim(), slug: chapterPartSlug.value.trim().toLowerCase() })
    selectedChapterPartId.value = part.id
    const parentTopic = outline.value.flatMap((subject) => subject.topics).find((topic) => topic.chapters.some((chapter) => chapter.id === chapterId))
    if (parentTopic) uploadChapterPartByTopic.value[parentTopic.id] = part.id
    chapterPartTitle.value = ''
    chapterPartSlug.value = ''
    chapterPartSlugTouched.value = false
    await loadOutline()
    successMessage.value = 'Chapter part added.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Chapter part could not be created.')
  } finally { saving.value = false }
}
function topicForChapterPart(partId: string) {
  for (const subject of outline.value) {
    for (const topic of subject.topics) {
      for (const chapter of topic.chapters) {
        if (chapter.parts.some((part) => part.id === partId)) return { topic, chapter }
      }
    }
  }
  return null
}
async function saveLesson(partId: string) {
  resetMessages()
  const context = topicForChapterPart(partId)
  if (!context) {
    errorMessage.value = 'Choose a chapter part before adding a lesson.'
    return
  }
  if (lessonType.value !== 'text' && selectedAssetTopicId.value !== context.topic.id) {
    errorMessage.value = 'Select a ready video, PDF or image from the same course part as this lesson.'
    return
  }
if (lessonType.value !== 'text' && !selectedAssetId.value) {
    errorMessage.value = 'Choose an uploaded media asset before saving a video or file lesson.'
    return
  }
  if (lessonType.value !== 'text') {
    const selectedAsset = context.topic.assets.find((asset) => asset.id === selectedAssetId.value)
    if (selectedAsset?.chapterPartId && selectedAsset.chapterPartId !== partId) {
      errorMessage.value = 'This asset belongs to another chapter part. Upload or select an asset from the chapter part chosen for this lesson.'
      return
    }
  }
  saving.value = true
  try {
    const payload: {
      title: string; slug: string; lessonType: string; content?: string; mediaAssetId?: string
      durationSeconds?: number; isPreview?: boolean; thumbnailUrl?: string
    } = {
      title: lessonTitle.value.trim(),
      slug: lessonSlug.value.trim().toLowerCase(),
      lessonType: lessonType.value,
      durationSeconds: Number(lessonDuration.value || 0),
      isPreview: lessonPreview.value,
      thumbnailUrl: lessonThumbnailUrl.value.trim() || undefined,
    }
    if (lessonType.value === 'text') payload.content = lessonContent.value.trim()
    else payload.mediaAssetId = selectedAssetId.value
    await createChapterPartLesson(partId, payload)
    lessonTitle.value = ''
    lessonSlug.value = ''
    lessonSlugTouched.value = false
    lessonContent.value = ''
    lessonThumbnailUrl.value = ''
    lessonDuration.value = 0
    lessonPreview.value = false
    selectedAssetId.value = ''
    selectedAssetTopicId.value = ''
    pendingVideoAssetId.value = ''
    await loadOutline()
    successMessage.value = 'Lesson added to chapter part.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Lesson could not be created. Video/file lessons need a ready media asset first.')
  } finally { saving.value = false }
}
async function selectAsset(topicId: string, asset: TopicAsset) {
  resetMessages()
  selectedAssetId.value = asset.id
  selectedAssetTopicId.value = topicId
  if (asset.assetType === 'video' && asset.status !== 'ready') pendingVideoAssetId.value = asset.id
  else pendingVideoAssetId.value = ''
  successMessage.value = asset.title + ' selected for the next lesson.'
}
function useAssetAsLessonThumbnail(asset: TopicAsset) {
  if (!asset.publicUrl) {
    errorMessage.value = 'A public CDN URL is not configured for this asset. Add an HTTPS image URL or configure BUNNY_CDN_HOSTNAME.'
    return
  }
  lessonThumbnailUrl.value = asset.publicUrl
  successMessage.value = 'Image selected as the next lesson thumbnail.'
  errorMessage.value = ''
}
async function useAssetAsCourseThumbnail(asset: TopicAsset) {
  if (!asset.publicUrl) {
    errorMessage.value = 'A public CDN URL is not configured for this asset. Add an HTTPS image URL or configure BUNNY_CDN_HOSTNAME.'
    return
  }
  if (!course.value) return
  resetMessages()
  saving.value = true
  try {
    const updated = await updateCourse(course.value.id, {
      title: course.value.title,
      slug: course.value.slug,
      description: course.value.description || undefined,
      priceInPaise: course.value.priceInPaise,
      thumbnailUrl: asset.publicUrl,
    })
    course.value = updated
    successMessage.value = 'Course thumbnail updated.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Course thumbnail could not be updated.')
  } finally { saving.value = false }
}
async function uploadStorageFile(topicId: string, event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  const context = getUploadContext(topicId)
  if (!context) {
    errorMessage.value = 'Create a chapter and chapter part first, then choose the destination chapter part before uploading media.'
    return
  }
  resetMessages()
  uploading.value = true
  uploadMode.value = 'storage'
  uploadTargetTopicId.value = topicId
  uploadProgress.value = 0
  try {
    const result = await uploadAsset(topicId, file, context, (percent) => { uploadProgress.value = percent })
    const assetId = String(result.assetId || '')
    if (!assetId) throw new Error('The upload finished without an asset ID.')
    selectedAssetId.value = assetId
    selectedAssetTopicId.value = topicId
    pendingVideoAssetId.value = ''
    await loadOutline()
    successMessage.value = file.type === 'application/pdf'
      ? 'PDF saved to Bunny Storage and indexed in this chapter part. Select it for a lesson.'
      : 'Image saved to Bunny Storage and indexed in this chapter part.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'File upload failed. Confirm Bunny Storage settings in backend/.env and retry.')
  } finally {
    uploading.value = false
    uploadMode.value = ''
    uploadTargetTopicId.value = ''
  }
}
async function uploadVideo(topicId: string, event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  const context = getUploadContext(topicId)
  if (!context) {
    errorMessage.value = 'Create a chapter and chapter part first, then choose the destination chapter part before uploading a video.'
    return
  }
  resetMessages()
  uploading.value = true
  uploadMode.value = 'stream'
  uploadTargetTopicId.value = topicId
  uploadProgress.value = 0
  try {
    const raw = await createVideoTicket(topicId, file.name.replace(/\.[^.]+$/, ''), context)
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
    await loadOutline()
    successMessage.value = 'Video uploaded. Wait until status is ready before selecting it for a lesson.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Video upload failed. Check Bunny Stream configuration and upload access.')
  } finally {
    uploading.value = false
    uploadMode.value = ''
    uploadTargetTopicId.value = ''
  }
}
async function refreshVideo(assetId: string) {
  resetMessages()
  try {
    const result = await refreshAssetStatus(assetId)
    const status = String(result.status || 'unknown')
    await loadOutline()
    successMessage.value = 'Video status: ' + status + '. Choose the asset when it is ready.'
    if (status === 'ready') {
      selectedAssetId.value = assetId
      pendingVideoAssetId.value = ''
    }
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'Could not refresh video status. Bunny Stream may not be configured.')
  }
}
function beginEdit(kind: EditableKind, item: ContentItem) {
  editTarget.value = { kind, item }
  editTitle.value = item.title
  editSlug.value = item.slug
  editDescription.value = item.description ?? ''
  editSortOrder.value = Number(item.sortOrder ?? 0)
  editLessonType.value = (item.lessonType || 'text') as 'text' | 'video' | 'file'
  editLessonContent.value = item.content ?? ''
  editMediaAssetId.value = item.mediaAssetId ?? ''
  editThumbnailUrl.value = item.thumbnailUrl ?? ''
  editDuration.value = Number(item.durationSeconds ?? 0)
  editPreview.value = Boolean(item.isPreview)
  resetMessages()
}
async function saveEditedContent() {
  const target = editTarget.value
  if (!target) return
  resetMessages()
  saving.value = true
  try {
    const basic = {
      title: editTitle.value.trim(),
      slug: editSlug.value.trim().toLowerCase(),
      description: editDescription.value.trim() || undefined,
      sortOrder: Number(editSortOrder.value || 0),
    }
    if (target.kind === 'subject') await updateSubject(target.item.id, basic)
    else if (target.kind === 'topic') await updateTopic(target.item.id, basic)
    else if (target.kind === 'chapter') await updateChapter(target.item.id, basic)
    else if (target.kind === 'chapterPart') await updateChapterPart(target.item.id, basic)
    else {
      await updateLesson(target.item.id, {
        title: basic.title,
        slug: basic.slug,
        lessonType: editLessonType.value,
        content: editLessonType.value === 'text' ? editLessonContent.value.trim() : undefined,
        mediaAssetId: editLessonType.value === 'text' ? undefined : editMediaAssetId.value.trim(),
        durationSeconds: Number(editDuration.value || 0),
        isPreview: editPreview.value,
        sortOrder: basic.sortOrder,
        thumbnailUrl: editThumbnailUrl.value.trim() || undefined,
        chapterId: target.item.chapterId || undefined,
        chapterPartId: target.item.chapterPartId || undefined,
      })
    }
    editTarget.value = null
    await loadOutline()
    successMessage.value = target.kind === 'chapterPart'
      ? 'Chapter part updated.'
      : target.kind.charAt(0).toUpperCase() + target.kind.slice(1) + ' updated.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'The changes could not be saved.')
  } finally {
    saving.value = false
  }
}
async function archiveContent(kind: EditableKind, item: ContentItem) {
  if (!import.meta.client || !window.confirm('Archive "' + item.title + '"? Child content will also be archived when applicable.')) return
  resetMessages()
  saving.value = true
  try {
    if (kind === 'subject') await deleteSubject(item.id)
    else if (kind === 'topic') await deleteTopic(item.id)
    else if (kind === 'chapter') await deleteChapter(item.id)
    else if (kind === 'chapterPart') await deleteChapterPart(item.id)
    else await deleteLesson(item.id)
    await loadOutline()
    successMessage.value = kind === 'chapterPart' ? 'Chapter part archived.' : kind.charAt(0).toUpperCase() + kind.slice(1) + ' archived.'
  } catch (error) {
    errorMessage.value = apiErrorMessage(error, 'This item could not be archived.')
  } finally {
    saving.value = false
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
    <header class="sticky top-0 z-30 border-b border-line bg-white/95 backdrop-blur-xl">
      <div class="mx-auto flex h-[72px] max-w-7xl items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
        <NuxtLink to="/dashboard" class="inline-flex shrink-0 items-center gap-3 font-semibold"><span class="grid h-10 w-10 place-items-center rounded-2xl bg-brand text-lg font-black text-white">N</span>newage<span class="text-brand">.</span></NuxtLink>
        <nav class="flex items-center gap-2 overflow-x-auto" aria-label="Course tools">
          <NuxtLink to="/dashboard" class="shrink-0 rounded-xl border border-line px-3 py-2.5 text-xs font-semibold text-muted hover:bg-canvas sm:px-4 sm:text-sm">← All courses</NuxtLink>
          <NuxtLink to="/dashboard/site" class="shrink-0 rounded-xl border border-line px-3 py-2.5 text-xs font-semibold text-brand hover:bg-brand-soft sm:px-4 sm:text-sm">Public site & theme</NuxtLink>
          <NuxtLink to="/dashboard/settings" class="hidden shrink-0 rounded-xl border border-line px-4 py-2.5 text-sm font-semibold text-muted hover:bg-canvas sm:inline-flex">Payments</NuxtLink>
        </nav>
      </div>
    </header>

    <main class="mx-auto max-w-7xl px-4 py-7 sm:px-6 sm:py-9 lg:px-8">
      <div class="overflow-hidden rounded-3xl border border-line bg-white">
        <div class="grid lg:grid-cols-[220px_minmax(0,1fr)]">
          <div class="relative min-h-36 overflow-hidden bg-gradient-to-br from-[#dce7ff] via-[#edf0ff] to-[#d9f8eb] lg:min-h-full">
            <img v-if="course?.thumbnailUrl" :src="course.thumbnailUrl" :alt="course.title + ' thumbnail'" class="absolute inset-0 h-full w-full object-cover" />
            <div v-if="course?.thumbnailUrl" class="absolute inset-0 bg-gradient-to-t from-black/40 to-transparent" />
            <div class="absolute -right-14 -top-16 h-48 w-48 rounded-full border-[24px] border-white/35" />
            <div class="absolute bottom-4 left-4 rounded-xl border border-white/70 bg-white/80 px-3 py-2 text-[10px] font-bold uppercase tracking-[0.16em] text-ink">Course cover</div>
          </div>
          <div class="min-w-0 p-5 sm:p-7">
            <div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-start">
              <div class="min-w-0"><p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Course management</p><h1 class="mt-2 break-words text-2xl font-semibold tracking-tight sm:text-3xl">{{ course?.title || 'Loading course…' }}</h1><p class="mt-2 max-w-3xl text-sm leading-6 text-muted">{{ course?.description || 'Build your full curriculum, including subjects, parts, chapters, videos, PDFs and other assets.' }}</p></div>
              <span v-if="course" class="w-fit shrink-0 rounded-full px-3 py-1.5 text-xs font-semibold capitalize" :class="course.status === 'published' ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'">{{ course.status }}</span>
            </div>
            <div class="mt-5 flex flex-wrap gap-2 text-xs">
              <span class="rounded-lg bg-canvas px-3 py-2 font-semibold text-muted">{{ outline.length }} subjects</span>
              <span class="rounded-lg bg-canvas px-3 py-2 font-semibold text-muted">{{ outline.reduce((count, item) => count + item.topics.length, 0) }} parts</span>
              <span class="rounded-lg bg-canvas px-3 py-2 font-semibold text-muted">{{ outline.reduce((count, item) => count + item.topics.reduce((n, part) => n + part.chapters.length, 0), 0) }} chapters</span>
              <span class="rounded-lg bg-canvas px-3 py-2 font-semibold text-muted">{{ outline.reduce((count, item) => count + item.topics.reduce((n, part) => n + part.chapters.reduce((m, chapter) => m + chapter.parts.length, 0), 0), 0) }} chapter parts</span>
            </div>
          </div>
        </div>
      </div>

      <div v-if="errorMessage" role="alert" class="mt-5 flex items-start gap-3 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm leading-6 text-red-700"><span class="font-bold">Error</span><span class="flex-1">{{ errorMessage }}</span><button type="button" class="font-semibold underline" @click="loadOutline">Retry</button></div>
      <div v-if="successMessage" role="status" class="mt-4 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm leading-6 text-emerald-800">{{ successMessage }}</div>

      <section class="mt-7 grid gap-6 xl:grid-cols-[380px_minmax(0,1fr)]">
        <aside v-if="canManageContent" class="space-y-4">
          <div class="mb-2"><p class="text-xs font-bold uppercase tracking-[0.18em] text-brand">Curriculum builder</p><h2 class="mt-1 text-xl font-semibold">Build the learning path</h2><p class="mt-1 text-sm leading-6 text-muted">Create each level from top to bottom. Then upload videos, thumbnails, PDFs and other files from the asset library.</p></div>

          <section class="rounded-2xl border border-line bg-white p-5">
            <div class="flex items-center gap-3"><span class="grid h-9 w-9 place-items-center rounded-xl bg-brand-soft text-sm font-bold text-brand">01</span><div><h3 class="font-semibold">Subject</h3><p class="text-xs text-muted">Main programme section</p></div></div>
            <form class="mt-4 space-y-3" @submit.prevent="saveSubject">
              <input v-model="subjectTitle" required minlength="2" maxlength="140" placeholder="e.g. Mathematics" aria-label="Subject title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="subjectSlug" @input="subjectSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="mathematics" aria-label="Subject slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <button :disabled="saving || !course" class="w-full rounded-xl bg-brand px-4 py-3 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Saving…' : '+ Add subject' }}</button>
            </form>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5">
            <div class="flex items-center gap-3"><span class="grid h-9 w-9 place-items-center rounded-xl bg-violet-100 text-sm font-bold text-violet-800">02</span><div><h3 class="font-semibold">Part</h3><p class="text-xs text-muted">Part inside a subject</p></div></div>
            <form class="mt-4 space-y-3" @submit.prevent="saveTopic(selectedSubjectId)">
              <select v-model="selectedSubjectId" required aria-label="Select subject for part" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand">
                <option value="" disabled>Select a subject</option><option v-for="subject in outline" :key="subject.id" :value="subject.id">{{ subject.title }}</option>
              </select>
              <input v-model="topicTitle" required minlength="2" maxlength="140" placeholder="e.g. Algebra — Part 1" aria-label="Part title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="topicSlug" @input="topicSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="algebra-part-1" aria-label="Part slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <button :disabled="saving || !selectedSubjectId" class="w-full rounded-xl border border-violet-200 bg-violet-50 px-4 py-3 text-sm font-semibold text-violet-800 disabled:opacity-50">{{ saving ? 'Saving…' : '+ Add part' }}</button>
            </form>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5">
            <div class="flex items-center gap-3"><span class="grid h-9 w-9 place-items-center rounded-xl bg-amber-100 text-sm font-bold text-amber-800">03</span><div><h3 class="font-semibold">Chapter</h3><p class="text-xs text-muted">Chapter inside a part</p></div></div>
            <form class="mt-4 space-y-3" @submit.prevent="saveChapter(selectedTopicId)">
              <select v-model="selectedTopicId" required aria-label="Select part for chapter" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand">
                <option value="" disabled>Select a part</option><option v-for="subject in outline" :key="subject.id + '-heading'" disabled>— {{ subject.title }} —</option><template v-for="subject in outline" :key="subject.id + '-part-options'"><option v-for="part in subject.topics" :key="part.id" :value="part.id">{{ subject.title }} / {{ part.title }}</option></template>
              </select>
              <input v-model="chapterTitle" required minlength="2" maxlength="140" placeholder="e.g. Linear equations" aria-label="Chapter title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="chapterSlug" @input="chapterSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="linear-equations" aria-label="Chapter slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <button :disabled="saving || !selectedTopicId" class="w-full rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm font-semibold text-amber-900 disabled:opacity-50">{{ saving ? 'Saving…' : '+ Add chapter' }}</button>
            </form>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5">
            <div class="flex items-center gap-3"><span class="grid h-9 w-9 place-items-center rounded-xl bg-sky-100 text-sm font-bold text-sky-800">04</span><div><h3 class="font-semibold">Chapter part</h3><p class="text-xs text-muted">Section inside a chapter</p></div></div>
            <form class="mt-4 space-y-3" @submit.prevent="saveChapterPart(selectedChapterId)">
              <select v-model="selectedChapterId" required aria-label="Select chapter for chapter part" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand">
                <option value="" disabled>Select a chapter</option><template v-for="subject in outline" :key="subject.id + '-chapter-head'"><option v-for="topic in subject.topics" :key="topic.id + '-part-heading'" disabled>— {{ subject.title }} / {{ topic.title }} —</option><template v-for="topic in subject.topics" :key="topic.id + '-chapter-options'"><option v-for="chapter in topic.chapters" :key="chapter.id" :value="chapter.id">{{ topic.title }} / {{ chapter.title }}</option></template></template>
              </select>
              <input v-model="chapterPartTitle" required minlength="2" maxlength="140" placeholder="e.g. Worked examples" aria-label="Chapter part title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="chapterPartSlug" @input="chapterPartSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="worked-examples" aria-label="Chapter part slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <button :disabled="saving || !selectedChapterId" class="w-full rounded-xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm font-semibold text-sky-900 disabled:opacity-50">{{ saving ? 'Saving…' : '+ Add chapter part' }}</button>
            </form>
          </section>

          <section class="rounded-2xl border border-line bg-white p-5">
            <div class="flex items-center gap-3"><span class="grid h-9 w-9 place-items-center rounded-xl bg-emerald-100 text-sm font-bold text-emerald-800">05</span><div><h3 class="font-semibold">Lesson / video / asset</h3><p class="text-xs text-muted">Content inside a chapter part</p></div></div>
            <form class="mt-4 space-y-3" @submit.prevent="saveLesson(selectedChapterPartId)">
              <select v-model="selectedChapterPartId" required aria-label="Select chapter part for lesson" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand">
                <option value="" disabled>Select a chapter part</option><template v-for="subject in outline" :key="subject.id + '-lesson-chapter-head'"><template v-for="topic in subject.topics" :key="topic.id + '-lesson-topic'"><template v-for="chapter in topic.chapters" :key="chapter.id + '-lesson-options'"><option :value="''" disabled>— {{ subject.title }} / {{ topic.title }} / {{ chapter.title }} —</option><option v-for="part in chapter.parts" :key="part.id" :value="part.id">{{ topic.title }} / {{ chapter.title }} / {{ part.title }}</option></template></template></template>
              </select>
              <input v-model="lessonTitle" required minlength="2" maxlength="140" placeholder="Lesson title" aria-label="Lesson title" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <input v-model="lessonSlug" @input="lessonSlugTouched = true" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" placeholder="lesson-slug" aria-label="Lesson slug" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <select v-model="lessonType" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm outline-none focus:border-brand"><option value="text">Text lesson</option><option value="video">Video lesson</option><option value="file">PDF / image / file lesson</option></select>
              <textarea v-if="lessonType === 'text'" v-model="lessonContent" required maxlength="50000" rows="5" placeholder="Write the text lesson…" class="w-full resize-y rounded-xl border border-line px-3.5 py-3 text-sm leading-6 outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" />
              <div v-else class="rounded-xl border border-dashed border-brand/30 bg-brand-soft/40 p-3"><p class="text-xs leading-5 text-muted">Select the video or file from the asset library on the matching course part. The selected asset must belong to the same part and be ready.</p><div class="mt-2 rounded-lg bg-white px-3 py-2.5"><p class="text-[10px] font-bold uppercase tracking-wider text-muted">Selected asset ID</p><p class="mt-1 break-all font-mono text-xs">{{ selectedAssetId || 'No asset selected' }}</p></div></div>
              <div><label for="lessonThumbnail" class="mb-1.5 block text-xs font-semibold text-muted">Lesson / video thumbnail URL (HTTPS)</label><input id="lessonThumbnail" v-model="lessonThumbnailUrl" type="url" placeholder="https://…" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /><p class="mt-1 text-[11px] leading-5 text-muted">Upload an image first, then use its public CDN URL from the asset library.</p></div>
              <div><label for="lessonDuration" class="mb-1.5 block text-xs font-semibold text-muted">Duration (seconds)</label><input id="lessonDuration" v-model.number="lessonDuration" type="number" min="0" max="86400" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /></div>
              <label class="flex items-center gap-2 text-sm text-muted"><input v-model="lessonPreview" type="checkbox" class="h-4 w-4 rounded border-line accent-brand" /> Allow as preview without enrolment</label>
              <button :disabled="saving || !selectedChapterPartId" class="w-full rounded-xl bg-ink px-4 py-3 text-sm font-semibold text-white disabled:opacity-50">{{ saving ? 'Saving…' : '+ Add lesson / media' }}</button>
            </form>
          </section>
        </aside>

        <section class="min-w-0 rounded-2xl border border-line bg-white p-4 sm:p-6">
          <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-center"><div><p class="text-xs font-bold uppercase tracking-[0.16em] text-brand">Your curriculum</p><h2 class="mt-1 text-xl font-semibold">Course content tree</h2><p class="mt-1 text-sm leading-6 text-muted">Subject → Part → Chapter → Chapter part → Video, text lesson, PDF or image.</p></div><span class="w-fit rounded-full bg-canvas px-3 py-1.5 text-xs font-semibold text-muted">{{ outline.length }} subjects</span></div>

          <div v-if="loading" class="mt-6 space-y-3"><div v-for="n in 5" :key="n" class="h-16 animate-pulse rounded-xl bg-canvas" /></div>
          <div v-else-if="!outline.length" class="mt-6 rounded-2xl border border-dashed border-slate-300 px-5 py-12 text-center"><h3 class="font-semibold">Start with your first subject</h3><p class="mt-2 text-sm leading-6 text-muted">Use the Curriculum builder on the left, then add a part, chapter, chapter part and lessons. Upload video, thumbnail, PDF and image assets under a course part.</p></div>

          <div v-else class="mt-6 space-y-4">
            <article v-for="(subject, subjectIndex) in outline" :key="subject.id" class="overflow-hidden rounded-2xl border border-line">
              <div class="flex items-start gap-3 bg-canvas px-4 py-4">
                <span class="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-brand-soft text-xs font-bold text-brand">{{ String(subjectIndex + 1).padStart(2, '0') }}</span>
                <div class="min-w-0 flex-1"><h3 class="font-semibold">{{ subject.title }}</h3><p class="mt-1 text-xs text-muted">{{ subject.topics.length }} parts <span v-if="subject.description">· {{ subject.description }}</span></p></div>
                <div v-if="canManageContent" class="flex shrink-0 gap-1.5"><button type="button" class="rounded-lg border border-line bg-white px-2.5 py-1.5 text-xs font-semibold hover:border-brand hover:text-brand" @click="beginEdit('subject', subject)">Edit</button><button type="button" class="rounded-lg border border-red-200 bg-white px-2.5 py-1.5 text-xs font-semibold text-red-700 hover:bg-red-50" @click="archiveContent('subject', subject)">Archive</button></div>
              </div>

              <div class="space-y-4 p-3 sm:p-4">
                <div v-for="(topic, topicIndex) in subject.topics" :key="topic.id" class="overflow-hidden rounded-xl border border-violet-200 bg-white">
                  <div class="flex flex-wrap items-start gap-3 bg-violet-50/70 px-3 py-3 sm:px-4">
                    <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-violet-100 text-xs font-bold text-violet-900">{{ subjectIndex + 1 }}.{{ topicIndex + 1 }}</span>
                    <div class="min-w-0 flex-1"><h4 class="font-semibold">{{ topic.title }}</h4><p class="mt-1 text-xs text-muted">{{ topic.chapters.length }} chapters · {{ topic.assets.length }} assets</p><p v-if="topic.description" class="mt-1 text-xs leading-5 text-muted">{{ topic.description }}</p></div>
                    <div v-if="canManageContent" class="flex shrink-0 gap-1.5"><button type="button" class="rounded-lg border border-violet-200 bg-white px-2.5 py-1.5 text-xs font-semibold hover:border-brand hover:text-brand" @click="beginEdit('topic', topic)">Edit part</button><button type="button" class="rounded-lg border border-red-200 bg-white px-2.5 py-1.5 text-xs font-semibold text-red-700 hover:bg-red-50" @click="archiveContent('topic', topic)">Archive</button></div>
                  </div>

                  <div v-if="canManageContent" class="border-t border-violet-100 bg-canvas/60 p-3">
                    <div class="grid gap-3 sm:grid-cols-2">
                      <div class="sm:col-span-2">
                        <label class="mb-2 block text-xs font-semibold text-muted">Save media under chapter / chapter part</label>
                        <select :value="getUploadContext(topic.id)?.chapterPartId || ''" :disabled="uploadOptions(topic).length === 0 || uploading" class="w-full rounded-xl border border-line bg-white px-3.5 py-2.5 text-xs outline-none focus:border-brand disabled:bg-canvas disabled:text-muted" @change="onUploadContextChange(topic.id, $event)">
                          <option v-if="uploadOptions(topic).length === 0" value="">Create a chapter and chapter part first</option>
                          <option v-for="option in uploadOptions(topic)" :key="option.part.id" :value="option.part.id">{{ option.chapter.title }} / {{ option.part.title }}</option>
                        </select>
                        <p class="mt-1.5 text-[11px] leading-5 text-muted">Storage paths include company, course, subject, part, chapter and chapter part. When attached to a lesson, images/PDFs move into that lesson’s folder.</p>
                      </div>
                      <label class="block text-xs font-semibold text-muted">Upload thumbnail / image / PDF to Bunny Storage
                        <input type="file" accept="image/jpeg,image/png,image/webp,image/gif,application/pdf" :disabled="uploading || !getUploadContext(topic.id)" class="mt-2 block w-full text-xs file:mr-2 file:rounded-lg file:border-0 file:bg-white file:px-3 file:py-2 file:text-xs file:font-semibold file:text-brand" @change="uploadStorageFile(topic.id, $event)" />
                      </label>
                      <label class="block text-xs font-semibold text-muted">Upload video to Bunny Stream
                        <input type="file" accept="video/*" :disabled="uploading || !getUploadContext(topic.id)" class="mt-2 block w-full text-xs file:mr-2 file:rounded-lg file:border-0 file:bg-white file:px-3 file:py-2 file:text-xs file:font-semibold file:text-brand" @change="uploadVideo(topic.id, $event)" />
                      </label>
                    </div>
                    <div v-if="uploading && uploadTargetTopicId === topic.id" class="mt-3 rounded-xl border border-brand/15 bg-white p-3">
                      <div class="flex items-center justify-between gap-3 text-xs"><span class="font-semibold text-brand">{{ uploadMode === 'storage' ? (uploadProgress >= 100 ? 'Saving to Bunny Storage…' : 'Uploading to Bunny Storage…') : (uploadProgress >= 100 ? 'Finalizing video on Bunny Stream…' : 'Uploading to Bunny Stream…') }}</span><span class="tabular-nums text-muted">{{ uploadProgress }}%</span></div>
                      <div class="mt-2 h-2 overflow-hidden rounded-full bg-canvas"><div class="h-full rounded-full bg-brand transition-[width]" :style="{ width: uploadProgress + '%' }" /></div>
                      <p class="mt-2 text-[11px] leading-5 text-muted">The server confirms success only after the provider accepts the upload. Keep this page open until completion.</p>
                    </div>
                    <div v-if="selectedAssetId && selectedAssetTopicId === topic.id" class="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-xs text-emerald-900"><span class="min-w-0 break-all font-mono">Selected: {{ selectedAssetId }}</span><div class="flex gap-3"><button v-if="pendingVideoAssetId" type="button" class="shrink-0 font-semibold underline" @click="refreshVideo(pendingVideoAssetId)">Refresh status</button><button type="button" class="shrink-0 underline" @click="selectedAssetId = ''; pendingVideoAssetId = ''; selectedAssetTopicId = ''">Clear selection</button></div></div>

                    <div class="mt-4">
                      <div class="flex items-center justify-between gap-3"><h5 class="text-xs font-bold uppercase tracking-[0.14em] text-muted">Asset library</h5><span class="text-[10px] text-muted">Video · image · PDF/file</span></div>
                      <div v-if="topic.assets.length" class="mt-2 grid gap-2 sm:grid-cols-2">
                        <article v-for="asset in topic.assets" :key="asset.id" class="flex min-w-0 gap-3 rounded-xl border border-line bg-white p-3">
                          <img v-if="asset.assetType === 'image' && asset.publicUrl" :src="asset.publicUrl" :alt="asset.title" class="h-14 w-14 shrink-0 rounded-lg border border-line object-cover" />
                          <div v-else class="grid h-14 w-14 shrink-0 place-items-center rounded-lg bg-canvas text-[10px] font-bold uppercase text-muted">{{ asset.assetType }}</div>
                          <div class="min-w-0 flex-1"><p class="truncate text-xs font-semibold" :title="asset.title">{{ asset.title }}</p><p class="mt-1 truncate font-mono text-[10px] text-muted">{{ asset.id }}</p><div class="mt-1 flex flex-wrap items-center gap-1.5"><span class="rounded bg-canvas px-1.5 py-0.5 text-[10px] capitalize text-muted">{{ asset.status }}</span><span class="text-[10px] text-muted">{{ asset.contentType || asset.provider }}<span v-if="asset.sizeBytes"> · {{ bytesLabel(asset.sizeBytes) }}</span></span></div>
                            <div class="mt-2 flex flex-wrap gap-1.5"><button type="button" :disabled="asset.status !== 'ready'" class="rounded-md border border-line px-2 py-1 text-[10px] font-semibold disabled:opacity-40" @click="selectAsset(topic.id, asset)">Use in lesson</button><button v-if="asset.assetType === 'video' && asset.status !== 'ready'" type="button" class="rounded-md border border-amber-200 bg-amber-50 px-2 py-1 text-[10px] font-semibold text-amber-900" @click="refreshVideo(asset.id)">Refresh status</button><button v-if="asset.assetType === 'image'" type="button" class="rounded-md border border-line px-2 py-1 text-[10px] font-semibold" @click="useAssetAsLessonThumbnail(asset)">Use as lesson thumb</button><button v-if="asset.assetType === 'image'" type="button" class="rounded-md border border-line px-2 py-1 text-[10px] font-semibold" @click="useAssetAsCourseThumbnail(asset)">Set course cover</button><button v-if="asset.publicUrl" type="button" class="rounded-md border border-line px-2 py-1 text-[10px] font-semibold text-brand" @click="copyUrl(asset.publicUrl)">Copy public URL</button></div>
                          </div>
                        </article>
                      </div>
                      <div v-else class="mt-2 rounded-xl border border-dashed border-slate-300 bg-white px-4 py-5 text-center"><p class="text-xs font-semibold text-ink">No assets uploaded yet</p><p class="mt-1 text-xs leading-5 text-muted">Upload a thumbnail, image, PDF or video above. Newly uploaded assets appear here.</p></div>
                    </div>
                  </div>

                  <div class="space-y-3 p-3 sm:p-4">
                    <div v-for="(chapter, chapterIndex) in topic.chapters" :key="chapter.id" class="overflow-hidden rounded-xl border border-amber-200">
                      <div class="flex flex-wrap items-start gap-3 bg-amber-50 px-3 py-3">
                        <span class="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-amber-100 text-[11px] font-bold text-amber-900">{{ topicIndex + 1 }}.{{ chapterIndex + 1 }}</span>
                        <div class="min-w-0 flex-1"><h5 class="text-sm font-semibold">{{ chapter.title }}</h5><p class="mt-1 text-[11px] text-muted">{{ chapter.parts.length }} chapter parts</p></div>
                        <div v-if="canManageContent" class="flex gap-1.5"><button type="button" class="rounded-lg border border-amber-200 bg-white px-2.5 py-1.5 text-[11px] font-semibold hover:border-brand hover:text-brand" @click="beginEdit('chapter', chapter)">Edit</button><button type="button" class="rounded-lg border border-red-200 bg-white px-2.5 py-1.5 text-[11px] font-semibold text-red-700" @click="archiveContent('chapter', chapter)">Archive</button></div>
                      </div>
                      <div class="space-y-3 p-3">
                        <div v-for="(part, partIndex) in chapter.parts" :key="part.id" class="rounded-xl border border-sky-200 p-3">
                          <div class="flex flex-wrap items-start gap-3"><span class="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-sky-100 text-[11px] font-bold text-sky-900">{{ chapterIndex + 1 }}.{{ partIndex + 1 }}</span><div class="min-w-0 flex-1"><h6 class="text-sm font-semibold">{{ part.title }}</h6><p class="mt-1 text-[11px] text-muted">{{ part.lessons.length }} lessons / assets <span v-if="part.description">· {{ part.description }}</span></p></div><div v-if="canManageContent" class="flex gap-1.5"><button type="button" class="rounded-lg border border-line px-2.5 py-1.5 text-[11px] font-semibold hover:border-brand hover:text-brand" @click="beginEdit('chapterPart', part)">Edit part</button><button type="button" class="rounded-lg border border-red-200 px-2.5 py-1.5 text-[11px] font-semibold text-red-700" @click="archiveContent('chapterPart', part)">Archive</button></div></div>
                          <div v-if="part.lessons.length" class="mt-3 space-y-2">
                            <div v-for="lesson in part.lessons" :key="lesson.id" class="flex flex-wrap items-center gap-3 rounded-xl border border-line/70 p-3">
                              <img v-if="lesson.thumbnailUrl" :src="lesson.thumbnailUrl" :alt="lesson.title + ' thumbnail'" class="h-11 w-16 shrink-0 rounded-lg object-cover" />
                              <div v-else class="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-canvas text-xs font-bold uppercase text-muted">{{ lesson.lessonType === 'video' ? '▶' : lesson.lessonType === 'file' ? 'PDF' : 'Aa' }}</div>
                              <div class="min-w-0 flex-1"><p class="truncate text-sm font-semibold">{{ lesson.title }}</p><p class="mt-1 text-[11px] capitalize text-muted">{{ lesson.lessonType || 'lesson' }}<span v-if="lesson.isPreview"> · preview</span><span v-if="lesson.durationSeconds"> · {{ lesson.durationSeconds }} sec</span></p></div>
                              <div class="flex flex-wrap gap-1.5"><button type="button" class="rounded-lg border border-line px-2.5 py-2 text-xs font-semibold hover:bg-canvas" @click="openLesson(lesson)">Open</button><button v-if="canManageContent" type="button" class="rounded-lg border border-line px-2.5 py-2 text-xs font-semibold hover:border-brand hover:text-brand" @click="beginEdit('lesson', lesson)">Edit</button><button v-if="canManageContent" type="button" class="rounded-lg border border-red-200 px-2.5 py-2 text-xs font-semibold text-red-700 hover:bg-red-50" @click="archiveContent('lesson', lesson)">Archive</button><button v-if="currentUser?.role === 'learner'" type="button" :disabled="busyLessonId === lesson.id" class="rounded-lg bg-emerald-100 px-2.5 py-2 text-xs font-semibold text-emerald-800 disabled:opacity-50" @click="markComplete(lesson)">{{ busyLessonId === lesson.id ? 'Saving…' : 'Complete' }}</button></div>
                              <button v-if="lesson.mediaAssetId && canManageContent && lesson.lessonType === 'video'" type="button" class="rounded-lg border border-line px-2.5 py-2 text-[10px] font-semibold text-muted hover:bg-canvas" @click="refreshVideo(lesson.mediaAssetId)">Refresh status</button>
                            </div>
                          </div>
                          <p v-else class="mt-3 rounded-lg bg-canvas px-3 py-2.5 text-xs text-muted">No lessons here yet. Use the form on the left to add a video, text lesson or file.</p>
                        </div>
                        <p v-if="!chapter.parts.length" class="rounded-xl border border-dashed border-slate-300 px-4 py-5 text-center text-xs text-muted">No chapter parts yet. Add a chapter part on the left.</p>
                      </div>
                    </div>
                    <p v-if="!topic.chapters.length" class="rounded-xl border border-dashed border-slate-300 px-4 py-6 text-center text-xs leading-5 text-muted">No chapters yet. Add a chapter in this part, then create chapter parts and attach lessons.</p>
                    <div v-if="topic.lessons.length" class="mt-3 rounded-xl border border-dashed border-line p-3"><p class="text-xs font-semibold text-muted">Older lessons attached directly to this part</p><div v-for="lesson in topic.lessons" :key="lesson.id" class="mt-2 flex flex-wrap items-center gap-3 rounded-lg bg-canvas p-3"><span class="min-w-0 flex-1 truncate text-xs font-semibold">{{ lesson.title }}</span><button type="button" class="rounded-lg border border-line bg-white px-2.5 py-1.5 text-[11px] font-semibold" @click="openLesson(lesson)">Open</button><button v-if="canManageContent" type="button" class="rounded-lg border border-line bg-white px-2.5 py-1.5 text-[11px] font-semibold" @click="beginEdit('lesson', lesson)">Edit</button><button v-if="canManageContent" type="button" class="rounded-lg border border-red-200 bg-white px-2.5 py-1.5 text-[11px] font-semibold text-red-700" @click="archiveContent('lesson', lesson)">Archive</button></div></div>
                  </div>
                </div>
                <p v-if="!subject.topics.length" class="rounded-xl border border-dashed border-slate-300 px-4 py-7 text-center text-xs text-muted">No parts yet. Add a part to this subject.</p>
              </div>
            </article>
          </div>
        </section>
      </section>

      <div v-if="editTarget" class="fixed inset-0 z-50 flex items-center justify-center bg-ink/60 p-4" @click.self="editTarget = null">
        <section class="max-h-[90vh] w-full max-w-xl overflow-auto rounded-3xl bg-white p-5 shadow-2xl sm:p-7" role="dialog" aria-modal="true" aria-labelledby="editContentHeading">
          <div class="flex items-start justify-between gap-4"><div><p class="text-xs font-bold uppercase tracking-[0.2em] text-brand">Content management</p><h2 id="editContentHeading" class="mt-2 text-xl font-semibold">Edit {{ editTarget.kind === 'chapterPart' ? 'chapter part' : editTarget.kind }}</h2><p class="mt-1 text-sm text-muted">Changes are saved to the API and shown in the curriculum.</p></div><button type="button" class="grid h-9 w-9 place-items-center rounded-xl border border-line text-muted hover:bg-canvas" aria-label="Close editor" @click="editTarget = null">×</button></div>
          <form class="mt-5 space-y-4" @submit.prevent="saveEditedContent">
            <div><label for="editContentTitle" class="mb-1.5 block text-sm font-semibold">Title</label><input id="editContentTitle" v-model="editTitle" required minlength="2" maxlength="140" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
            <div><label for="editContentSlug" class="mb-1.5 block text-sm font-semibold">URL slug</label><input id="editContentSlug" v-model="editSlug" required minlength="2" maxlength="80" pattern="[a-z0-9]+(-[a-z0-9]+)*" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
            <div v-if="editTarget.kind !== 'lesson'"><label for="editContentDescription" class="mb-1.5 block text-sm font-semibold">Description</label><textarea id="editContentDescription" v-model="editDescription" rows="3" maxlength="10000" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
            <template v-if="editTarget.kind === 'lesson'">
              <div><label for="editLessonType" class="mb-1.5 block text-sm font-semibold">Lesson type</label><select id="editLessonType" v-model="editLessonType" class="w-full rounded-xl border border-line bg-white px-3.5 py-3 text-sm"><option value="text">Text</option><option value="video">Video</option><option value="file">File / PDF / image</option></select></div>
              <div v-if="editLessonType === 'text'"><label for="editLessonContent" class="mb-1.5 block text-sm font-semibold">Lesson content</label><textarea id="editLessonContent" v-model="editLessonContent" required maxlength="50000" rows="7" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm leading-6 outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /></div>
              <div v-else><label for="editLessonAsset" class="mb-1.5 block text-sm font-semibold">Ready media asset ID</label><input id="editLessonAsset" v-model="editMediaAssetId" required class="w-full rounded-xl border border-line px-3.5 py-3 font-mono text-xs outline-none focus:border-brand focus:ring-4 focus:ring-brand/10" /><p class="mt-1 text-xs text-muted">Use an asset that is ready and belongs to this topic.</p></div>
              <div><label for="editLessonThumb" class="mb-1.5 block text-sm font-semibold">Lesson thumbnail URL (HTTPS)</label><input id="editLessonThumb" v-model="editThumbnailUrl" type="url" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /></div>
              <div><label for="editLessonDuration" class="mb-1.5 block text-sm font-semibold">Duration (seconds)</label><input id="editLessonDuration" v-model.number="editDuration" type="number" min="0" max="86400" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /></div>
              <label class="flex items-center gap-2 text-sm text-muted"><input v-model="editPreview" type="checkbox" class="h-4 w-4 rounded border-line accent-brand" /> Allow preview without enrolment</label>
            </template>
            <div><label for="editContentOrder" class="mb-1.5 block text-sm font-semibold">Display order</label><input id="editContentOrder" v-model.number="editSortOrder" type="number" min="0" max="1000000" class="w-full rounded-xl border border-line px-3.5 py-3 text-sm outline-none focus:border-brand" /><p class="mt-1 text-xs text-muted">Lower numbers appear first.</p></div>
            <div class="flex gap-3 pt-2"><button type="submit" :disabled="saving" class="flex-1 rounded-xl bg-brand px-4 py-3 text-sm font-semibold text-white disabled:opacity-60">{{ saving ? 'Saving…' : 'Save changes' }}</button><button type="button" class="rounded-xl border border-line px-4 py-3 text-sm font-semibold text-muted" @click="editTarget = null">Cancel</button></div>
          </form>
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

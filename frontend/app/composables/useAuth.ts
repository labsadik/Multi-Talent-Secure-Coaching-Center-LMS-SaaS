import type {
  AuthCompany,
  AuthResponse,
  AuthUser,
  CompanyUser,
  ContentItem,
  Course,
  CourseProgress,
  CreateCourseInput,
  LearnerCourse,
  LearnerRegisterInput,
  LoginInput,
  PlaybackInfo,
  RegisterInput,
  TopicAsset,
} from '~/types/api'

interface MeResponse { user: AuthUser; company: AuthCompany }

export function useAuth() {
  const currentUser = useState<AuthUser | null>('newage.current-user', () => null)
  const currentCompany = useState<AuthCompany | null>('newage.current-company', () => null)
  const api = useRequestFetch()

  function saveSession(session: Pick<AuthResponse, 'user' | 'company'>) {
    currentUser.value = session.user
    currentCompany.value = session.company
    return session
  }

  async function login(input: LoginInput) {
    const session = await api<AuthResponse>('/api/session/login', { method: 'POST', body: input })
    return saveSession(session)
  }
  async function register(input: RegisterInput) {
    const session = await api<AuthResponse>('/api/session/register', { method: 'POST', body: input })
    return saveSession(session)
  }
  async function registerLearner(input: LearnerRegisterInput) {
    const session = await api<AuthResponse>('/api/session/register-learner', { method: 'POST', body: input })
    return saveSession(session)
  }
  async function loadCurrentUser() {
    const result = await api<MeResponse>('/api/session/me')
    currentUser.value = result.user
    currentCompany.value = result.company
    return result
  }
  function backend<T>(path: string, options: Record<string, unknown> = {}) {
    return api<T>('/api/backend/' + path.replace(/^\/+/, ''), options as Parameters<typeof api<T>>[1])
  }
  async function getCompanySite() { return backend<import('~/types/api').CompanySiteReply>('company/site') }
  async function updateCompanySite(input: import('~/types/api').SiteSettings) {
    return backend<import('~/types/api').CompanySiteReply>('company/site', { method: 'PUT', body: input })
  }
  async function getCourses() { return backend<Course[]>('courses') }
  async function createCourse(input: CreateCourseInput) { return backend<Course>('courses', { method: 'POST', body: input }) }
  async function updateCourse(courseId: string, input: CreateCourseInput) {
    return backend<Course>('courses/' + encodeURIComponent(courseId), { method: 'PATCH', body: input })
  }
  async function deleteCourse(courseId: string) {
    return backend<Course>('courses/' + encodeURIComponent(courseId), { method: 'DELETE' })
  }
  async function publishCourse(courseId: string) {
    return backend<Course>('courses/' + encodeURIComponent(courseId) + '/publish', { method: 'POST' })
  }
  async function getSubjects(courseId: string) { return backend<ContentItem[]>('courses/' + encodeURIComponent(courseId) + '/subjects') }
  async function createSubject(courseId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('courses/' + encodeURIComponent(courseId) + '/subjects', { method: 'POST', body: input })
  }
  async function updateSubject(subjectId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('subjects/' + encodeURIComponent(subjectId), { method: 'PATCH', body: input })
  }
  async function deleteSubject(subjectId: string) {
    return backend<void>('subjects/' + encodeURIComponent(subjectId), { method: 'DELETE' })
  }
  async function getTopics(subjectId: string) { return backend<ContentItem[]>('subjects/' + encodeURIComponent(subjectId) + '/topics') }
  async function createTopic(subjectId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('subjects/' + encodeURIComponent(subjectId) + '/topics', { method: 'POST', body: input })
  }
  async function updateTopic(topicId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('topics/' + encodeURIComponent(topicId), { method: 'PATCH', body: input })
  }
  async function deleteTopic(topicId: string) {
    return backend<void>('topics/' + encodeURIComponent(topicId), { method: 'DELETE' })
  }
  async function getChapters(topicId: string) { return backend<ContentItem[]>('topics/' + encodeURIComponent(topicId) + '/chapters') }
  async function createChapter(topicId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('topics/' + encodeURIComponent(topicId) + '/chapters', { method: 'POST', body: input })
  }
  async function updateChapter(chapterId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('chapters/' + encodeURIComponent(chapterId), { method: 'PATCH', body: input })
  }
  async function deleteChapter(chapterId: string) {
    return backend<void>('chapters/' + encodeURIComponent(chapterId), { method: 'DELETE' })
  }
  async function getChapterParts(chapterId: string) { return backend<ContentItem[]>('chapters/' + encodeURIComponent(chapterId) + '/parts') }
  async function createChapterPart(chapterId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('chapters/' + encodeURIComponent(chapterId) + '/parts', { method: 'POST', body: input })
  }
  async function updateChapterPart(partId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('chapter-parts/' + encodeURIComponent(partId), { method: 'PATCH', body: input })
  }
  async function deleteChapterPart(partId: string) {
    return backend<void>('chapter-parts/' + encodeURIComponent(partId), { method: 'DELETE' })
  }
  async function getChapterPartLessons(partId: string) { return backend<ContentItem[]>('chapter-parts/' + encodeURIComponent(partId) + '/lessons') }
  async function createChapterPartLesson(partId: string, input: {
    title: string; slug: string; lessonType: string; content?: string; mediaAssetId?: string
    durationSeconds?: number; isPreview?: boolean; sortOrder?: number; thumbnailUrl?: string
  }) {
    return backend<ContentItem>('chapter-parts/' + encodeURIComponent(partId) + '/lessons', { method: 'POST', body: input })
  }
  async function getLessons(topicId: string) { return backend<ContentItem[]>('topics/' + encodeURIComponent(topicId) + '/lessons') }
  async function createLesson(topicId: string, input: {
    title: string; slug: string; lessonType: string; content?: string; mediaAssetId?: string
    durationSeconds?: number; isPreview?: boolean; sortOrder?: number; thumbnailUrl?: string
  }) {
    return backend<ContentItem>('topics/' + encodeURIComponent(topicId) + '/lessons', { method: 'POST', body: input })
  }
  async function updateLesson(lessonId: string, input: {
    title: string; slug: string; lessonType: string; content?: string; mediaAssetId?: string
    durationSeconds?: number; isPreview?: boolean; sortOrder?: number; thumbnailUrl?: string
    chapterId?: string; chapterPartId?: string
  }) {
    return backend<ContentItem>('lessons/' + encodeURIComponent(lessonId), { method: 'PATCH', body: input })
  }
  async function deleteLesson(lessonId: string) {
    return backend<void>('lessons/' + encodeURIComponent(lessonId), { method: 'DELETE' })
  }
  async function getCompanyUsers() { return backend<CompanyUser[]>('company/users') }
  async function createCompanyUser(input: { name: string; email: string; password: string; role: string }) {
    return backend<CompanyUser>('company/users', { method: 'POST', body: input })
  }
  async function updateCompanyUser(userId: string, input: { role?: string; status?: string }) {
    return backend<CompanyUser>('company/users/' + encodeURIComponent(userId), { method: 'PATCH', body: input })
  }
  async function getMyCourses() { return backend<LearnerCourse[]>('me/courses') }
  async function updateLessonProgress(lessonId: string, status: 'in_progress' | 'completed', positionSeconds = 0) {
    return backend<Record<string, unknown>>('lessons/' + encodeURIComponent(lessonId) + '/progress', {
      method: 'PUT', body: { status, positionSeconds },
    })
  }
  async function getCourseProgress(courseId: string) { return backend<CourseProgress>('courses/' + encodeURIComponent(courseId) + '/my-progress') }
  async function getPlayback(lessonId: string) { return backend<PlaybackInfo>('lessons/' + encodeURIComponent(lessonId) + '/playback') }
  async function getTopicAssets(topicId: string) {
    return backend<TopicAsset[]>('topics/' + encodeURIComponent(topicId) + '/assets')
  }
  async function uploadAsset(
    topicId: string,
    file: File,
    context?: { chapterId: string; chapterPartId: string },
    onProgress?: (percent: number) => void,
  ) {
    const params = new URLSearchParams({ filename: file.name })
    if (context) {
      params.set('chapterId', context.chapterId)
      params.set('chapterPartId', context.chapterPartId)
    }
    const path = '/api/backend/topics/' + encodeURIComponent(topicId) + '/assets?' + params.toString()
    return await new Promise<Record<string, unknown>>((resolve, reject) => {
      const request = new XMLHttpRequest()
      request.open('PUT', path)
      request.withCredentials = true
      request.setRequestHeader('Content-Type', file.type || 'application/octet-stream')
      request.upload.onprogress = (event) => {
        if (event.lengthComputable && event.total > 0) {
          onProgress?.(Math.min(100, Math.round((event.loaded / event.total) * 100)))
        }
      }
      request.onerror = () => reject(new Error('The upload connection was interrupted. Check the frontend and Rust API terminals, then retry.'))
      request.onabort = () => reject(new Error('The upload was cancelled.'))
      request.onload = () => {
        let payload: unknown = {}
        try { payload = request.responseText ? JSON.parse(request.responseText) : {} } catch { payload = { message: request.responseText } }
        if (request.status >= 200 && request.status < 300) {
          resolve(payload as Record<string, unknown>)
          return
        }
        const body = payload as { error?: { message?: string }; statusMessage?: string; message?: string }
        reject(new Error(body.error?.message || body.statusMessage || body.message || ('File upload failed (HTTP ' + request.status + ').')))
      }
      request.send(file)
    })
  }
  async function createVideoTicket(
    topicId: string,
    title: string,
    context?: { chapterId: string; chapterPartId: string },
  ) {
    return backend<Record<string, unknown>>('topics/' + encodeURIComponent(topicId) + '/video-upload-ticket', {
      method: 'POST',
      body: { title, ...(context || {}) },
    })
  }
  async function refreshAssetStatus(assetId: string) {
    return backend<Record<string, unknown>>('assets/' + encodeURIComponent(assetId) + '/refresh-status', { method: 'POST' })
  }
  async function getRazorpaySettings() { return backend<{ configured: boolean; keyId?: string | null }>('company/integrations/razorpay') }
  async function configureRazorpay(input: { keyId: string; keySecret: string; webhookSecret: string }) {
    return backend<{ configured: boolean; keyId?: string | null }>('company/integrations/razorpay', { method: 'PUT', body: input })
  }
  async function createCourseOrder(courseId: string) {
    return backend<Record<string, unknown>>('courses/' + encodeURIComponent(courseId) + '/orders', { method: 'POST' })
  }
  async function verifyCoursePayment(input: { razorpayOrderId: string; razorpayPaymentId: string; razorpaySignature: string }) {
    return backend<Record<string, unknown>>('payments/verify', { method: 'POST', body: input })
  }
  async function createSubscriptionOrder(planCode: 'starter' | 'pro' | 'business') {
    return backend<Record<string, unknown>>('company/subscription/orders', { method: 'POST', body: { planCode } })
  }
  async function logout() {
    try { await api('/api/session/logout', { method: 'POST' }) }
    finally { clearSession() }
  }
  function clearSession() {
    currentUser.value = null
    currentCompany.value = null
  }
  return {
    currentUser, currentCompany, login, register, registerLearner, loadCurrentUser,
    getCompanySite, updateCompanySite,
    getCourses, createCourse, updateCourse, deleteCourse, publishCourse,
    getSubjects, createSubject, updateSubject, deleteSubject,
    getTopics, createTopic, updateTopic, deleteTopic,
    getLessons, createLesson, updateLesson, deleteLesson,
    getChapters, createChapter, updateChapter, deleteChapter,
    getChapterParts, createChapterPart, updateChapterPart, deleteChapterPart,
    getChapterPartLessons, createChapterPartLesson, getTopicAssets,
    getCompanyUsers, createCompanyUser, updateCompanyUser, getMyCourses, updateLessonProgress, getCourseProgress, getPlayback,
    uploadAsset, createVideoTicket, refreshAssetStatus, getRazorpaySettings, configureRazorpay, createCourseOrder,
    verifyCoursePayment, createSubscriptionOrder, logout, clearSession,
  }
}
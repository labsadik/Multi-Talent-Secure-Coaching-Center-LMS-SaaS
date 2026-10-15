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
  async function getCourses() { return backend<Course[]>('courses') }
  async function createCourse(input: CreateCourseInput) { return backend<Course>('courses', { method: 'POST', body: input }) }
  async function publishCourse(courseId: string) {
    return backend<Course>('courses/' + encodeURIComponent(courseId) + '/publish', { method: 'POST' })
  }
  async function getSubjects(courseId: string) { return backend<ContentItem[]>('courses/' + encodeURIComponent(courseId) + '/subjects') }
  async function createSubject(courseId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('courses/' + encodeURIComponent(courseId) + '/subjects', { method: 'POST', body: input })
  }
  async function getTopics(subjectId: string) { return backend<ContentItem[]>('subjects/' + encodeURIComponent(subjectId) + '/topics') }
  async function createTopic(subjectId: string, input: { title: string; slug: string; description?: string; sortOrder?: number }) {
    return backend<ContentItem>('subjects/' + encodeURIComponent(subjectId) + '/topics', { method: 'POST', body: input })
  }
  async function getLessons(topicId: string) { return backend<ContentItem[]>('topics/' + encodeURIComponent(topicId) + '/lessons') }
  async function createLesson(topicId: string, input: {
    title: string; slug: string; lessonType: string; content?: string; mediaAssetId?: string
    durationSeconds?: number; isPreview?: boolean; sortOrder?: number
  }) {
    return backend<ContentItem>('topics/' + encodeURIComponent(topicId) + '/lessons', { method: 'POST', body: input })
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
  async function uploadAsset(topicId: string, file: File) {
    return backend<Record<string, unknown>>('topics/' + encodeURIComponent(topicId) + '/assets', {
      method: 'PUT', headers: { 'Content-Type': file.type || 'application/octet-stream' }, body: await file.arrayBuffer(),
    })
  }
  async function createVideoTicket(topicId: string, title: string) {
    return backend<Record<string, unknown>>('topics/' + encodeURIComponent(topicId) + '/video-upload-ticket', { method: 'POST', body: { title } })
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
    getCourses, createCourse, publishCourse, getSubjects, createSubject, getTopics, createTopic, getLessons, createLesson,
    getCompanyUsers, createCompanyUser, updateCompanyUser, getMyCourses, updateLessonProgress, getCourseProgress, getPlayback,
    uploadAsset, createVideoTicket, refreshAssetStatus, getRazorpaySettings, configureRazorpay, createCourseOrder,
    verifyCoursePayment, createSubscriptionOrder, logout, clearSession,
  }
}
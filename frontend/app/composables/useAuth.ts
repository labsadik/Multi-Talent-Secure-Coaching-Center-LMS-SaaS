import type {
  AuthCompany,
  AuthResponse,
  AuthUser,
  Course,
  CreateCourseInput,
  LoginInput,
  RegisterInput,
} from '~/types/api'

interface MeResponse {
  user: AuthUser
  company: AuthCompany
}

export function useAuth() {
  const accessToken = useState<string | null>('newage.access-token', () => null)
  const currentUser = useState<AuthUser | null>('newage.current-user', () => null)
  const currentCompany = useState<AuthCompany | null>('newage.current-company', () => null)
  const config = useRuntimeConfig()
  const api = $fetch.create({
    baseURL: String(config.public.apiBase).replace(/\/$/, ''),
    headers: { Accept: 'application/json' },
  })

  function saveSession(session: AuthResponse) {
    accessToken.value = session.accessToken
    currentUser.value = session.user
    currentCompany.value = session.company
    return session
  }

  async function login(input: LoginInput) {
    const session = await api<AuthResponse>('/api/v1/auth/login', {
      method: 'POST',
      body: input,
    })
    return saveSession(session)
  }

  async function register(input: RegisterInput) {
    const session = await api<AuthResponse>('/api/v1/auth/register', {
      method: 'POST',
      body: input,
    })
    return saveSession(session)
  }

  async function loadCurrentUser() {
    if (!accessToken.value) {
      throw new Error('Your session has ended. Please sign in again.')
    }
    const result = await api<MeResponse>('/api/v1/auth/me', {
      headers: { Authorization: 'Bearer ' + accessToken.value },
    })
    currentUser.value = result.user
    currentCompany.value = result.company
    return result
  }

  async function getCourses() {
    if (!accessToken.value) {
      throw new Error('Your session has ended. Please sign in again.')
    }
    return api<Course[]>('/api/v1/courses', {
      headers: { Authorization: 'Bearer ' + accessToken.value },
    })
  }

  async function createCourse(input: CreateCourseInput) {
    if (!accessToken.value) {
      throw new Error('Your session has ended. Please sign in again.')
    }
    return api<Course>('/api/v1/courses', {
      method: 'POST',
      body: input,
      headers: { Authorization: 'Bearer ' + accessToken.value },
    })
  }

  async function publishCourse(courseId: string) {
    if (!accessToken.value) {
      throw new Error('Your session has ended. Please sign in again.')
    }
    return api<Course>('/api/v1/courses/' + encodeURIComponent(courseId) + '/publish', {
      method: 'POST',
      headers: { Authorization: 'Bearer ' + accessToken.value },
    })
  }

  function clearSession() {
    accessToken.value = null
    currentUser.value = null
    currentCompany.value = null
  }

  return {
    accessToken,
    currentUser,
    currentCompany,
    login,
    register,
    loadCurrentUser,
    getCourses,
    createCourse,
    publishCourse,
    clearSession,
  }
}
import { createError, deleteCookie, getCookie, getMethod, getRequestHeader, getRequestURL, readRawBody, setCookie, setResponseHeader, setResponseStatus, type H3Event } from 'h3'

const ACCESS_COOKIE = 'newage_access'
const REFRESH_COOKIE = 'newage_refresh'
const REFRESH_TTL_SECONDS = 30 * 24 * 60 * 60

type AuthResponse = {
  accessToken: string; refreshToken: string; tokenType: string; expiresIn: number; refreshExpiresIn?: number
  user: { id: string; name: string; email: string; role: string }
  company: { id: string; name: string; slug: string; status?: string }
}
type ApiError = { status?: number; statusCode?: number; statusMessage?: string; message?: string; data?: unknown }

function apiBase(event: H3Event) {
  const config = useRuntimeConfig(event)
  return String(config.apiInternalBase || 'http://127.0.0.1:8080').replace(/\/+$/, '')
}
function cookieOptions(event: H3Event, maxAge: number) {
  const proxyProto = getRequestHeader(event, 'x-forwarded-proto')?.split(',')[0]?.trim()
  const secure = proxyProto ? proxyProto === 'https' : getRequestURL(event).protocol === 'https:'
  return { httpOnly: true, secure, sameSite: 'lax' as const, path: '/', maxAge }
}
function setSessionCookies(event: H3Event, session: AuthResponse) {
  setCookie(event, ACCESS_COOKIE, session.accessToken, cookieOptions(event, Math.max(1, session.expiresIn)))
  setCookie(event, REFRESH_COOKIE, session.refreshToken, cookieOptions(event, Math.max(1, session.refreshExpiresIn ?? REFRESH_TTL_SECONDS)))
}
function clearSessionCookies(event: H3Event) {
  const options = cookieOptions(event, 0)
  deleteCookie(event, ACCESS_COOKIE, options)
  deleteCookie(event, REFRESH_COOKIE, options)
}
function passThroughError(error: unknown, baseURL = 'http://127.0.0.1:8080') {
  const value = error as ApiError | null
  const rawMessage = typeof value?.message === 'string' ? value.message : ''
  const cannotConnect = /fetch failed|failed to fetch|ECONNREFUSED|ECONNRESET|ENOTFOUND|ETIMEDOUT|EHOSTUNREACH/i.test(rawMessage)
  const statusCode = cannotConnect
    ? 503
    : typeof value?.statusCode === 'number'
      ? value.statusCode
      : typeof value?.status === 'number'
        ? value.status
        : 502
  const data = value?.data
  const message = data && typeof data === 'object' && 'error' in data ? (data as { error?: { message?: string } }).error?.message : undefined
  const statusMessage = cannotConnect
    ? 'Cannot connect to the Newage API at ' + baseURL + '. Start the Rust backend and confirm that /health/live returns 200, then retry.'
    : message || value?.statusMessage || value?.message || 'The API request failed'
  return createError({ statusCode, statusMessage, data })
}
export function assertSameOrigin(event: H3Event) {
  const origin = getRequestHeader(event, 'origin')
  if (origin && origin !== getRequestURL(event).origin) throw createError({ statusCode: 403, statusMessage: 'Cross-origin request rejected' })
}
export function publicSession(session: AuthResponse) {
  return { user: session.user, company: session.company, expiresIn: session.expiresIn }
}
export async function startSlackSession(event: H3Event, companySlug: string, code: string, nonce: string) {
  try {
    const session = await $fetch<AuthResponse>('/api/v1/auth/slack-login', {
      baseURL: apiBase(event),
      method: 'POST',
      body: { companySlug, code, nonce },
    })
    setSessionCookies(event, session)
    return publicSession(session)
  } catch (error) {
    throw passThroughError(error, apiBase(event))
  }
}

export async function startSession(event: H3Event, action: 'login' | 'register' | 'register-learner', body: unknown) {
  assertSameOrigin(event)
  try {
    const session = await $fetch<AuthResponse>('/api/v1/auth/' + action, { baseURL: apiBase(event), method: 'POST', body: body as Record<string, unknown> })
    setSessionCookies(event, session)
    return publicSession(session)
  } catch (error) { throw passThroughError(error, apiBase(event)) }
}
async function refreshAccessToken(event: H3Event) {
  const refreshToken = getCookie(event, REFRESH_COOKIE)
  if (!refreshToken) {
    clearSessionCookies(event)
    throw createError({ statusCode: 401, statusMessage: 'Your session has ended. Please sign in again.' })
  }
  try {
    const session = await $fetch<AuthResponse>('/api/v1/auth/refresh', { baseURL: apiBase(event), method: 'POST', body: { refreshToken } })
    setSessionCookies(event, session)
    return session.accessToken
  } catch (error) {
    const status = (error as ApiError | null)?.statusCode ?? (error as ApiError | null)?.status
    // Expired/revoked credentials should be cleared, but transient API/database errors
    // must not destroy a valid refresh session.
    if (status === 401 || status === 403) clearSessionCookies(event)
    throw passThroughError(error, apiBase(event))
  }
}
async function performBackendRequest(event: H3Event, path: string, init: RequestInit = {}) {
  let accessToken = getCookie(event, ACCESS_COOKIE)
  if (!accessToken) accessToken = await refreshAccessToken(event)
  const sendRequest = async (token: string) => {
    try {
      return await fetch(apiBase(event) + path, {
        ...init,
        headers: { Accept: 'application/json', ...(init.headers || {}), Authorization: 'Bearer ' + token },
        redirect: 'manual',
      })
    } catch (error) {
      throw passThroughError(error, apiBase(event))
    }
  }
  let response = await sendRequest(accessToken)
  if (response.status === 401 && getCookie(event, REFRESH_COOKIE)) {
    try {
      accessToken = await refreshAccessToken(event)
      response = await sendRequest(accessToken)
    } catch (error) {
      const status = (error as ApiError | null)?.statusCode ?? (error as ApiError | null)?.status
      if (status === 401 || status === 403) {
        clearSessionCookies(event)
        return response
      }
      throw error
    }
  }
  if (response.status === 401) clearSessionCookies(event)
  return response
}
export async function backendJson<T>(event: H3Event, path: string, init: RequestInit = {}): Promise<T> {
  const response = await performBackendRequest(event, path, init)
  setResponseStatus(event, response.status)
  if (response.status === 204) return null as T
  if ((response.headers.get('content-type') || '').includes('application/json')) return await response.json() as T
  return await response.text() as T
}
export async function proxyBackendRequest(event: H3Event) {
  const match = event.path.match(/^\/api\/backend\/(.+?)(?:\?.*)?$/)
  const rawPath = match?.[1]
  if (!rawPath || rawPath.includes('..') || rawPath.includes('\\\\') || !/^[A-Za-z0-9._/-]+$/.test(rawPath)) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid API path' })
  }
  const url = getRequestURL(event)
  const path = '/api/v1/' + rawPath + url.search
  const method = getMethod(event)
  const headers: Record<string, string> = {}
  const contentType = getRequestHeader(event, 'content-type')
  if (contentType) headers['Content-Type'] = contentType
  const body = method === 'GET' || method === 'HEAD' ? undefined : await readRawBody(event, false)
  const response = await performBackendRequest(event, path, { method, headers, body: body as BodyInit | undefined })
  setResponseStatus(event, response.status)
  for (const name of ['content-type', 'content-disposition', 'cache-control', 'etag']) {
    const value = response.headers.get(name)
    if (value) setResponseHeader(event, name, value)
  }
  if (method === 'HEAD' || response.status === 204 || response.status === 304) return null
  const responseType = response.headers.get('content-type') || ''
  if (responseType.includes('application/json')) return await response.json()
  if (responseType.startsWith('text/')) return await response.text()
  return Buffer.from(await response.arrayBuffer())
}
export async function logoutSession(event: H3Event) {
  assertSameOrigin(event)
  const refreshToken = getCookie(event, REFRESH_COOKIE)
  clearSessionCookies(event)
  if (refreshToken) {
    try { await $fetch('/api/v1/auth/logout', { baseURL: apiBase(event), method: 'POST', body: { refreshToken } }) } catch { /* still clear browser cookies if the API is down */ }
  }
  return { loggedOut: true }
}

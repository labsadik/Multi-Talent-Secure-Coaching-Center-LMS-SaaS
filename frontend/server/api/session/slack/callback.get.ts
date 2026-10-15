import { deleteCookie, getCookie, getQuery, getRequestHeader, getRequestURL, sendRedirect } from 'h3'
import { startSlackSession } from '../../../utils/session'

const STATE_COOKIE = 'newage_slack_state'
const COMPANY_COOKIE = 'newage_slack_company'
const COOKIE_PATH = '/api/session/slack'

function cookieOptions(event: Parameters<typeof deleteCookie>[0]) {
  const forwardedProto = getRequestHeader(event, 'x-forwarded-proto')?.split(',')[0]?.trim()
  const secure = forwardedProto ? forwardedProto === 'https' : getRequestURL(event).protocol === 'https:'
  return { httpOnly: true, secure, sameSite: 'lax' as const, path: COOKIE_PATH }
}

function loginPath(companySlug: string, error?: string) {
  const safeSlug = /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(companySlug) ? companySlug : ''
  if (!safeSlug) return '/login?slackError=' + encodeURIComponent(error || 'failed')
  return '/workspace/' + encodeURIComponent(safeSlug) + '/login' +
    (error ? '?slackError=' + encodeURIComponent(error) : '')
}

export default defineEventHandler(async (event) => {
  const query = getQuery(event)
  const code = typeof query.code === 'string' ? query.code : ''
  const returnedState = typeof query.state === 'string' ? query.state : ''
  const oauthError = typeof query.error === 'string' ? query.error : ''
  const savedState = getCookie(event, STATE_COOKIE) || ''
  const companySlug = getCookie(event, COMPANY_COOKIE) || ''
  const options = cookieOptions(event)

  deleteCookie(event, STATE_COOKIE, options)
  deleteCookie(event, COMPANY_COOKIE, options)

  if (oauthError) return sendRedirect(event, loginPath(companySlug, 'denied'), 302)
  if (!savedState || !returnedState || savedState.length !== returnedState.length
      || savedState !== returnedState || !code || code.length > 4096) {
    return sendRedirect(event, loginPath(companySlug, 'state'), 302)
  }

  try {
    await startSlackSession(event, companySlug, code, returnedState)
    return sendRedirect(event, '/dashboard', 302)
  } catch {
    // Avoid leaking OAuth codes, tokens, API internals, or upstream error details into URLs.
    return sendRedirect(event, loginPath(companySlug, 'failed'), 302)
  }
})

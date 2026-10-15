import { randomBytes } from 'node:crypto'
import { getQuery, getRequestHeader, getRequestURL, sendRedirect, setCookie } from 'h3'

const STATE_COOKIE = 'newage_slack_state'
const COMPANY_COOKIE = 'newage_slack_company'
const COOKIE_PATH = '/api/session/slack'

function isValidSlug(value: string) {
  return value.length >= 2 && value.length <= 80
    && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value)
}

function cookieOptions(event: Parameters<typeof setCookie>[0], maxAge: number) {
  const forwardedProto = getRequestHeader(event, 'x-forwarded-proto')?.split(',')[0]?.trim()
  const secure = forwardedProto ? forwardedProto === 'https' : getRequestURL(event).protocol === 'https:'
  return { httpOnly: true, secure, sameSite: 'lax' as const, path: COOKIE_PATH, maxAge }
}

export default defineEventHandler(async (event) => {
  const query = getQuery(event)
  const companySlug = typeof query.companySlug === 'string' ? query.companySlug.trim().toLowerCase() : ''
  if (!isValidSlug(companySlug)) {
    return sendRedirect(event, '/login?slackError=invalid', 302)
  }

  const config = useRuntimeConfig(event)
  const clientId = String(config.slackClientId || '')
  const redirectUri = String(config.slackRedirectUri || '')
  if (!clientId || !redirectUri) {
    return sendRedirect(event, '/workspace/' + encodeURIComponent(companySlug) + '/login?slackError=not-configured', 302)
  }

  const state = randomBytes(32).toString('hex')
  setCookie(event, STATE_COOKIE, state, cookieOptions(event, 600))
  setCookie(event, COMPANY_COOKIE, companySlug, cookieOptions(event, 600))

  const authorizationUrl = new URL('https://slack.com/openid/connect/authorize')
  authorizationUrl.searchParams.set('client_id', clientId)
  authorizationUrl.searchParams.set('scope', 'openid email profile')
  authorizationUrl.searchParams.set('response_type', 'code')
  authorizationUrl.searchParams.set('redirect_uri', redirectUri)
  authorizationUrl.searchParams.set('state', state)
  authorizationUrl.searchParams.set('nonce', state)

  return sendRedirect(event, authorizationUrl.toString(), 302)
})

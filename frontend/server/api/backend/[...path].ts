import { assertSameOrigin, proxyBackendRequest } from '../../utils/session'

export default defineEventHandler(async (event) => {
  const method = getMethod(event)
  if (!['GET', 'HEAD', 'OPTIONS'].includes(method)) assertSameOrigin(event)
  return proxyBackendRequest(event)
})

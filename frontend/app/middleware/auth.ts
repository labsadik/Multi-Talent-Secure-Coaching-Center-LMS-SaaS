export default defineNuxtRouteMiddleware(async (to) => {
  const { currentUser, loadCurrentUser, clearSession } = useAuth()

  if (!currentUser.value) {
    try {
      await loadCurrentUser()
    } catch (error) {
      const failure = error as { status?: number; statusCode?: number } | null
      const status = failure?.statusCode ?? failure?.status

      // Only an explicit authentication failure should end the session. A 502/503
      // from the API must not masquerade as a signed-out user.
      if (status === 401 || status === 403) {
        clearSession()
        const tenantMatch = to.path.match(/^\\/workspace\\/([a-z0-9]+(?:-[a-z0-9]+)*)\\/dashboard/)
        return navigateTo({
          path: tenantMatch?.[1]
            ? '/workspace/' + encodeURIComponent(tenantMatch[1]) + '/login'
            : '/login',
          query: { redirect: to.fullPath },
        })
      }

      throw createError({
        statusCode: status && status >= 400 && status <= 599 ? status : 503,
        statusMessage: 'Newage could not verify your session because the API is unavailable. Check that the Rust backend is running at http://127.0.0.1:8080, then retry.',
      })
    }
  }

  if (to.path === '/dashboard' && currentUser.value?.role === 'learner') {
    return navigateTo('/dashboard/learn')
  }
})

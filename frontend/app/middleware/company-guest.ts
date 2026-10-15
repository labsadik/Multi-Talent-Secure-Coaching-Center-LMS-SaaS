export default defineNuxtRouteMiddleware(async (to) => {
  const { currentUser, currentCompany, loadCurrentUser, clearSession } = useAuth()

  if (!currentUser.value || !currentCompany.value) {
    try {
      await loadCurrentUser()
    } catch (error) {
      const failure = error as { status?: number; statusCode?: number } | null
      const status = failure?.statusCode ?? failure?.status

      if (status === 401 || status === 403) {
        clearSession()
        return
      }

      throw createError({
        statusCode: status && status >= 400 && status <= 599 ? status : 503,
        statusMessage: 'Newage could not verify your current workspace. Check that the API is available, then retry.',
      })
    }
  }

  if (!currentUser.value) return

  const requestedSlug = String(to.params.companySlug || '').trim().toLowerCase()
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(requestedSlug)) return

  // A session from another tenant must never be treated as membership in this workspace.
  // Leave the page available so it can ask the user to sign out before switching.
  if (currentCompany.value?.slug?.trim().toLowerCase() !== requestedSlug) return

  const courseId = typeof to.query.courseId === 'string' ? to.query.courseId.trim() : ''
  const base = '/workspace/' + encodeURIComponent(requestedSlug) + '/dashboard'
  if (to.path === '/workspace/' + requestedSlug + '/register') {
    // An authenticated learner does not need a second account.
    return navigateTo(courseId
      ? { path: base + '/learn', query: { checkoutCourse: courseId } }
      : (currentUser.value.role === 'learner' ? base + '/learn' : base))
  }
  const requested = typeof to.query.redirect === 'string' ? to.query.redirect : ''
  if (requested.startsWith(base) && !requested.startsWith('//')) return navigateTo(requested)
  if (courseId) return navigateTo({ path: base + '/learn', query: { checkoutCourse: courseId } })
  return navigateTo(currentUser.value.role === 'learner' ? base + '/learn' : base)
})

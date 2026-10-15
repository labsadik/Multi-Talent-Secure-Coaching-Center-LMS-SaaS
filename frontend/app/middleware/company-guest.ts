export default defineNuxtRouteMiddleware(async (to) => {
  const { currentUser, currentCompany, loadCurrentUser, clearSession } = useAuth()

  if (!currentUser.value) {
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
  if (currentUser.value.role === 'learner') {
    return navigateTo(courseId
      ? { path: '/dashboard/learn', query: { checkoutCourse: courseId } }
      : '/dashboard/learn')
  }

  const requested = typeof to.query.redirect === 'string' ? to.query.redirect : '/dashboard'
  return navigateTo(requested.startsWith('/') && !requested.startsWith('//') ? requested : '/dashboard')
})

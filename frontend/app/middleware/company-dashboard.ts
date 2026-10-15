export default defineNuxtRouteMiddleware(async (to) => {
  const rawSlug = to.params.companySlug
  if (typeof rawSlug !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(rawSlug)) {
    return navigateTo('/login')
  }
  const slug = rawSlug.toLowerCase()
  const { currentUser, currentCompany, loadCurrentUser, clearSession } = useAuth()

  if (!currentUser.value || !currentCompany.value) {
    try {
      await loadCurrentUser()
    } catch (error) {
      const failure = error as { status?: number; statusCode?: number } | null
      const status = failure?.statusCode ?? failure?.status
      if (status === 401 || status === 403) {
        clearSession()
        return navigateTo({
          path: '/workspace/' + encodeURIComponent(slug) + '/login',
          query: { redirect: to.fullPath },
        })
      }
      throw createError({
        statusCode: status && status >= 400 && status <= 599 ? status : 503,
        statusMessage: 'Newage could not verify this company session. Check that the API is available, then retry.',
      })
    }
  }

  if (!currentUser.value || !currentCompany.value) {
    return navigateTo({
      path: '/workspace/' + encodeURIComponent(slug) + '/login',
      query: { redirect: to.fullPath },
    })
  }

  if (currentCompany.value.slug.trim().toLowerCase() !== slug) {
    return navigateTo({
      path: '/workspace/' + encodeURIComponent(slug) + '/login',
      query: { redirect: to.fullPath },
    })
  }

  if (to.path === '/workspace/' + slug + '/dashboard') {
    if (currentUser.value.role === 'learner') {
      const courseId = typeof to.query.checkoutCourse === 'string' ? to.query.checkoutCourse : ''
      return navigateTo(courseId
        ? { path: '/workspace/' + encodeURIComponent(slug) + '/dashboard/learn', query: { checkoutCourse: courseId } }
        : '/workspace/' + encodeURIComponent(slug) + '/dashboard/learn')
    }
  }

  if (to.path === '/workspace/' + slug + '/dashboard/learn'
      && currentUser.value.role !== 'learner') {
    return navigateTo('/workspace/' + encodeURIComponent(slug) + '/dashboard')
  }
})

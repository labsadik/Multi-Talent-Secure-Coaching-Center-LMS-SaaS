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
        const match = to.path.match(/^\/workspace\/([a-z0-9]+(?:-[a-z0-9]+)*)\/dashboard(.*)$/)
        const slug = match?.[1]
        return navigateTo({
          path: slug ? '/workspace/' + encodeURIComponent(slug) + '/login' : '/login',
          query: { redirect: to.fullPath },
        })
      }
      throw createError({
        statusCode: status && status >= 400 && status <= 599 ? status : 503,
        statusMessage: 'Newage could not verify this company session. Check that the API is available, then retry.',
      })
    }
  }

  if (!currentUser.value || !currentCompany.value) return navigateTo('/login')
  const rawSlug = to.params.companySlug
  // Legacy dashboard URLs are redirected to the authenticated tenant's slug URL.
  if (typeof rawSlug !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(rawSlug)) {
    const slug = currentCompany.value.slug.trim().toLowerCase()
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug)) return navigateTo('/login')
    const suffix = to.path.replace(/^\/dashboard/, '')
    return navigateTo({
      path: '/workspace/' + encodeURIComponent(slug) + '/dashboard' + suffix,
      query: to.query,
    })
  }

  const slug = rawSlug.toLowerCase()
  if (currentCompany.value.slug.trim().toLowerCase() !== slug) {
    return navigateTo({
      path: '/workspace/' + encodeURIComponent(slug) + '/login',
      query: { redirect: to.fullPath },
    })
  }

  const dashboardRoot = '/workspace/' + encodeURIComponent(slug) + '/dashboard'
  const role = currentUser.value.role

  if (to.path === dashboardRoot && role === 'learner') {
    const courseId = typeof to.query.checkoutCourse === 'string' ? to.query.checkoutCourse : ''
    return navigateTo(courseId
      ? { path: dashboardRoot + '/learn', query: { checkoutCourse: courseId } }
      : dashboardRoot + '/learn')
  }

  if (to.path === dashboardRoot + '/learn' && role !== 'learner') {
    return navigateTo(dashboardRoot)
  }

  // Learners can discover, purchase, and study courses, but cannot enter staff tools.
  const learnerRestrictedPaths = ['/users', '/site', '/settings'].map((path) => dashboardRoot + path)
  if (role === 'learner' && learnerRestrictedPaths.some((path) => to.path === path || to.path.startsWith(path + '/'))) {
    return navigateTo(dashboardRoot + '/learn')
  }

  // Instructors author courses and manage learners, not workspace branding or billing.
  const adminOnlyPaths = ['/site', '/settings'].map((path) => dashboardRoot + path)
  if (role === 'instructor' && adminOnlyPaths.some((path) => to.path === path || to.path.startsWith(path + '/'))) {
    return navigateTo(dashboardRoot)
  }
})

export default defineNuxtRouteMiddleware((to) => {
  const companySlug = typeof to.query.companySlug === 'string'
    ? to.query.companySlug.trim().toLowerCase()
    : ''

  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(companySlug)) {
    return navigateTo('/login')
  }

  const query: Record<string, string> = {}
  const courseId = typeof to.query.courseId === 'string' ? to.query.courseId.trim() : ''
  if (courseId) query.courseId = courseId

  return navigateTo({
    path: '/workspace/' + encodeURIComponent(companySlug) + '/register',
    query,
  })
})

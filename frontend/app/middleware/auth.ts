export default defineNuxtRouteMiddleware(async (to) => {
  const { currentUser, loadCurrentUser, clearSession } = useAuth()

  if (!currentUser.value) {
    try {
      await loadCurrentUser()
    } catch {
      clearSession()
      return navigateTo({
        path: '/login',
        query: { redirect: to.fullPath },
      })
    }
  }

  if (to.path === '/dashboard' && currentUser.value?.role === 'learner') {
    return navigateTo('/dashboard/learn')
  }
})
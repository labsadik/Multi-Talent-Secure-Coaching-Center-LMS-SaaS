export default defineNuxtRouteMiddleware(async () => {
  const { currentUser, loadCurrentUser } = useAuth()
  if (!currentUser.value) {
    try { await loadCurrentUser() } catch { return }
  }
  return navigateTo(currentUser.value?.role === 'learner' ? '/dashboard/learn' : '/dashboard')
})
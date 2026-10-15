export default defineNuxtRouteMiddleware((to) => {
  const { accessToken } = useAuth()

  if (!accessToken.value) {
    return navigateTo({
      path: '/login',
      query: { redirect: to.fullPath },
    })
  }
})
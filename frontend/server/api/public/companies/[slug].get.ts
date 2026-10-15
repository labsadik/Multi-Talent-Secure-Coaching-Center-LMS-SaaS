export default defineEventHandler(async (event) => {
  const slug = getRouterParam(event, 'slug')
  if (!slug || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug)) {
    throw createError({ statusCode: 404, statusMessage: 'Company page not found' })
  }
  const config = useRuntimeConfig(event)
  const baseURL = String(config.apiInternalBase || 'http://127.0.0.1:8080').replace(/\/+$/, '')
  return await $fetch('/api/v1/public/companies/' + encodeURIComponent(slug), { baseURL })
})

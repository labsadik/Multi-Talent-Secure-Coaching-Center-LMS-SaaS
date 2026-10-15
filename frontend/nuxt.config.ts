import tailwindcss from '@tailwindcss/vite'

export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: false },
  css: ['~/assets/css/main.css'],
  vite: {
    plugins: [tailwindcss()],
  },
  runtimeConfig: {
    // Private server-side API origin. Do not expose bearer tokens or secrets to the browser.
    apiInternalBase: 'http://127.0.0.1:8080',
  },
  typescript: {
    strict: true,
    typeCheck: true,
  },
  hooks: {
    'pages:extend'(pages) {
      // Vue Router aliases must share the same params as their original path.
      // Tenant paths are independent route records so companySlug stays available.
      const workspaceRoutes = [
        { fileSuffix: '/pages/dashboard/index.vue', path: '/workspace/:companySlug/dashboard', name: 'workspace-dashboard' },
        { fileSuffix: '/pages/dashboard/courses/[courseId].vue', path: '/workspace/:companySlug/dashboard/courses/:courseId', name: 'workspace-dashboard-courses-courseId' },
        { fileSuffix: '/pages/dashboard/learn.vue', path: '/workspace/:companySlug/dashboard/learn', name: 'workspace-dashboard-learn' },
        { fileSuffix: '/pages/dashboard/settings.vue', path: '/workspace/:companySlug/dashboard/settings', name: 'workspace-dashboard-settings' },
        { fileSuffix: '/pages/dashboard/site.vue', path: '/workspace/:companySlug/dashboard/site', name: 'workspace-dashboard-site' },
        { fileSuffix: '/pages/dashboard/users.vue', path: '/workspace/:companySlug/dashboard/users', name: 'workspace-dashboard-users' },
      ]

      for (const workspaceRoute of workspaceRoutes) {
        const original = pages.find(page =>
          typeof page.file === 'string' && page.file.replace(/\\/g, '/').endsWith(workspaceRoute.fileSuffix),
        )
        if (!original) throw new Error(`Could not register workspace route for ${workspaceRoute.fileSuffix}`)
        if (!pages.some(page => page.path === workspaceRoute.path)) {
          pages.push({ ...original, name: workspaceRoute.name, path: workspaceRoute.path, alias: undefined })
        }
      }
    },
  },
  app: {
    head: {
      title: 'Newage — Learning, organised',
      htmlAttrs: { lang: 'en' },
      meta: [
        {
          name: 'description',
          content: 'A calmer workspace to create courses, guide learners, and grow your learning business.',
        },
        { name: 'theme-color', content: '#f8f9fc' },
      ],
    },
  },
})
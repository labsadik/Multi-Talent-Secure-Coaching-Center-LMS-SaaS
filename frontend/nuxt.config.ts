import tailwindcss from '@tailwindcss/vite'

export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: false },
  css: ['~/assets/css/main.css'],
  vite: {
    plugins: [tailwindcss()],
  },
  runtimeConfig: {
    public: {
      apiBase: 'http://127.0.0.1:8080',
    },
  },
  typescript: {
    strict: true,
    typeCheck: true,
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
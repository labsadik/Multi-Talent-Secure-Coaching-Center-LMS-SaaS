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
    slackClientId: process.env.NUXT_SLACK_CLIENT_ID || '',
    slackRedirectUri: process.env.NUXT_SLACK_REDIRECT_URI || 'http://localhost:3000/api/session/slack/callback',
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
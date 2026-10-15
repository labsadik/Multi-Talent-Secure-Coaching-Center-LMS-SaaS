import { backendJson } from '../../utils/session'

interface MeResponse {
  user: { id: string; name: string; email: string; role: string }
  company: { id: string; name: string; slug: string; status?: string }
}

export default defineEventHandler((event) => backendJson<MeResponse>(event, '/api/v1/auth/me'))

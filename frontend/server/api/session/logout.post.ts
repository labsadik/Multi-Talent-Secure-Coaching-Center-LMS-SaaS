import { logoutSession } from '../../utils/session'

export default defineEventHandler((event) => logoutSession(event))

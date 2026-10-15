import { readBody } from 'h3'
import { startSession } from '../../utils/session'

export default defineEventHandler(async (event) => startSession(event, 'login', await readBody(event)))

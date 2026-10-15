interface FetchErrorShape {
  data?: {
    error?: {
      message?: string
    }
  }
  statusMessage?: string
  message?: string
}

export function apiErrorMessage(error: unknown, fallback: string) {
  const value = error as FetchErrorShape | null
  return value?.data?.error?.message || value?.statusMessage || value?.message || fallback
}
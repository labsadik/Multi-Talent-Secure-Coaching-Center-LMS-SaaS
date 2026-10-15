export type ConfirmTone = 'danger' | 'primary'

export interface ConfirmRequest {
  id: number
  title: string
  message: string
  confirmLabel: string
  cancelLabel: string
  tone: ConfirmTone
}

type ConfirmInput = Partial<Omit<ConfirmRequest, 'id'>> & Pick<ConfirmRequest, 'title' | 'message'>

const pendingResolvers = new Map<number, (confirmed: boolean) => void>()
let nextConfirmationId = 0

export function useConfirmDialog() {
  const request = useState<ConfirmRequest | null>('newage-confirm-dialog', () => null)

  function finish(id: number, confirmed: boolean) {
    const resolve = pendingResolvers.get(id)
    pendingResolvers.delete(id)
    if (request.value?.id === id) request.value = null
    resolve?.(confirmed)
  }

  function confirm(input: ConfirmInput): Promise<boolean> {
    if (!import.meta.client) return Promise.resolve(false)
    if (request.value) finish(request.value.id, false)

    const id = ++nextConfirmationId
    const next: ConfirmRequest = {
      id,
      title: input.title,
      message: input.message,
      confirmLabel: input.confirmLabel || 'Confirm',
      cancelLabel: input.cancelLabel || 'Cancel',
      tone: input.tone || 'primary',
    }
    return new Promise<boolean>((resolve) => {
      pendingResolvers.set(id, resolve)
      request.value = next
    })
  }

  return { request, confirm, finish }
}

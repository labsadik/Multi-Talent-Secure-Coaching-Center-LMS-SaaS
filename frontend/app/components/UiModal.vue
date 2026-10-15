<script setup lang="ts">
const props = withDefaults(defineProps<{
  open: boolean
  title: string
  description?: string
  size?: 'sm' | 'md' | 'lg' | 'xl'
  closeOnBackdrop?: boolean
}>(), {
  description: '',
  size: 'md',
  closeOnBackdrop: true,
})
const emit = defineEmits<{ 'update:open': [value: boolean] }>()

const dialogRef = ref<HTMLElement | null>(null)
const titleId = useId()
const descriptionId = useId()
let previousFocus: HTMLElement | null = null
let previousBodyOverflow = ''

const sizeClass = computed(() => ({
  sm: 'max-w-md',
  md: 'max-w-xl',
  lg: 'max-w-3xl',
  xl: 'max-w-5xl',
})[props.size])

function close() {
  emit('update:open', false)
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return
  if (event.key === 'Escape') {
    event.preventDefault()
    close()
    return
  }
  if (event.key !== 'Tab') return
  const focusable = dialogRef.value?.querySelectorAll<HTMLElement>(
    'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])',
  )
  if (!focusable?.length) {
    event.preventDefault()
    dialogRef.value?.focus()
    return
  }
  const first = focusable.item(0)
  const last = focusable.item(focusable.length - 1)
  if (!first || !last) return
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault()
    last.focus()
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault()
    first.focus()
  }
}

watch(() => props.open, async (open) => {
  if (!import.meta.client) return
  if (open) {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    previousBodyOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    await nextTick()
    dialogRef.value?.focus()
  } else {
    document.body.style.overflow = previousBodyOverflow
    previousFocus?.focus({ preventScroll: true })
  }
}, { immediate: true })

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => {
  if (import.meta.client) {
    window.removeEventListener('keydown', onKeydown)
    if (props.open) document.body.style.overflow = previousBodyOverflow
  }
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="ui-modal-backdrop fixed inset-0 z-[100] flex items-end justify-center bg-slate-950/55 p-0 backdrop-blur-sm sm:items-center sm:p-5"
      @click.self="closeOnBackdrop && close()"
    >
      <section
        ref="dialogRef"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        :aria-describedby="description ? descriptionId : undefined"
        tabindex="-1"
        class="ui-modal-panel relative flex max-h-[94dvh] w-full flex-col overflow-hidden rounded-t-3xl border border-white/70 bg-white shadow-2xl shadow-slate-950/25 outline-none sm:max-h-[min(90dvh,54rem)] sm:rounded-3xl"
        :class="sizeClass"
      >
        <header class="flex shrink-0 items-start gap-4 border-b border-line px-5 py-4 sm:px-7 sm:py-5">
          <div class="min-w-0 flex-1">
            <h2 :id="titleId" class="text-lg font-semibold tracking-tight text-ink sm:text-xl">{{ title }}</h2>
            <p v-if="description" :id="descriptionId" class="mt-1.5 text-sm leading-6 text-muted">{{ description }}</p>
          </div>
          <button type="button" class="grid h-9 w-9 shrink-0 place-items-center rounded-xl border border-line text-xl leading-none text-muted transition hover:bg-canvas hover:text-ink" aria-label="Close dialog" @click="close">×</button>
        </header>
        <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain p-5 sm:p-7">
          <slot />
        </div>
      </section>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
const { request, finish } = useConfirmDialog()
function cancel() { if (request.value) finish(request.value.id, false) }
function onOpenChange(open: boolean) { if (!open) cancel() }
function accept() { if (request.value) finish(request.value.id, true) }
</script>

<template>
  <UiModal :open="Boolean(request)" :title="request?.title || 'Confirm action'" :description="request?.message || ''" size="sm" @update:open="onOpenChange">
    <div class="flex flex-col-reverse gap-3 sm:flex-row sm:justify-end">
      <button type="button" class="min-h-11 rounded-xl border border-line px-5 py-3 text-sm font-semibold text-ink transition hover:bg-canvas" @click="cancel">{{ request?.cancelLabel || 'Cancel' }}</button>
      <button type="button" class="min-h-11 rounded-xl px-5 py-3 text-sm font-semibold text-white transition disabled:cursor-not-allowed disabled:opacity-50" :class="request?.tone === 'danger' ? 'bg-red-600 hover:bg-red-700' : 'bg-brand hover:bg-brand-dark'" @click="accept">{{ request?.confirmLabel || 'Confirm' }}</button>
    </div>
  </UiModal>
</template>

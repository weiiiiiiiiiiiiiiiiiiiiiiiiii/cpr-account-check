<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
const props = defineProps<{ value: { name: string, text: string } | null }>()
const emit = defineEmits<{ close: [] }>()
const dialog = ref<HTMLDialogElement>()
const textarea = ref<HTMLTextAreaElement>()
const message = ref('')
watch(() => props.value, async (value) => { message.value = ''; await nextTick(); if (value && !dialog.value?.open) dialog.value?.showModal(); else if (!value) dialog.value?.close() })
async function copy() {
  if (!props.value) return
  try { await navigator.clipboard.writeText(props.value.text); message.value = '已复制，可保存为 JSON 文件' }
  catch {
    textarea.value?.focus(); textarea.value?.select()
    if (document.execCommand('copy')) message.value = '已复制，可保存为 JSON 文件'
    else message.value = '已选中文本，请复制后保存为 JSON 文件'
  }
}
</script>

<template>
  <dialog ref="dialog" aria-labelledby="export-title" @close="emit('close')">
    <template v-if="value">
      <header class="section-toolbar"><h2 id="export-title">导出 JSON</h2><button class="ghost" @click="dialog?.close()">关闭</button></header>
      <p class="muted">{{ value.name }}</p>
      <label class="export-content">JSON 内容<textarea ref="textarea" :value="value.text" readonly rows="12" spellcheck="false" /></label>
      <div class="section-toolbar"><p class="muted" role="status">{{ message || '复制内容后保存为 JSON 文件' }}</p><button class="primary" @click="copy">复制 JSON</button></div>
    </template>
  </dialog>
</template>

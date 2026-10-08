<script setup lang="ts">
import type { RunResult } from './types'
import { ref, watch, nextTick } from 'vue'
import { outcomeLabels } from './labels'
const props = defineProps<{ result: RunResult | null }>()
const emit = defineEmits<{ close: [] }>()
const dialog = ref<HTMLDialogElement>()
watch(() => props.result, async (result) => { await nextTick(); if (result && !dialog.value?.open) dialog.value?.showModal(); else if (!result) dialog.value?.close() })
</script>

<template>
  <dialog ref="dialog" aria-labelledby="result-title" @close="emit('close')" @click="event => { if (event.target === dialog) dialog?.close() }">
    <template v-if="result">
      <header class="section-toolbar"><h2 id="result-title">测试详情</h2><button class="ghost" @click="dialog?.close()">关闭</button></header>
      <dl class="details-grid">
        <dt>账号</dt><dd>{{ result.accountName || result.input.accountId }}</dd>
        <dt>题目</dt><dd>{{ result.input.probe.name }} · 第 {{ result.input.repetition }} 次</dd>
        <dt>结果</dt><dd><span :class="['badge', result.outcome]">{{ outcomeLabels[result.outcome] }}</span></dd>
        <dt>模型</dt><dd>{{ result.input.model }}<span v-if="result.responseModel && result.responseModel !== result.input.model"> → {{ result.responseModel }}</span></dd>
        <dt>推理档位</dt><dd>{{ result.input.reasoning }}</dd>
        <dt>耗时 / 超时</dt><dd>{{ (result.elapsedMs / 1000).toFixed(2) }} 秒 / {{ result.input.timeoutSeconds }} 秒</dd>
        <dt>测试时间</dt><dd>{{ new Date(result.testedAtMs).toLocaleString() }}</dd>
        <dt>预期答案</dt><dd>{{ result.input.probe.expected || '人工查看' }}</dd>
      </dl>
      <p v-if="result.error" class="notice error">{{ result.error }}<span v-if="result.errorCode">（{{ result.errorCode }}）</span></p>
      <h3>原始回答</h3><pre>{{ result.answer || '（无回答）' }}</pre>
      <p v-if="result.answerTruncated" class="notice">回答超出保存长度，显示前 8 KiB，未计入答案判定</p>
      <details><summary>题目与请求信息</summary><pre>{{ result.input.probe.prompt }}</pre><p class="muted">账号 ID：{{ result.input.accountId }}<br>请求 ID：{{ result.requestId || '未返回' }}<br>Client Key ID：{{ result.input.clientKeyId }}<br>账号约束：由宿主强制指定</p></details>
    </template>
  </dialog>
</template>

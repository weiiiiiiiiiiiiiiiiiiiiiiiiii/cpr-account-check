import type { Account, QueueEntry, RunInput, Settings } from './types'
import { computed, ref } from 'vue'
import { api } from './api'

export function useRunner() {
  const queue = ref<QueueEntry[]>([])
  const running = ref(false)
  const stopped = ref(false)
  const total = computed(() => queue.value.length)
  const done = computed(() => queue.value.filter(row => row.status === 'done').length)
  const active = computed(() => queue.value.filter(row => row.status === 'running').length)
  const cancelled = computed(() => queue.value.filter(row => row.status === 'cancelled').length)

  async function start(accounts: Account[], settings: Settings) {
    if (running.value) return
    const probes = settings.probes.filter(probe => probe.enabled)
    if (!accounts.length || !probes.length) throw new Error('请选择账号并启用至少一道题目')
    if (!settings.clientKeyId || !settings.model) throw new Error('请选择 Client Key 和模型')
    if (!Number.isInteger(settings.concurrency) || settings.concurrency < 1 || settings.concurrency > 4
      || !Number.isInteger(settings.repetitions) || settings.repetitions < 1 || settings.repetitions > 5
      || !Number.isInteger(settings.timeoutSeconds) || settings.timeoutSeconds < 5 || settings.timeoutSeconds > 90)
      throw new Error('并发须为 1–4，重复次数须为 1–5，超时须为 5–90 秒')
    if (probes.some(probe => !probe.name.trim() || !probe.prompt.trim() || (probe.rule !== 'manual' && !probe.expected.trim())))
      throw new Error('请补齐启用题目的名称、提示词和预期答案')
    const count = accounts.length * probes.length * settings.repetitions
    if (count > 2000) throw new Error('单批最多 2000 次测试，请减少账号、题目或重复次数')
    const batchId = crypto.randomUUID()
    const entries: QueueEntry[] = []
    for (const account of accounts) {
      for (const probe of probes) {
        for (let repetition = 1; repetition <= settings.repetitions; repetition++) {
          const input: RunInput = { runId: crypto.randomUUID(), batchId, accountId: account.account_id, probe: { ...probe }, model: settings.model, clientKeyId: settings.clientKeyId, reasoning: settings.reasoning, timeoutSeconds: settings.timeoutSeconds, repetition }
          entries.push({ input, accountName: account.name, status: 'pending' })
        }
      }
    }
    queue.value = entries
    running.value = true
    stopped.value = false
    let next = 0
    async function worker() {
      while (!stopped.value && next < queue.value.length) {
        const index = next++
        const entry = queue.value[index]!
        entry.status = 'running'
        try {
          const reply = await api.run(entry.input)
          entry.result = reply.result
          if (!reply.saved) entry.saveError = reply.saveError || '历史保存失败，请导出本次结果'
        }
        catch (error) { entry.error = error instanceof Error ? error.message : '调用失败' }
        finally { entry.status = 'done' }
      }
    }
    try { await Promise.all(Array.from({ length: Math.min(settings.concurrency, entries.length) }, worker)) }
    finally {
      for (const row of queue.value) { if (row.status === 'pending') row.status = 'cancelled' }
      running.value = false
    }
  }

  function stop() {
    stopped.value = true
    for (const row of queue.value) { if (row.status === 'pending') row.status = 'cancelled' }
  }
  return { queue, running, stopped, total, done, active, cancelled, start, stop }
}

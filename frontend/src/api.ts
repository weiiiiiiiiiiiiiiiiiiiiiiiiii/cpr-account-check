import type { RunInput, RunReply, Settings, Snapshot } from './types'

async function request<T>(path: string, method: 'GET' | 'POST', body?: unknown): Promise<T> {
  const host = window.codexProxyPlugin
  if (!host || host.version !== 2) throw new Error('请从 CPR 插件页面打开')
  const reply = await host.request({
    path,
    method,
    ...(body === undefined ? {} : { contentType: 'application/json', body: JSON.stringify(body) }),
  })
  if (reply.contentType.split(';')[0]?.trim() !== 'application/json') throw new Error('插件响应格式无效')
  const value = JSON.parse(new TextDecoder().decode(reply.body))
  if (reply.status < 200 || reply.status >= 300) throw new Error(value.error?.message || `请求失败（${reply.status}）`)
  return value as T
}

export const api = {
  snapshot: () => request<Snapshot>('api/snapshot', 'GET'),
  history: () => request<{ entries: RunReply['result'][] }>('api/history', 'GET'),
  models: (clientKeyId: string) => request<{ models: string[] }>('api/models', 'POST', { clientKeyId }),
  save: (expectedVersion: number | null, value: Settings) => request<{ version: number }>('api/settings', 'POST', { expectedVersion, value }),
  run: (input: RunInput) => request<RunReply>('api/run', 'POST', input),
}

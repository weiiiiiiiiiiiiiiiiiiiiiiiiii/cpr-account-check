// 仅供 Vite 开发模式演示，生产构建不包含模拟宿主
import type { Account, HostBridge, RunInput, RunResult, Settings } from './types'

export function installDemo() {
  const accounts: Account[] = [
    { account_id: 'demo-account-1', provider_id: 'openai', name: '主力账号', email: 'primary@example.com', group_ids: ['demo'], enabled: true, updated_at_ms: 0 },
    { account_id: 'demo-account-2', provider_id: 'openai', name: '备用账号', email: 'backup@example.com', group_ids: ['demo'], enabled: true, updated_at_ms: 0 },
    { account_id: 'demo-account-3', provider_id: 'openai', name: '待检查账号', email: 'check@example.com', group_ids: ['demo'], enabled: true, updated_at_ms: 0 },
    { account_id: 'demo-account-4', provider_id: 'openai', name: '停用账号', email: 'disabled@example.com', group_ids: [], enabled: false, updated_at_ms: 0 },
  ]
  let version: number | null = null
  let settings: Settings = { selectedProbeId: 'candy', probes: [{ id: 'candy', name: '糖果题', prompt: '在一个黑色的袋子里放有三种口味的糖果，每种糖果有两种不同的形状（圆形和五角星形，不同的形状靠手感可以分辨）。现已知不同口味的糖和不同形状的数量统计如下表。参赛者需要在活动前决定摸出的糖果数目，那么，最少取出多少个糖果才能保证手中同时拥有不同形状的苹果味和桃子味的糖？（同时手中有圆形苹果味匹配五角星桃子味糖果，或者有圆形桃子味匹配五角星苹果味糖果都满足要求）\n苹果味 桃子味 西瓜味\n圆形 7 9 8\n五角星形 7 6 4\n直接回答数字答案', expected: '21', rule: 'exact', enabled: true }], clientKeyId: 'demo-key', model: 'gpt-5', reasoning: 'high', timeoutSeconds: 60, repetitions: 1, concurrency: 2 }
  const history: RunResult[] = []
  const host: HostBridge = {
    version: 2,
    get theme() { return new URLSearchParams(location.search).get('theme') === 'dark' ? 'dark' : 'light' },
    async request(input) {
      await new Promise(resolve => setTimeout(resolve, input.path === 'api/run' ? 1000 : 80))
      let value: unknown
      let status = 200
      const data = input.body ? JSON.parse(input.body) : {}
      switch (input.path) {
        case 'api/snapshot': value = { accounts, keys: [{ id: 'demo-key', name: '检测专用 Key', enabled: true }], version, settings }; break
        case 'api/models': value = { models: ['gpt-5', 'gpt-5.1', 'gpt-5.2'] }; break
        case 'api/history': value = { entries: history }; break
        case 'api/settings':
          if (data.expectedVersion !== version) { status = 409; value = { error: { message: '设置已更新，请重新加载' } } }
          else { settings = data.value; version = (version || 0) + 1; value = { version } }
          break
        case 'api/run': {
          const run = data as RunInput
          const account = accounts.find(account => account.account_id === run.accountId)!
          const passed = account.account_id === 'demo-account-1'
          const callError = account.account_id === 'demo-account-3'
          const result: RunResult = { input: run, accountName: account.name, provider: account.provider_id, selectionMode: 'host_required_account', outcome: callError ? 'call_error' : passed ? 'passed' : 'wrong_answer', answer: callError ? '' : passed ? '21' : '22', errorCode: callError ? 'upstream' : null, error: callError ? '上游限流，请稍后重试' : null, requestId: 'demo-request', responseModel: run.model, elapsedMs: 1050, testedAtMs: Date.now(), answerTruncated: false }
          history.unshift(result); history.splice(100); value = { result, saved: true, saveError: null }; break
        }
        default: status = 404; value = { error: { message: '未知模拟接口' } }
      }
      return { status, contentType: 'application/json', body: new TextEncoder().encode(JSON.stringify(value)).buffer }
    },
  }
  Object.defineProperty(window, 'codexProxyPlugin', { value: host })
}

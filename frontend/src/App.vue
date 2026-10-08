<script setup lang="ts">
import type { Account, ClientKey, QueueEntry, RunResult, Settings } from './types'
import { computed, onMounted, ref, watch } from 'vue'
import { api } from './api'
import { useRunner } from './runner'
import { outcomeLabels } from './labels'
import ProbeEditor from './ProbeEditor.vue'
import ResultDetail from './ResultDetail.vue'
import ExportDialog from './ExportDialog.vue'

const accounts = ref<Account[]>([])
const keys = ref<ClientKey[]>([])
const settings = ref<Settings>({ probes: [], selectedProbeId: '', model: '', clientKeyId: '', reasoning: 'high', timeoutSeconds: 60, repetitions: 1, concurrency: 2 })
const version = ref<number | null>(null)
const baseline = ref('')
const initialized = ref(false)
const loading = ref(false)
const saving = ref(false)
const message = ref('')
const error = ref('')
const tab = ref<'accounts' | 'probes' | 'history'>('accounts')
const search = ref('')
const provider = ref('')
const selected = ref<string[]>([])
const models = ref<string[]>([])
const modelsLoading = ref(false)
const modelsError = ref('')
const history = ref<RunResult[]>([])
const historySearch = ref('')
const historyOutcome = ref('')
const detail = ref<RunResult | null>(null)
const exported = ref<{ name: string, text: string } | null>(null)
const demo = import.meta.env.DEV && new URLSearchParams(location.search).has('demo')
const runner = useRunner()
const { queue, running, stopped, total, done, active, cancelled } = runner
const dirty = computed(() => JSON.stringify(settings.value) !== baseline.value)
const providers = computed(() => [...new Set(accounts.value.map(account => account.provider_id))])
const visible = computed(() => accounts.value.filter(account => (!provider.value || account.provider_id === provider.value) && `${account.name} ${account.email || ''} ${account.account_id}`.toLowerCase().includes(search.value.toLowerCase())))
const selectable = computed(() => visible.value.filter(account => account.enabled))
const allSelected = computed(() => selectable.value.length > 0 && selectable.value.every(account => selected.value.includes(account.account_id)))
const enabledProbes = computed(() => settings.value.probes.filter(probe => probe.enabled))
const selectedProbe = computed(() => enabledProbes.value.find(probe => probe.id === settings.value.selectedProbeId))
const planned = computed(() => selectedProbe.value ? selected.value.length * settings.value.repetitions : 0)
watch([enabledProbes, () => settings.value.selectedProbeId], () => {
  if (!selectedProbe.value) settings.value.selectedProbeId = enabledProbes.value[0]?.id || ''
}, { flush: 'sync' })
const filteredHistory = computed(() => history.value.filter(result => (!historyOutcome.value || result.outcome === historyOutcome.value) && `${result.accountName} ${result.input.accountId} ${result.input.model} ${result.input.probe.name}`.toLowerCase().includes(historySearch.value.toLowerCase())))
const currentResults = computed(() => queue.value.flatMap(row => row.result ? [row.result] : []))
const combined = computed(() => [...currentResults.value].reverse().concat(history.value).filter((row, i, rows) => rows.findIndex(other => other.input.runId === row.input.runId) === i))
const passed = computed(() => currentResults.value.filter(result => result.outcome === 'passed').length)
const anomalies = computed(() => currentResults.value.filter(result => ['wrong_answer', 'format_error'].includes(result.outcome)).length)
const callErrors = computed(() => currentResults.value.filter(result => result.outcome === 'call_error').length + queue.value.filter(row => row.error).length)
const canRun = computed(() => initialized.value && !loading.value && !running.value && !saving.value && !modelsLoading.value && Boolean(settings.value.model && settings.value.clientKeyId && selectedProbe.value))

function latest(accountId: string) {
  const matching = combined.value.filter(result => result.input.accountId === accountId && result.input.probe.id === settings.value.selectedProbeId)
  const newest = matching[0]
  if (!newest) return null
  const results = matching.filter(result => result.input.batchId === newest.input.batchId)
  const valid = results.filter(result => ['passed', 'wrong_answer', 'format_error'].includes(result.outcome))
  const hasError = results.some(result => result.outcome === 'call_error')
  return { newest, valid: valid.length, passed: valid.filter(result => result.outcome === 'passed').length, outcome: valid.some(result => result.outcome !== 'passed') ? '疑似异常' : hasError ? '无法完整判定' : valid.length ? '通过' : '待人工查看', className: valid.some(result => result.outcome !== 'passed') ? 'wrong_answer' : hasError ? 'call_error' : valid.length ? 'passed' : 'manual_review' }
}
function toggleAll() { const ids = new Set(selected.value); for (const account of selectable.value) { if (allSelected.value) ids.delete(account.account_id); else ids.add(account.account_id) } selected.value = [...ids] }
async function load(reset = false) {
  loading.value = true; error.value = ''; message.value = ''
  try {
    const snapshot = await api.snapshot()
    accounts.value = snapshot.accounts; keys.value = snapshot.keys
    selected.value = selected.value.filter(id => accounts.value.some(account => account.account_id === id && account.enabled))
    if (!initialized.value || reset) {
      settings.value = snapshot.settings; version.value = snapshot.version
      if (!keys.value.some(key => key.id === settings.value.clientKeyId && key.enabled)) settings.value.clientKeyId = keys.value.find(key => key.enabled)?.id || ''
      baseline.value = JSON.stringify(settings.value); initialized.value = true
    }
    history.value = (await api.history()).entries
  } catch (cause) { error.value = cause instanceof Error ? cause.message : '加载失败' }
  finally { loading.value = false }
}
watch(() => settings.value.clientKeyId, async (key, _old, onCleanup) => {
  let valid = true; onCleanup(() => { valid = false })
  modelsLoading.value = Boolean(key); modelsError.value = ''; models.value = []
  if (!key) { settings.value.model = ''; return }
  try { const reply = await api.models(key); if (!valid) return; models.value = reply.models; if (!models.value.includes(settings.value.model)) settings.value.model = models.value[0] || '' }
  catch (cause) { if (valid) { settings.value.model = ''; modelsError.value = cause instanceof Error ? cause.message : '无法加载模型' } }
  finally { if (valid) modelsLoading.value = false }
})
async function save() {
  saving.value = true; error.value = ''; message.value = ''
  try { const reply = await api.save(version.value, settings.value); version.value = reply.version; baseline.value = JSON.stringify(settings.value); message.value = '设置已保存' }
  catch (cause) { error.value = cause instanceof Error ? cause.message : '保存失败' }
  finally { saving.value = false }
}
async function start(ids?: string[]) {
  error.value = ''; message.value = ''
  try {
    const testAccounts = accounts.value.filter(account => account.enabled && (ids || selected.value).includes(account.account_id))
    const snapshot = JSON.parse(JSON.stringify(settings.value)) as Settings
    await runner.start(testAccounts, snapshot)
    try { history.value = (await api.history()).entries } catch { message.value = '测试已结束，历史加载失败，可导出本次结果' }
  } catch (cause) { error.value = cause instanceof Error ? cause.message : '测试失败' }
}
function rowLabel(row: QueueEntry) { return row.result ? outcomeLabels[row.result.outcome] : row.error ? '调用错误' : ({ pending: '待执行', running: '执行中', done: '已完成', cancelled: '已取消' })[row.status] }
function download(name: string, value: unknown) {
  exported.value = { name, text: JSON.stringify(value, null, 2) }
}
onMounted(() => load())
window.addEventListener('beforeunload', event => { if (running.value) { event.preventDefault(); event.returnValue = '' } })
</script>

<template>
  <main>
    <p v-if="demo" class="notice">模拟数据预览，未连接真实 CPR 或调用账号</p>
    <div class="toolbar">
      <nav aria-label="检测页面"><button v-for="item in [{ id: 'accounts', name: '账号测试' }, { id: 'probes', name: '测试题库' }, { id: 'history', name: '历史记录' }]" :key="item.id" :class="{ selected: tab === item.id }" :aria-current="tab === item.id ? 'page' : undefined" @click="tab = item.id as typeof tab">{{ item.name }}</button></nav>
      <div class="inline"><span v-if="dirty" class="muted">设置未保存</span><button :disabled="loading || running" @click="load()">{{ loading ? '加载中…' : '刷新' }}</button><button :disabled="!initialized || saving || running" @click="save">{{ saving ? '保存中…' : '保存设置' }}</button></div>
    </div>
    <p v-if="error" class="notice error" role="alert">{{ error }}<button v-if="initialized && !running" class="ghost" @click="load(true)">重新加载设置</button><button v-if="!initialized" @click="load()">重试</button></p>
    <p v-if="message" class="notice success" role="status">{{ message }}</p>
    <section v-if="initialized" class="panel settings">
      <fieldset :disabled="running || saving" class="settings-grid">
        <label>Client Key<select v-model="settings.clientKeyId"><option value="">请选择</option><option v-for="key in keys" :key="key.id" :value="key.id" :disabled="!key.enabled">{{ key.name }}{{ key.enabled ? '' : '（停用）' }}</option></select></label>
        <label>测试模型<select v-model="settings.model" :disabled="modelsLoading"><option value="">{{ modelsLoading ? '加载中…' : '请选择' }}</option><option v-for="model in models" :key="model">{{ model }}</option></select></label>
        <label>推理档位<select v-model="settings.reasoning"><option value="default">模型默认</option><option v-for="level in ['none', 'minimal', 'low', 'medium', 'high', 'xhigh']" :key="level">{{ level }}</option></select></label>
        <label>超时（秒）<input v-model.number="settings.timeoutSeconds" type="number" min="5" max="90"></label>
        <label>重复次数<input v-model.number="settings.repetitions" type="number" min="1" max="5"></label>
        <label>并发<input v-model.number="settings.concurrency" type="number" min="1" max="4"></label>
      </fieldset>
      <p v-if="modelsError" class="notice error" role="alert">{{ modelsError }}</p>
      <p class="muted compact">调用会消耗所选 Key 的额度，调用错误不计为答案错误</p>
    </section>
    <section v-if="tab === 'accounts'" class="panel">
      <label class="probe-select">测试题目<select v-model="settings.selectedProbeId" :disabled="running || saving || !enabledProbes.length"><option v-if="!enabledProbes.length" value="">请先在题库启用题目</option><option v-for="probe in enabledProbes" :key="probe.id" :value="probe.id">{{ probe.name }}</option></select></label>
      <p class="muted compact">每批只测试所选题目，同一账号的重复测试按顺序执行；CPR 停用的账号不支持插件测试</p>
      <div class="section-toolbar">
        <div class="inline grow"><input v-model="search" class="search" aria-label="搜索账号" placeholder="搜索名称、邮箱或账号 ID"><select v-model="provider" aria-label="筛选 Provider"><option value="">全部 Provider</option><option v-for="value in providers" :key="value">{{ value }}</option></select></div>
        <button class="primary" :disabled="!canRun || !selected.length" @click="start()">测试勾选账号（{{ selected.length }}）</button>
      </div>
      <div class="section-toolbar muted compact"><span>{{ accounts.length }} 个账号 · 当前题目：{{ selectedProbe?.name || '未选择' }} · 预计 {{ planned }} 次调用</span><button v-if="selected.length" class="ghost" :disabled="running" @click="selected = []">清空选择</button></div>
      <div class="table-wrap"><table>
        <thead><tr><th class="check"><input type="checkbox" aria-label="勾选当前筛选下的启用账号" :checked="allSelected" :disabled="running || !selectable.length" @change="toggleAll"></th><th>账号</th><th>Provider</th><th>状态</th><th>最近检测</th><th>通过 / 判定</th><th>操作</th></tr></thead>
        <tbody><tr v-for="account in visible" :key="account.account_id">
          <td><input v-model="selected" type="checkbox" :value="account.account_id" :aria-label="`选择 ${account.name}`" :disabled="!account.enabled || running"></td>
          <td><strong>{{ account.name }}</strong><div class="muted small account-email">{{ account.email || account.account_id }}</div></td>
          <td>{{ account.provider_id }}</td><td><span :class="['badge', account.enabled ? 'enabled' : 'disabled']">{{ account.enabled ? '启用' : '停用' }}</span></td>
          <td><button v-if="latest(account.account_id)" class="ghost result-link" @click="detail = latest(account.account_id)!.newest"><span :class="['badge', latest(account.account_id)!.className]">{{ latest(account.account_id)!.outcome }}</span></button><span v-else class="muted">未测试</span></td>
          <td>{{ latest(account.account_id)?.valid ? `${latest(account.account_id)!.passed} / ${latest(account.account_id)!.valid}` : '—' }}</td>
          <td><button :disabled="!canRun || !account.enabled" @click="start([account.account_id])">测试</button></td>
        </tr><tr v-if="!visible.length"><td colspan="7" class="empty">{{ loading ? '正在加载账号…' : '没有匹配的账号' }}</td></tr></tbody>
      </table></div>
    </section>
    <ProbeEditor v-if="tab === 'probes' && initialized" :probes="settings.probes" :disabled="running || saving" @change="settings.probes = $event" />
    <section v-if="tab === 'accounts' && queue.length" class="panel">
      <div class="section-toolbar"><h2>本次测试</h2><div class="inline"><button :disabled="!queue.length" @click="download('本次账号测试.json', queue)">导出结果</button><button v-if="running" :disabled="stopped" @click="runner.stop">{{ stopped ? '正在等待执行中的测试结束' : '取消待执行测试' }}</button></div></div>
      <div class="stats"><span><b>{{ done }}</b> / {{ total }} 已完成</span><span><b>{{ active }}</b> 执行中</span><span class="good"><b>{{ passed }}</b> 通过</span><span class="bad"><b>{{ anomalies }}</b> 答案或格式异常</span><span><b>{{ callErrors }}</b> 调用错误</span><span v-if="cancelled"><b>{{ cancelled }}</b> 已取消</span></div>
      <progress :value="done + cancelled" :max="total" aria-label="批量测试进度" />
      <div class="table-wrap"><table><thead><tr><th>账号</th><th>题目</th><th>次数</th><th>结果</th><th>耗时</th><th>操作</th></tr></thead><tbody><tr v-for="row in queue" :key="row.input.runId"><td>{{ row.accountName }}</td><td>{{ row.input.probe.name }}</td><td>{{ row.input.repetition }}</td><td><span :class="['badge', row.result?.outcome || (row.error ? 'call_error' : row.status)]">{{ rowLabel(row) }}</span><div v-if="row.error || row.result?.error || row.saveError" class="small bad">{{ row.error || row.result?.error || row.saveError }}</div></td><td>{{ row.result ? `${(row.result.elapsedMs / 1000).toFixed(2)} 秒` : '—' }}</td><td><button v-if="row.result" class="ghost" @click="detail = row.result">详情</button></td></tr></tbody></table></div>
      <p class="muted compact">单题未通过仅代表本次测试异常，不能单独证明账号已降智</p>
    </section>
    <section v-if="tab === 'history'" class="panel">
      <div class="section-toolbar"><div class="inline grow"><input v-model="historySearch" class="search" aria-label="搜索历史" placeholder="搜索账号、题目或模型"><select v-model="historyOutcome" aria-label="筛选测试结果"><option value="">全部结果</option><option v-for="(label, value) in outcomeLabels" :key="value" :value="value">{{ label }}</option></select></div><button :disabled="!filteredHistory.length" @click="download('账号测试历史.json', filteredHistory)">导出历史</button></div>
      <p class="muted compact">保存最近 100 次测试，历史保留当时的题目和参数</p>
      <div class="table-wrap"><table><thead><tr><th>测试时间</th><th>账号 / 题目</th><th>模型 / 推理</th><th>结果</th><th>耗时</th><th>操作</th></tr></thead><tbody><tr v-for="result in filteredHistory" :key="result.input.runId"><td>{{ new Date(result.testedAtMs).toLocaleString() }}</td><td><strong>{{ result.accountName || result.input.accountId }}</strong><div class="muted small">{{ result.input.probe.name }} · 第 {{ result.input.repetition }} 次</div></td><td>{{ result.input.model }}<div class="muted small">{{ result.input.reasoning }}</div></td><td><span :class="['badge', result.outcome]">{{ outcomeLabels[result.outcome] }}</span></td><td>{{ (result.elapsedMs / 1000).toFixed(2) }} 秒</td><td><button class="ghost" @click="detail = result">详情</button></td></tr><tr v-if="!filteredHistory.length"><td colspan="6" class="empty">暂无匹配的测试记录</td></tr></tbody></table></div>
    </section>
    <ResultDetail :result="detail" @close="detail = null" />
    <ExportDialog :value="exported" @close="exported = null" />
  </main>
</template>

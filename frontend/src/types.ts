export type Rule = 'exact' | 'contains' | 'regex' | 'manual'
export type Outcome = 'passed' | 'wrong_answer' | 'format_error' | 'call_error' | 'manual_review'
export interface Probe { id: string, name: string, prompt: string, expected: string, rule: Rule, enabled: boolean }
export interface Settings { probes: Probe[], model: string, clientKeyId: string, reasoning: string, timeoutSeconds: number, repetitions: number, concurrency: number }
export interface Account { account_id: string, provider_id: string, name: string, email: string | null, group_ids: string[], enabled: boolean, updated_at_ms: number }
export interface ClientKey { id: string, name: string, enabled: boolean }
export interface Snapshot { accounts: Account[], keys: ClientKey[], version: number | null, settings: Settings }
export interface RunInput { runId: string, batchId: string, accountId: string, probe: Probe, model: string, clientKeyId: string, reasoning: string, timeoutSeconds: number, repetition: number }
export interface RunResult { input: RunInput, accountName: string, provider: string, selectionMode: string, outcome: Outcome, answer: string, errorCode: string | null, error: string | null, requestId: string | null, responseModel: string | null, elapsedMs: number, testedAtMs: number, answerTruncated: boolean }
export interface RunReply { result: RunResult, saved: boolean, saveError: string | null }
export interface QueueEntry { input: RunInput, accountName: string, status: 'pending' | 'running' | 'done' | 'cancelled', result?: RunResult, error?: string, saveError?: string }
export interface HostBridge {
  readonly version: number
  readonly theme: 'light' | 'dark'
  request(input: { method: 'GET' | 'POST', path: string, contentType?: string, body?: string }): Promise<{ status: number, contentType: string, body: ArrayBuffer }>
}
declare global { interface Window { readonly codexProxyPlugin?: HostBridge } }

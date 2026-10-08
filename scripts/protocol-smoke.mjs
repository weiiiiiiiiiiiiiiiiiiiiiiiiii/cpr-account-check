// 模拟宿主协议，验证真实插件进程，不调用账号或上游
import { spawn } from 'node:child_process';
import assert from 'node:assert/strict';
import { once } from 'node:events';
import { resolve } from 'node:path';
import net from 'node:net';

const binary = process.argv[2];
if (!binary) throw new Error('用法: node scripts/protocol-smoke.mjs <插件可执行文件>');
let child, exited;
if (binary.startsWith('tcp:')) {
  const socket = net.createConnection({ host: '127.0.0.1', port: Number(binary.slice(4)) });
  await once(socket, 'connect');
  child = { stdin: socket, stdout: socket, stderr: { on() {} }, kill: () => socket.destroy() };
  exited = once(socket, 'close');
} else {
  child = spawn(resolve(binary), [], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true });
  exited = once(child, 'exit');
}
child.stderr.on('data', data => process.stderr.write(data));
const handshake = { protocol_version: 2, artifact_sha256: 'a'.repeat(64), plugin_id: 'account-lab.cognition-check', instance_id: 'test-instance', generation: 1, incarnation: 'test-incarnation', configuration: {}, contributes: { management: { id: 'account-lab.cognition-check.management', version: 1, stages: ['management'], inputFormats: [], outputFormats: [] } } };
const pending = new Map();
const state = new Map();
const executions = new Map();
let stash = Buffer.alloc(0), next = 1, starts = 0, closes = 0, conflicts = 0;
let ready;
const readyPromise = new Promise(resolve => { ready = resolve });
const faults = [];
const accounts = [
  { account_id: 'a1', provider_id: 'openai', name: 'Test One', email: 'one@example.com', enabled: true, group_ids: ['g1'], updated_at_ms: 0 },
  { account_id: 'a2', provider_id: 'openai', name: 'Test Two', email: null, enabled: true, group_ids: ['g1'], updated_at_ms: 0 },
  { account_id: 'a3', provider_id: 'openai', name: 'Disabled', email: null, enabled: false, group_ids: [], updated_at_ms: 0 },
];

function frame(message, payload = Buffer.alloc(0)) {
  const metadata = Buffer.from(JSON.stringify(message));
  assert.ok(metadata.length <= 64 * 1024, 'Protocol metadata exceeds SDK frame limit');
  const header = Buffer.alloc(12); header.writeUInt32BE(metadata.length); header.writeBigUInt64BE(BigInt(payload.length), 4);
  child.stdin.write(Buffer.concat([header, metadata, payload]));
}
function error(id, code, message) { frame({ type: 'error', id, error: { code, message, send_state: 'not_sent' } }) }
function result(id, value, payload = Buffer.alloc(0)) { frame({ type: 'result', id, result: value }, payload) }
function batch(answer, reason = 'stop') {
  const metadata = Buffer.from(JSON.stringify({ facts: [{ type: 'text_delta', index: 0, text: answer }, { type: 'completed', id: 'response-1', model: 'test-model', reason }] }));
  const eventHeader = Buffer.alloc(16); eventHeader.write('GPE2'); eventHeader.writeUInt32BE(metadata.length, 4);
  const event = Buffer.concat([eventHeader, metadata]);
  const header = Buffer.alloc(12); header.write('HME1'); header.writeUInt32BE(1, 4); header.writeUInt32BE(event.length, 8);
  return Buffer.concat([header, event]);
}
async function callback(message, payload) {
  const { id, method, params } = message;
  switch (method) {
    case 'host.data.accounts.list': {
      const query = JSON.parse(payload);
      assert.equal(query.limit, 200);
      const second = query.cursor === 'page2';
      result(id, {}, Buffer.from(JSON.stringify({ schema_version: 1, accounts: second ? accounts.slice(1) : accounts.slice(0, 1), next_cursor: second ? null : 'page2' }))); break;
    }
    case 'host.keys.list': result(id, { keys: [{ id: 'key1', name: 'Test Key', enabled: true }], next_cursor: null }); break;
    case 'host.models.list': assert.equal(params.client_key_id, 'key1'); result(id, { models: ['test-model'] }); break;
    case 'host.state.get': assert.equal(payload.length, 0); result(id, { record: state.get(`${params.namespace}/${params.key}`) || null }); break;
    case 'host.state.put': {
      assert.equal(payload.length, 0);
      assert.ok(Buffer.byteLength(JSON.stringify(params.value)) <= 48 * 1024, 'State exceeds declared record limit');
      const key = `${params.namespace}/${params.key}`;
      const previous = state.get(key);
      if (params.expected_version !== (previous?.version ?? null) || (params.key === 'index' && conflicts-- > 0)) error(id, 'conflict', '版本冲突');
      else { const version = (previous?.version || 0) + 1; state.set(key, { version, schema_version: 1, value: params.value }); result(id, { version }); }
      break;
    }
    case 'host.model.execute_stream': {
      const body = JSON.parse(payload);
      assert.ok(['a1', 'a2'].includes(params.account_id));
      assert.equal(params.provider, 'openai'); assert.equal(params.client_key_id, 'key1'); assert.equal(params.protocol, 'openai');
      assert.equal(body.stream, true); assert.equal(body.store, false); assert.equal(body.reasoning.effort, 'high');
      starts++;
      if (body.input === 'error') { error(id, 'upstream', '上游限流'); break; }
      const stream = `stream-${starts}`; executions.set(stream, body.input); result(id, { request_id: `req-${starts}`, stream }); break;
    }
    case 'host.model.stream_read': {
      const mode = executions.get(params.stream);
      if (mode === 'timeout' || mode === 'cancel') break;
      const answer = mode === 'format' ? '答案是21' : mode === 'wrong' ? '22' : mode === 'long' ? '桃'.repeat(3000) : ' 21\n';
      result(id, { events: 1, end: true }, batch(answer, mode === 'incomplete' ? 'length' : 'stop')); break;
    }
    case 'host.model.stream_close': closes++; executions.delete(params.stream); result(id, {}); break;
    default: throw new Error(`Unexpected callback ${method}`);
  }
}
child.stdout.on('data', data => {
  stash = Buffer.concat([stash, data]);
  while (stash.length >= 12) {
    const metadataLength = stash.readUInt32BE(0), payloadLength = Number(stash.readBigUInt64BE(4)), total = 12 + metadataLength + payloadLength;
    if (stash.length < total) break;
    const message = JSON.parse(stash.subarray(12, 12 + metadataLength));
    const payload = stash.subarray(12 + metadataLength, total); stash = stash.subarray(total);
    if (message.type === 'ready') ready(message);
    else if (message.type === 'callback') callback(message, payload).catch(error => { faults.push(error); child.kill() });
    else if (['result', 'error', 'cancelled'].includes(message.type)) {
      const wait = pending.get(message.id); pending.delete(message.id); wait?.({ message, payload });
    }
  }
});
function call(method, params, payload = Buffer.alloc(0), stage = 'management', timeout = 120000) {
  const id = next; next += 2;
  const promise = new Promise(resolve => pending.set(id, resolve));
  const context = { call_id: id, instance_id: handshake.instance_id, generation: 1, incarnation: handshake.incarnation, stage, timeout_ms: timeout, resource_stream: false, resource_scope_id: `scope-${id}` };
  frame({ type: 'call', id, method, context, params }, payload);
  return { id, promise };
}
async function api(path, body, timeout) {
  const { promise } = call('management.handle', { method: body ? 'POST' : 'GET', path, query: '', content_type: body ? 'application/json' : null, headers: [] }, body ? Buffer.from(JSON.stringify(body)) : Buffer.alloc(0), 'management', timeout);
  const reply = await promise;
  assert.equal(reply.message.type, 'result');
  return { status: reply.message.result.status, value: JSON.parse(reply.payload) };
}
function input(mode, accountId = 'a1', rule = 'exact') {
  return { runId: `run-${mode}-${next}`, batchId: 'batch1', accountId, probe: { id: mode, name: mode, prompt: mode, expected: '21', enabled: true, rule }, model: 'test-model', clientKeyId: 'key1', reasoning: 'high', timeoutSeconds: 5, repetition: 1 };
}
const watchdog = setTimeout(() => { console.error('Protocol test timed out'); child.kill(); process.exitCode = 1 }, 35000);
try {
  frame({ type: 'hello', handshake });
  assert.equal((await readyPromise).protocol_version, 2);
  const registration = await call('plugin.register', {}, Buffer.alloc(0), 'registration').promise;
  assert.equal(registration.message.result.contributes.management.version, 1);
  const pages = await call('management.register', {}, Buffer.alloc(0), 'registration').promise;
  assert.equal(JSON.parse(pages.payload).pages[0].entry, 'web/index.html');
  const snapshot = await api('api/snapshot');
  assert.equal(snapshot.value.accounts.length, 3); assert.equal(snapshot.value.settings.probes[0].expected, '21');
  assert.equal((await api('api/models', { clientKeyId: 'key1' })).value.models[0], 'test-model');
  const settings = snapshot.value.settings;
  assert.equal((await api('api/settings', { expectedVersion: null, value: settings })).status, 200);
  assert.equal((await api('api/settings', { expectedVersion: null, value: settings })).status, 409);
  conflicts = 1;
  const passInput = input('pass', 'a2');
  const pass = await api('api/run', passInput);
  assert.equal(pass.value.result.outcome, 'passed'); assert.equal(pass.value.saved, true);
  assert.equal(pass.value.result.input.accountId, 'a2');
  const count = starts;
  assert.equal((await api('api/run', passInput)).value.result.outcome, 'passed'); assert.equal(starts, count);
  assert.equal((await api('api/run', { ...passInput, model: 'changed-model' })).status, 409);
  for (const [mode, expected] of [['wrong', 'wrong_answer'], ['format', 'format_error'], ['error', 'call_error'], ['incomplete', 'call_error'], ['long', 'call_error']]) {
    const reply = await api('api/run', input(mode)); assert.equal(reply.value.result.outcome, expected, mode); assert.equal(reply.value.saved, true);
  }
  assert.equal((await api('api/run', input('manual', 'a1', 'manual'))).value.result.outcome, 'manual_review');
  assert.equal((await api('api/run', input('disabled', 'a3'))).value.result.outcome, 'call_error');
  const bad = input('bad'); bad.probe.rule = 'regex'; bad.probe.expected = '[';
  assert.equal((await api('api/run', bad)).status, 400);
  const concurrent = await Promise.all([api('api/run', input('pass')), api('api/run', input('pass', 'a2'))]);
  assert.ok(concurrent.every(reply => reply.value.saved));
  const timeoutInput = input('timeout');
  const timeoutPromise = api('api/run', timeoutInput, 10000);
  await new Promise(resolve => setTimeout(resolve, 100));
  assert.equal((await api('api/run', timeoutInput)).status, 409);
  const timeout = await timeoutPromise;
  assert.equal(timeout.value.result.outcome, 'call_error'); assert.equal(timeout.value.result.errorCode, 'timeout'); assert.ok(closes > 0);
  const cancelInput = input('cancel');
  const cancellation = call('management.handle', { method: 'POST', path: 'api/run', query: '', content_type: 'application/json', headers: [] }, Buffer.from(JSON.stringify(cancelInput)));
  await new Promise(resolve => setTimeout(resolve, 100)); frame({ type: 'cancel', id: cancellation.id });
  assert.equal((await cancellation.promise).message.type, 'cancelled');
  assert.equal((await api('api/run', input('pass'))).value.result.outcome, 'passed');
  const history = (await api('api/history')).value.entries;
  assert.ok(history.some(result => result.input.runId === passInput.runId));
  assert.equal(new Set(history.map(result => result.input.runId)).size, history.length);
  for (let i = 0; i < 105; i++) { const reply = await api('api/run', input('long')); assert.ok(reply.value.saved, reply.value.saveError); }
  const bounded = (await api('api/history')).value.entries;
  assert.equal(bounded.length, 100);
  assert.ok(state.size <= 102);
  assert.equal(bounded[0].input.probe.id, 'long');
  assert.equal(faults.length, 0);
  console.log('PASS: handshake, registration, pagination, settings conflict, account pinning, grading, call errors, history merge, duplicate prevention, timeout close, cancellation recovery, history capacity');
} finally {
  clearTimeout(watchdog);
  frame({ type: 'shutdown' });
  setTimeout(() => child.kill(), 1000).unref();
  await exited;
}

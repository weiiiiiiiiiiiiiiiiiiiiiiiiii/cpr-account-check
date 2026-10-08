// 使用实际调度器与 Vue 响应式状态验证单题选择、账号串行及取消，不调用上游
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { randomUUID } from 'node:crypto';
import vm from 'node:vm';
import { test } from 'node:test';

const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const ts = require('typescript');
const vue = require('vue');
const source = await readFile(new URL('../frontend/src/runner.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
function runnerWith(run) {
  const exports = {};
  vm.runInNewContext(compiled, { exports, crypto: { randomUUID }, require: name => {
    if (name === 'vue') return vue;
    if (name === './api') return { api: { run } };
    throw new Error(`Unexpected import ${name}`);
  } });
  return exports.useRunner();
}
const accounts = ['a', 'b', 'c'].map(account_id => ({ account_id, name: account_id, enabled: true }));
const probe = id => ({ id, name: id, prompt: id, expected: '21', rule: 'exact', enabled: true });
const settings = overrides => ({ probes: [probe('one'), { ...probe('two'), prompt: '', expected: '' }], selectedProbeId: 'one', clientKeyId: 'key', model: 'model', reasoning: 'high', timeoutSeconds: 60, repetitions: 3, concurrency: 2, ...overrides });
const reply = input => ({ result: { input, outcome: 'passed' }, saved: true });
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r }); return { promise, resolve }; };

test('only selected question runs; another unfinished question does not block it', async () => {
  const calls = [];
  const runner = runnerWith(async input => { calls.push(input); return reply(input) });
  await runner.start(accounts.slice(0, 1), settings());
  assert.equal(calls.length, 3);
  assert.ok(calls.every(input => input.probe.id === 'one'));
  assert.equal(runner.total.value, 3);
  assert.equal(runner.done.value, 3);
  await runner.start(accounts.slice(0, 1), settings({ selectedProbeId: 'two', probes: [probe('one'), probe('two')], repetitions: 1 }));
  assert.equal(calls.at(-1).probe.id, 'two');
  await assert.rejects(runner.start(accounts, settings({ selectedProbeId: 'missing' })));
  assert.equal(calls.length, 4);
});

test('accounts run concurrently while repetitions of each account never overlap', async () => {
  const inflight = new Set();
  const gates = [];
  const calls = [];
  let maximum = 0;
  const runner = runnerWith(async input => {
    assert.ok(!inflight.has(input.accountId), `overlapping requests for ${input.accountId}`);
    inflight.add(input.accountId);
    maximum = Math.max(maximum, inflight.size);
    calls.push(input);
    const gate = deferred(); gates.push(gate);
    await gate.promise;
    inflight.delete(input.accountId);
    return reply(input);
  });
  const running = runner.start(accounts, settings());
  assert.equal(runner.active.value, 2);
  assert.equal(calls.length, 2);
  for (let i = 0; i < 9; i++) {
    assert.ok(gates[i], `request ${i + 1} scheduled`);
    gates[i].resolve();
    await new Promise(resolve => setImmediate(resolve));
  }
  await running;
  assert.equal(maximum, 2);
  assert.equal(calls.length, 9);
  for (const account of accounts) assert.deepEqual(calls.filter(input => input.accountId === account.account_id).map(input => input.repetition), [1, 2, 3]);
  assert.equal(runner.done.value, 9);
  assert.equal(runner.running.value, false);
});

test('cancel prevents remaining calls and lets the active calls finish', async () => {
  const gate = deferred(); let calls = 0;
  const runner = runnerWith(async input => { calls++; await gate.promise; return reply(input) });
  const running = runner.start(accounts, settings());
  assert.equal(calls, 2);
  runner.stop();
  assert.equal(runner.cancelled.value, 7);
  assert.equal(runner.running.value, true);
  gate.resolve(); await running;
  assert.equal(calls, 2);
  assert.equal(runner.done.value, 2);
  assert.equal(runner.active.value, 0);
  assert.equal(runner.running.value, false);
});

test('capacity errors are recorded once without automatic retries', async () => {
  let calls = 0;
  const runner = runnerWith(async input => { calls++; return { ...reply(input), result: { input, outcome: 'call_error', errorCode: 'capacity', error: 'account limit' } } });
  await runner.start(accounts.slice(0, 1), settings({ repetitions: 1 }));
  assert.equal(calls, 1);
  assert.equal(runner.queue.value[0].result.error, 'account limit');
});

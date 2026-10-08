<script setup lang="ts">
import type { Probe, Rule } from './types'
const props = defineProps<{ probes: Probe[], disabled: boolean }>()
const emit = defineEmits<{ change: [probes: Probe[]] }>()
const rules: { value: Rule, label: string }[] = [
  { value: 'exact', label: '精确匹配' }, { value: 'contains', label: '包含内容' },
  { value: 'regex', label: '正则匹配' }, { value: 'manual', label: '人工查看' },
]
function add() { emit('change', [...props.probes, { id: crypto.randomUUID(), name: '新题目', prompt: '', expected: '', rule: 'exact', enabled: true }]) }
function remove(id: string) { emit('change', props.probes.filter(probe => probe.id !== id)) }
</script>

<template>
  <section class="panel">
    <div class="section-toolbar"><span>{{ probes.length }} / 20 道题目</span><button :disabled="disabled || probes.length >= 20" @click="add">新增题目</button></div>
    <p class="muted">精确匹配只忽略首尾空白，糖果题的预期答案为 21</p>
    <fieldset v-for="(probe, index) in probes" :key="probe.id" class="probe" :disabled="disabled">
      <legend>题目 {{ index + 1 }}</legend>
      <div class="probe-toolbar">
        <label class="inline"><input v-model="probe.enabled" type="checkbox">启用</label>
        <label class="grow">名称<input v-model="probe.name" maxlength="128"></label>
        <button class="danger ghost" :disabled="probes.length === 1" @click="remove(probe.id)">删除</button>
      </div>
      <label>提示词<textarea v-model="probe.prompt" rows="6" placeholder="输入要发送给模型的完整题目" /></label>
      <div class="grid two">
        <label>判定规则<select v-model="probe.rule"><option v-for="rule in rules" :key="rule.value" :value="rule.value">{{ rule.label }}</option></select></label>
        <label>预期答案 / 正则<input v-model="probe.expected" :disabled="probe.rule === 'manual'" placeholder="例如 21"></label>
      </div>
    </fieldset>
  </section>
</template>

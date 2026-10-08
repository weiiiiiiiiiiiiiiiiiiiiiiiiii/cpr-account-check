import type { Outcome } from './types'
export const outcomeLabels: Record<Outcome, string> = { passed: '通过', wrong_answer: '答案错误', format_error: '格式错误', call_error: '调用错误', manual_review: '待人工查看' }

//! 锁定指定账号执行模型测试，并关闭超时的受管模型流

use crate::domain::{Outcome, RunInput, RunResult, judge};
use gateway_plugin_sdk::{
    ErrorCode, PluginFault,
    call::{
        data::{AccountFacts, AccountFactsQuery},
        host::{
            ModelEventBatch, ModelExecuteRequest, ModelOperation, ModelStreamReadRequest,
            ModelStreamReadResult, ModelStreamResult,
        },
        model::{CanonicalEvent, FinishReason},
    },
    client::{HostClient, SessionError},
};
use serde_json::json;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub async fn accounts(host: &HostClient) -> Result<Vec<AccountFacts>, PluginFault> {
    let mut accounts = Vec::new();
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    loop {
        let page = host
            .account_facts(AccountFactsQuery {
                provider_id: None,
                cursor,
                limit: 200,
            })
            .await?;
        accounts.extend(page.accounts);
        cursor = page.next_cursor;
        match &cursor {
            None => return Ok(accounts),
            Some(next) if seen.insert(next.clone()) => {}
            _ => return Err(PluginFault::new(ErrorCode::Fault, "账号分页游标重复")),
        }
    }
}

pub async fn run(host: &HostClient, input: RunInput, parent_timeout_ms: u64) -> RunResult {
    let started = Instant::now();
    let mut result = RunResult {
        input,
        account_name: String::new(),
        provider: String::new(),
        selection_mode: "host_required_account".into(),
        outcome: Outcome::CallError,
        answer: String::new(),
        error_code: None,
        error: None,
        request_id: None,
        response_model: None,
        elapsed_ms: 0,
        tested_at_ms: now_ms(),
        answer_truncated: false,
    };
    let mut stream = None;
    // 为流关闭和持久化保留时间，不能让用户设置超过当前父调用期限
    let budget = Duration::from_millis(
        (result.input.timeout_seconds * 1000).min(parent_timeout_ms.saturating_sub(5000)),
    );
    let execution = tokio::time::timeout(budget, execute(host, &mut result, &mut stream)).await;
    let failure = match execution {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error),
        Err(_) => Some(PluginFault::new(ErrorCode::Timeout, "测试超时")),
    };
    if let Some(stream) = stream {
        // 超时或解析失败仍关闭子请求，不把失去消费的流留到后续测试
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            host.call(
                "host.model.stream_close",
                json!({ "stream": stream }),
                Vec::new(),
            ),
        )
        .await;
    }
    if let Some(error) = failure {
        result.error_code = Some(
            serde_json::to_value(error.code)
                .unwrap_or_default()
                .as_str()
                .unwrap_or("fault")
                .into(),
        );
        result.error = Some(match error.code {
            ErrorCode::Timeout => "测试超时".into(),
            ErrorCode::Cancelled => "测试被取消".into(),
            ErrorCode::PermissionDenied => "Client Key 无权访问该账号或模型".into(),
            ErrorCode::Capacity if error.message.trim().is_empty() => {
                "并发或额度限制，暂时无法调用".into()
            }
            _ => error.message.chars().take(500).collect(),
        });
    }
    result.elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    result
}

async fn execute(
    host: &HostClient,
    result: &mut RunResult,
    stream: &mut Option<String>,
) -> Result<(), PluginFault> {
    let account = accounts(host)
        .await?
        .into_iter()
        .find(|account| account.account_id == result.input.account_id)
        .ok_or_else(|| PluginFault::new(ErrorCode::Rejected, "账号已删除，请刷新账号列表"))?;
    result.account_name = account.name;
    result.provider = account.provider_id;
    if !account.enabled {
        return Err(PluginFault::new(ErrorCode::Rejected, "账号已停用"));
    }
    let input = &result.input;
    let request = ModelExecuteRequest {
        client_key_id: Some(input.client_key_id.clone()),
        model: input.model.clone(),
        protocol: "openai".into(),
        operation: ModelOperation::Generate,
        provider: Some(result.provider.clone()),
        account_id: Some(input.account_id.clone()),
        previous_response_id: None,
    };
    let mut body = json!({ "model": input.model, "input": input.probe.prompt, "stream": true, "store": false });
    if input.reasoning != "default" {
        body["reasoning"] = json!({ "effort": input.reasoning });
    }
    let reply = host
        .call(
            "host.model.execute_stream",
            serde_json::to_value(request).map_err(encoding)?,
            serde_json::to_vec(&body).map_err(encoding)?,
        )
        .await
        .map_err(SessionError::into_plugin_fault)?;
    let opened: ModelStreamResult = serde_json::from_value(reply.result).map_err(encoding)?;
    result.request_id = Some(opened.request_id);
    *stream = Some(opened.stream.clone());
    let mut completed = None;
    let mut total_answer_bytes = 0usize;
    loop {
        let request = ModelStreamReadRequest {
            stream: opened.stream.clone(),
            maximum_bytes: 1024 * 1024,
        };
        let reply = host
            .call(
                "host.model.stream_read",
                serde_json::to_value(request).map_err(encoding)?,
                Vec::new(),
            )
            .await
            .map_err(SessionError::into_plugin_fault)?;
        let read: ModelStreamReadResult = serde_json::from_value(reply.result).map_err(encoding)?;
        let events = if reply.payload.is_empty() {
            Vec::new()
        } else {
            ModelEventBatch::decode(&reply.payload)
                .map_err(|_| PluginFault::new(ErrorCode::Fault, "模型事件解码失败"))?
                .events
        };
        if events.len() != read.events as usize {
            return Err(PluginFault::new(ErrorCode::Fault, "模型事件数量不一致"));
        }
        for event in events {
            for fact in event.facts {
                match fact {
                    CanonicalEvent::TextDelta { text, .. } => {
                        total_answer_bytes = total_answer_bytes.saturating_add(text.len());
                        let remaining = 8192usize.saturating_sub(result.answer.len());
                        let mut end = text.len().min(remaining);
                        while !text.is_char_boundary(end) {
                            end -= 1;
                        }
                        result.answer.push_str(&text[..end]);
                        result.answer_truncated = total_answer_bytes > 8192;
                    }
                    CanonicalEvent::Completed { model, reason, .. } => {
                        result.response_model = model;
                        completed = Some(reason);
                    }
                    _ => {}
                }
            }
        }
        if read.end {
            *stream = None;
            break;
        }
    }
    if completed != Some(FinishReason::Stop) {
        return Err(PluginFault::new(
            ErrorCode::Upstream,
            "模型响应未正常完成，无法判定答案",
        ));
    }
    if result.answer_truncated {
        return Err(PluginFault::new(
            ErrorCode::Capacity,
            "回答超过保存长度，无法完整判分",
        ));
    }
    result.outcome = judge(&result.input.probe, &result.answer)
        .map_err(|message| PluginFault::new(ErrorCode::InvalidInput, message))?;
    Ok(())
}

fn encoding(_: serde_json::Error) -> PluginFault {
    PluginFault::new(ErrorCode::Fault, "模型调用数据无效")
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

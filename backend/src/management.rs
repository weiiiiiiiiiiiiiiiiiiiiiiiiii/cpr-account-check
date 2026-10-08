//! 注册管理页面和题库、账号、测试、历史接口

use crate::{
    domain::{RunInput, Settings, bounded},
    execution, storage,
};
use gateway_plugin_sdk::{
    ErrorCode, PluginFault,
    call::{
        host::{KeyListRequest, ModelListRequest, ModelListResult},
        management::{
            ManagementPage, ManagementRegistration, ManagementRequest, ManagementResource,
            ManagementResponse, ManagementRoute,
        },
    },
    client::{SessionError, TypedCall, TypedReply},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use tokio::sync::Semaphore;

pub struct AppState {
    capacity: Arc<Semaphore>,
    in_flight: Arc<Mutex<BTreeSet<String>>>,
    history_writer: tokio::sync::Mutex<()>,
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            capacity: Arc::new(Semaphore::new(4)),
            in_flight: Arc::new(Mutex::new(BTreeSet::new())),
            history_writer: tokio::sync::Mutex::new(()),
        }
    }
}
struct RunGuard {
    id: String,
    registry: Arc<Mutex<BTreeSet<String>>>,
}
impl Drop for RunGuard {
    fn drop(&mut self) {
        self.registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.id);
    }
}
impl AppState {
    fn reserve(&self, id: &str) -> Result<RunGuard, PluginFault> {
        if !self
            .in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id.into())
        {
            return Err(PluginFault::new(
                ErrorCode::Conflict,
                "该测试已在执行，请勿重复提交",
            ));
        }
        Ok(RunGuard {
            id: id.into(),
            registry: Arc::clone(&self.in_flight),
        })
    }
}

pub fn registration() -> ManagementRegistration {
    ManagementRegistration {
        routes: [
            ("GET", "api/snapshot"),
            ("GET", "api/history"),
            ("POST", "api/settings"),
            ("POST", "api/models"),
            ("POST", "api/run"),
        ]
        .into_iter()
        .map(|(method, path)| ManagementRoute {
            method: method.into(),
            path: path.into(),
            request_content_types: if method == "GET" {
                Vec::new()
            } else {
                vec!["application/json".into()]
            },
            response_content_types: vec!["application/json".into()],
        })
        .collect(),
        resources: ["web/index.html", "web/app.js", "web/app.css"]
            .into_iter()
            .map(|path| ManagementResource {
                path: path.into(),
                public: false,
            })
            .collect(),
        pages: vec![ManagementPage {
            id: "account-check".into(),
            title: "账号能力检测".into(),
            description: Some("编辑题库，测试指定账号并对比历史结果".into()),
            entry: "web/index.html".into(),
            icon: None,
        }],
        callbacks: Vec::new(),
    }
}

pub async fn handle(
    call: TypedCall<ManagementRequest>,
    state: Arc<AppState>,
) -> Result<TypedReply<ManagementResponse>, PluginFault> {
    match route(&call, state).await {
        Ok(reply) => Ok(reply),
        Err(error) => {
            let status = match error.code {
                ErrorCode::InvalidInput => 400,
                ErrorCode::Conflict => 409,
                ErrorCode::Capacity => 429,
                _ => 502,
            };
            reply(
                status,
                &json!({ "error": { "code": error.code, "message": error.message } }),
            )
        }
    }
}

async fn route(
    call: &TypedCall<ManagementRequest>,
    state: Arc<AppState>,
) -> Result<TypedReply<ManagementResponse>, PluginFault> {
    if !call.request.query.is_empty() || call.payload.len() > 256 * 1024 {
        return Err(invalid("查询参数或正文长度无效"));
    }
    if call.request.method == "GET" && !call.payload.is_empty() {
        return Err(invalid("GET 接口不接受正文"));
    }
    match (call.request.method.as_str(), call.request.path.as_str()) {
        ("GET", "api/snapshot") => {
            let accounts = execution::accounts(&call.host).await?;
            let mut keys = Vec::new();
            let mut cursor = None;
            let mut seen = BTreeSet::new();
            loop {
                let page = call
                    .host
                    .list_keys(KeyListRequest { cursor, limit: 200 })
                    .await?;
                keys.extend(page.keys);
                cursor = page.next_cursor;
                match &cursor {
                    None => break,
                    Some(next) if seen.insert(next.clone()) => {}
                    _ => return Err(PluginFault::new(ErrorCode::Fault, "Key 分页游标重复")),
                }
            }
            let (version, settings) = storage::settings(&call.host).await?;
            reply(
                200,
                &json!({ "accounts": accounts, "keys": keys, "version": version, "settings": settings }),
            )
        }
        ("GET", "api/history") => reply(200, &storage::history(&call.host).await?),
        ("POST", "api/settings") => {
            let request: SaveSettings = decode(call)?;
            request.value.validate().map_err(invalid)?;
            let version = storage::put(
                &call.host,
                "settings",
                request.expected_version,
                &request.value,
            )
            .await?;
            reply(200, &json!({ "version": version }))
        }
        ("POST", "api/models") => {
            let request: Models = decode(call)?;
            if !bounded(&request.client_key_id, 1, 256) {
                return Err(invalid("请选择 Client Key"));
            }
            let request = ModelListRequest {
                client_key_id: request.client_key_id,
                protocol: "openai".into(),
                client_version: "account-check/0.1.0".into(),
            };
            let response = call
                .host
                .call(
                    "host.models.list",
                    serde_json::to_value(request).map_err(encoding)?,
                    Vec::new(),
                )
                .await
                .map_err(SessionError::into_plugin_fault)?;
            let models: ModelListResult =
                serde_json::from_value(response.result).map_err(encoding)?;
            reply(200, &models)
        }
        ("POST", "api/run") => {
            let input: RunInput = decode(call)?;
            input.validate().map_err(invalid)?;
            let _permit = Arc::clone(&state.capacity)
                .try_acquire_owned()
                .map_err(|_| {
                    PluginFault::new(ErrorCode::Capacity, "已有 4 个测试正在执行，请稍后再试")
                })?;
            let _guard = state.reserve(&input.run_id)?;
            if let Some(existing) = storage::find(&call.host, &input.run_id).await? {
                if serde_json::to_value(&existing.input).map_err(encoding)?
                    != serde_json::to_value(&input).map_err(encoding)?
                {
                    return Err(PluginFault::new(
                        ErrorCode::Conflict,
                        "测试 ID 已被其他参数使用",
                    ));
                }
                return reply(
                    200,
                    &json!({ "result": existing, "saved": true, "saveError": null }),
                );
            }
            let result = execution::run(&call.host, input, call.context.timeout_ms).await;
            let _history_writer = state.history_writer.lock().await;
            let saved = storage::append(&call.host, &result).await;
            reply(
                200,
                &json!({ "result": result, "saved": saved.is_ok(), "saveError": saved.err().map(|error| error.message) }),
            )
        }
        _ => reply(
            404,
            &json!({ "error": { "message": "未找到插件管理接口" } }),
        ),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveSettings {
    expected_version: Option<u64>,
    value: Settings,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Models {
    client_key_id: String,
}

fn decode<T: DeserializeOwned>(call: &TypedCall<ManagementRequest>) -> Result<T, PluginFault> {
    if call
        .request
        .content_type
        .as_deref()
        .is_none_or(|value| value.split(';').next().unwrap_or("").trim() != "application/json")
    {
        return Err(invalid("接口只接受 JSON 正文"));
    }
    serde_json::from_slice(&call.payload).map_err(|_| invalid("请求 JSON 无效"))
}
fn invalid(message: impl Into<String>) -> PluginFault {
    PluginFault::new(ErrorCode::InvalidInput, message)
}
fn encoding(_: serde_json::Error) -> PluginFault {
    PluginFault::new(ErrorCode::Fault, "管理数据编码失败")
}
fn reply(
    status: u16,
    body: &impl Serialize,
) -> Result<TypedReply<ManagementResponse>, PluginFault> {
    Ok(TypedReply::new(ManagementResponse {
        status,
        content_type: "application/json".into(),
        headers: Vec::new(),
    })
    .with_payload(serde_json::to_vec(body).map_err(encoding)?))
}

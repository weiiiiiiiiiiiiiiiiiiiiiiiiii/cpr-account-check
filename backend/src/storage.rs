//! 插件私有状态与有界历史环形记录，单条值不越过协议元数据上限

use crate::domain::{RunResult, Settings};
use gateway_plugin_sdk::{
    ErrorCode, PluginFault,
    call::host::{StateGetRequest, StateGetResult, StatePutRequest, StatePutResult},
    client::{HostClient, SessionError},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub async fn get<T: DeserializeOwned>(
    host: &HostClient,
    namespace: &str,
) -> Result<(Option<u64>, Option<T>), PluginFault> {
    get_at(host, namespace, "value").await
}

async fn get_at<T: DeserializeOwned>(
    host: &HostClient,
    namespace: &str,
    key: &str,
) -> Result<(Option<u64>, Option<T>), PluginFault> {
    let request = StateGetRequest {
        namespace: namespace.into(),
        key: key.into(),
    };
    let reply = host
        .call(
            "host.state.get",
            serde_json::to_value(request).map_err(encoding)?,
            Vec::new(),
        )
        .await
        .map_err(SessionError::into_plugin_fault)?;
    let result: StateGetResult = serde_json::from_value(reply.result).map_err(encoding)?;
    match result.record {
        Some(record) if record.schema_version == 1 => Ok((
            Some(record.version),
            Some(serde_json::from_value(record.value).map_err(encoding)?),
        )),
        Some(_) => Err(PluginFault::new(ErrorCode::Fault, "私有状态版本不受支持")),
        None => Ok((None, None)),
    }
}

pub async fn put<T: Serialize>(
    host: &HostClient,
    namespace: &str,
    version: Option<u64>,
    value: &T,
) -> Result<u64, PluginFault> {
    put_at(host, namespace, "value", version, value).await
}

async fn put_at<T: Serialize>(
    host: &HostClient,
    namespace: &str,
    key: &str,
    version: Option<u64>,
    value: &T,
) -> Result<u64, PluginFault> {
    if serde_json::to_vec(value).map_err(encoding)?.len() > 48 * 1024 {
        return Err(PluginFault::new(ErrorCode::Capacity, "记录超过保存容量"));
    }
    let request = StatePutRequest {
        namespace: namespace.into(),
        key: key.into(),
        expected_version: version,
        value: serde_json::to_value(value).map_err(encoding)?,
    };
    let reply = host
        .call(
            "host.state.put",
            serde_json::to_value(request).map_err(encoding)?,
            Vec::new(),
        )
        .await
        .map_err(SessionError::into_plugin_fault)?;
    let result: StatePutResult = serde_json::from_value(reply.result).map_err(encoding)?;
    Ok(result.version)
}

#[derive(Default, Deserialize, Serialize)]
pub struct History {
    pub entries: Vec<RunResult>,
}

#[derive(Default, Deserialize, Serialize)]
struct HistoryIndex {
    next_slot: u8,
    entries: Vec<HistoryEntry>,
}
#[derive(Deserialize, Serialize)]
struct HistoryEntry {
    slot: u8,
    run_id: String,
}

async fn index(host: &HostClient) -> Result<(Option<u64>, HistoryIndex), PluginFault> {
    let (version, index) = get_at::<HistoryIndex>(host, "history", "index").await?;
    let index = index.unwrap_or_default();
    if index.next_slot >= 100
        || index.entries.len() > 100
        || index.entries.iter().any(|entry| entry.slot >= 100)
    {
        return Err(PluginFault::new(ErrorCode::Fault, "历史索引无效"));
    }
    Ok((version, index))
}

async fn entry(
    host: &HostClient,
    reference: &HistoryEntry,
) -> Result<Option<RunResult>, PluginFault> {
    let (_, result) =
        get_at::<RunResult>(host, "history", &format!("slot-{}", reference.slot)).await?;
    // 读取期间槽位可能更新，旧索引绝不能把新测试记到另一次测试上
    Ok(result.filter(|result| result.input.run_id == reference.run_id))
}

pub async fn history(host: &HostClient) -> Result<History, PluginFault> {
    let (_, index) = index(host).await?;
    let mut history = History::default();
    for reference in &index.entries {
        if let Some(result) = entry(host, reference).await? {
            history.entries.push(result);
        }
    }
    Ok(history)
}

pub async fn find(host: &HostClient, run_id: &str) -> Result<Option<RunResult>, PluginFault> {
    let (_, index) = index(host).await?;
    match index.entries.iter().find(|entry| entry.run_id == run_id) {
        Some(reference) => entry(host, reference).await,
        None => Ok(None),
    }
}

pub async fn append(host: &HostClient, result: &RunResult) -> Result<(), PluginFault> {
    // 调用方串行化写入，固定 100 个槽位，不因取消或进程退出遗留无限孤立记录
    // CAS 冲突只重试状态提交，不重新执行模型
    for _ in 0..8 {
        let (version, mut index) = index(host).await?;
        let slot = index.next_slot;
        let key = format!("slot-{slot}");
        let (slot_version, _) = get_at::<RunResult>(host, "history", &key).await?;
        put_at(host, "history", &key, slot_version, result).await?;
        index
            .entries
            .retain(|entry| entry.slot != slot && entry.run_id != result.input.run_id);
        index.entries.insert(
            0,
            HistoryEntry {
                slot,
                run_id: result.input.run_id.clone(),
            },
        );
        index.entries.truncate(100);
        index.next_slot = (slot + 1) % 100;
        match put_at(host, "history", "index", version, &index).await {
            Ok(_) => return Ok(()),
            Err(error) if error.code == ErrorCode::Conflict => continue,
            Err(error) => return Err(error),
        }
    }
    Err(PluginFault::new(
        ErrorCode::Conflict,
        "历史记录写入冲突，请导出本次结果",
    ))
}

pub async fn settings(host: &HostClient) -> Result<(Option<u64>, Settings), PluginFault> {
    let (version, value) = get::<Settings>(host, "settings").await?;
    Ok((version, value.unwrap_or_default()))
}

fn encoding(_: serde_json::Error) -> PluginFault {
    PluginFault::new(ErrorCode::Fault, "私有状态数据无效")
}

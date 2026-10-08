//! 启动插件会话，协议输出只交给 SDK

mod domain;
mod execution;
mod management;
mod storage;

use gateway_plugin_sdk::client::{PluginBuilder, PluginSession, SessionConfig, SessionError};
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = PluginBuilder::from_json(include_bytes!("../../plugin.json"))?;
    let session = PluginSession::accept(
        tokio::io::stdin(),
        tokio::io::stdout(),
        SessionConfig::default(),
    )
    .await?;
    let manifest =
        gateway_plugin_sdk::Manifest::from_author_slice(include_bytes!("../../plugin.json"))?;
    if session.handshake().plugin_id != "account-lab.cognition-check"
        || session.handshake().contributes != manifest.contributes
        || !session
            .handshake()
            .configuration
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
    {
        return Err(SessionError::Handshake.into());
    }
    let state = Arc::new(management::AppState::default());
    let plugin = builder
        .management(management::registration(), move |call| {
            let state = Arc::clone(&state);
            async move { management::handle(call, state).await }
        })?
        .build()?;
    session.run(plugin).await?;
    Ok(())
}

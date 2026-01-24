use crate::application::ui::UiAssembler;
use crate::client::McpClient;
use crate::config::AppConfig;
use crate::model::ModelProvider;
use std::sync::Arc;

pub(crate) struct ServerState<P: ModelProvider> {
    client: Arc<McpClient<P>>,
    pub(crate) ui_assembler: Arc<UiAssembler>,
}

impl<P: ModelProvider> ServerState<P> {
    pub(crate) fn new(client: Arc<McpClient<P>>, config: &AppConfig) -> Self {
        let ui_assembler = Arc::new(UiAssembler::new(config.ui.clone()));
        Self {
            client,
            ui_assembler,
        }
    }

    pub(crate) fn client(&self) -> Arc<McpClient<P>> {
        Arc::clone(&self.client)
    }
}

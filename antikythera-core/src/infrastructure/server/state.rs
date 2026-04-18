use crate::application::client::McpClient;
use crate::infrastructure::model::ModelProvider;
use std::sync::Arc;

pub(crate) struct ServerState<P: ModelProvider + Send + Sync + 'static> {
    client: Arc<McpClient<P>>,
}

impl<P: ModelProvider + Send + Sync + 'static> ServerState<P> {
    pub(crate) fn new(client: Arc<McpClient<P>>) -> Self {
        Self {
            client,
        }
    }

    pub(crate) fn client(&self) -> Arc<McpClient<P>> {
        Arc::clone(&self.client)
    }
}

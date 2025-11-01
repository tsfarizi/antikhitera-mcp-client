use crate::client::McpClient;
use crate::model::ModelProvider;
use std::sync::Arc;

pub(crate) struct ServerState<P: ModelProvider> {
    client: Arc<McpClient<P>>,
}

impl<P: ModelProvider> ServerState<P> {
    pub(crate) fn new(client: Arc<McpClient<P>>) -> Self {
        Self { client }
    }

    pub(crate) fn client(&self) -> Arc<McpClient<P>> {
        Arc::clone(&self.client)
    }
}

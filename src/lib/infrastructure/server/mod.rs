mod docs;
mod dto;
mod error;
mod router;
mod routes;
mod state;

pub use error::ServerError;
pub(crate) use state::ServerState;

use crate::client::McpClient;
use crate::model::ModelProvider;
use std::net::SocketAddr;
use std::sync::Arc;

pub async fn serve<P>(client: Arc<McpClient<P>>, addr: SocketAddr) -> Result<(), ServerError>
where
    P: ModelProvider + 'static,
{
    router::serve(client, addr).await
}

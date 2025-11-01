use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("failed to bind HTTP listener on {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("HTTP server error: {0}")]
    Serve(#[from] std::io::Error),
}

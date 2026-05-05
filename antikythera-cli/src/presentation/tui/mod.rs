pub mod app;
pub mod event_loop;
pub(crate) mod handlers;
pub(crate) mod render;
pub mod types;

pub use event_loop::run_chat_app;

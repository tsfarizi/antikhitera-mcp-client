//! Wasmtime module runner implementation.

use std::sync::Arc;

use thiserror::Error;
use tracing::{debug, warn};
use wasmtime::{Caller, Engine, Extern, Linker, Module, Store};

use crate::application::agent::multi_agent::task::{AgentTask, TaskResult};

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, Error)]
pub enum WasmRuntimeError {
    #[error("wasmtime error: {0}")]
    Engine(#[from] anyhow::Error),
    #[error("missing required WASM export: '{0}'")]
    MissingExport(&'static str),
    #[error("task serialisation error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("WASM execution failed: {0}")]
    Execution(String),
    #[error("I/O error loading module: {0}")]
    Io(#[from] std::io::Error),
}

// ============================================================================
// Host state
// ============================================================================

/// State threaded through the wasmtime `Store` during a single task run.
struct HostState {}

// ============================================================================
// WasmAgentRunner
// ============================================================================

/// Runs a pre-compiled WASM agent module as a sandboxed [`AgentTask`] executor.
///
/// # Cloneability
///
/// Both `Engine` and `Module` use internal `Arc` reference counting, making
/// `WasmAgentRunner` cheap to clone and share across threads.
///
/// # Example
///
/// ```rust,no_run
/// use antikythera_core::infrastructure::wasm::WasmAgentRunner;
/// use antikythera_core::application::agent::multi_agent::task::AgentTask;
/// use std::sync::Arc;
///
/// # async fn run() -> anyhow::Result<()> {
/// let wasm_bytes = std::fs::read("my_agent.wasm")?;
/// let runner = WasmAgentRunner::from_bytes("my-agent", &wasm_bytes)?;
///
/// let handler = Arc::new(|_req: String| -> String {
///     r#"{"content": "stub response", "model": "stub"}"#.to_string()
/// });
///
/// let task = AgentTask::new("hello");
/// let result = runner.run_task(task, handler).await;
/// println!("{}", result.success);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct WasmAgentRunner {
    agent_id: String,
    engine: Engine,
    module: Module,
}

impl WasmAgentRunner {
    // ----------------------------------------------------------------
    // Constructors
    // ----------------------------------------------------------------

    /// Load a WASM agent from raw bytes.
    pub fn from_bytes(
        agent_id: impl Into<String>,
        wasm: &[u8],
    ) -> Result<Self, WasmRuntimeError> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm)?;
        Ok(Self { agent_id: agent_id.into(), engine, module })
    }

    /// Load a WASM agent from a file on disk.
    pub fn from_file(
        agent_id: impl Into<String>,
        path: &std::path::Path,
    ) -> Result<Self, WasmRuntimeError> {
        let wasm = std::fs::read(path)?;
        Self::from_bytes(agent_id, &wasm)
    }

    /// The agent ID this runner was created with.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    // ----------------------------------------------------------------
    // Task execution
    // ----------------------------------------------------------------

    /// Execute an [`AgentTask`] inside the WASM sandbox.
    ///
    /// `llm_handler` is a **blocking** function that accepts a JSON-encoded
    /// LLM request string and returns a JSON-encoded response string.  It is
    /// called from within a `tokio::task::block_in_place` context.
    ///
    /// The entire WASM execution runs inside `tokio::task::spawn_blocking` so
    /// the calling async task is not blocked while the guest runs.
    ///
    /// # Panics
    ///
    /// Panics if called outside of a `rt-multi-thread` tokio runtime.
    pub async fn run_task<F>(&self, task: AgentTask, llm_handler: Arc<F>) -> TaskResult
    where
        F: Fn(String) -> String + Send + Sync + 'static,
    {
        let runner = self.clone();
        let task_clone = task.clone();

        let result = tokio::task::spawn_blocking(move || {
            runner.run_task_sync(task_clone, llm_handler)
        })
        .await;

        match result {
            Ok(r) => r,
            Err(e) => TaskResult::failure(
                task.task_id,
                self.agent_id.clone(),
                format!("spawn_blocking panicked: {e}"),
            ),
        }
    }

    // ----------------------------------------------------------------
    // Internal synchronous execution
    // ----------------------------------------------------------------

    fn run_task_sync<F>(&self, task: AgentTask, llm_handler: Arc<F>) -> TaskResult
    where
        F: Fn(String) -> String + Send + Sync + 'static,
    {
        let state = HostState {};
        let mut store = Store::new(&self.engine, state);

        let mut linker: Linker<HostState> = Linker::new(&self.engine);

        // ----------------------------------------------------------------
        // Register the `call_llm_sync` host import
        // ----------------------------------------------------------------
        if let Err(e) = linker.func_wrap(
            "antikythera",
            "call_llm_sync",
            move |mut caller: Caller<'_, HostState>, req_ptr: i32, req_len: i32| -> i64 {
                // Read request JSON from WASM memory
                let req_json = match Self::read_wasm_string(&mut caller, req_ptr, req_len) {
                    Some(s) => s,
                    None => return -1,
                };

                debug!(req_len = req_len, "WASM agent calling LLM");
                let response = (llm_handler)(req_json);
                let response_bytes = response.into_bytes();
                let response_len = response_bytes.len() as i32;

                // Allocate space in WASM memory for the response
                let alloc_fn =
                    match caller.get_export("antikythera_alloc").and_then(|e| {
                        if let Extern::Func(f) = e { Some(f) } else { None }
                    }) {
                        Some(f) => f,
                        None => return -2,
                    };

                // Split typed-check from call to avoid simultaneous &/&mut borrow
                let typed_alloc = match alloc_fn.typed::<i32, i32>(&caller) {
                    Ok(t) => t,
                    Err(_) => return -3,
                };
                let result_ptr = match typed_alloc.call(&mut caller, response_len) {
                    Ok(ptr) => ptr,
                    Err(_) => return -3,
                };

                // Write the response into WASM memory
                if let Err(()) =
                    Self::write_wasm_bytes(&mut caller, result_ptr, &response_bytes)
                {
                    return -4;
                }

                ((result_ptr as i64) << 32) | response_len as i64
            },
        ) {
            warn!(error = %e, "Failed to register call_llm_sync host function");
            return TaskResult::failure(
                task.task_id,
                self.agent_id.clone(),
                format!("Linker error: {e}"),
            );
        }

        // ----------------------------------------------------------------
        // Instantiate the module
        // ----------------------------------------------------------------
        let instance = match linker.instantiate(&mut store, &self.module) {
            Ok(i) => i,
            Err(e) => {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    format!("WASM instantiation failed: {e}"),
                )
            }
        };

        // ----------------------------------------------------------------
        // Serialise the task to JSON
        // ----------------------------------------------------------------
        let task_json = match serde_json::to_string(&task) {
            Ok(j) => j,
            Err(e) => {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    format!("Failed to serialise task: {e}"),
                )
            }
        };
        let task_bytes = task_json.as_bytes();
        let task_len = task_bytes.len() as i32;

        // ----------------------------------------------------------------
        // Allocate space in WASM memory for the task JSON
        // ----------------------------------------------------------------
        let alloc = match instance.get_typed_func::<i32, i32>(&mut store, "antikythera_alloc") {
            Ok(f) => f,
            Err(_) => {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    "WASM module missing 'antikythera_alloc' export".to_string(),
                )
            }
        };

        let task_ptr = match alloc.call(&mut store, task_len) {
            Ok(p) => p,
            Err(e) => {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    format!("WASM alloc failed: {e}"),
                )
            }
        };

        // Write task JSON into WASM memory
        if let Some(Extern::Memory(memory)) = instance.get_export(&mut store, "memory") {
            let wasm_mem = memory.data_mut(&mut store);
            let start = task_ptr as usize;
            let end = start + task_bytes.len();
            if end > wasm_mem.len() {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    "WASM memory too small for task input".to_string(),
                );
            }
            wasm_mem[start..end].copy_from_slice(task_bytes);
        } else {
            return TaskResult::failure(
                task.task_id,
                self.agent_id.clone(),
                "WASM module missing 'memory' export".to_string(),
            );
        }

        // ----------------------------------------------------------------
        // Call `antikythera_run`
        // ----------------------------------------------------------------
        let run_fn =
            match instance.get_typed_func::<(i32, i32), i64>(&mut store, "antikythera_run") {
                Ok(f) => f,
                Err(_) => {
                    return TaskResult::failure(
                        task.task_id,
                        self.agent_id.clone(),
                        "WASM module missing 'antikythera_run' export".to_string(),
                    )
                }
            };

        let packed = match run_fn.call(&mut store, (task_ptr, task_len)) {
            Ok(v) => v,
            Err(e) => {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    format!("WASM execution error: {e}"),
                )
            }
        };

        if packed < 0 {
            return TaskResult::failure(
                task.task_id,
                self.agent_id.clone(),
                format!("WASM agent returned error code: {packed}"),
            );
        }

        let result_ptr = ((packed as u64) >> 32) as usize;
        let result_len = (packed as u32) as usize;

        // ----------------------------------------------------------------
        // Read the result JSON from WASM memory
        // ----------------------------------------------------------------
        let result_bytes = if let Some(Extern::Memory(memory)) =
            instance.get_export(&mut store, "memory")
        {
            let mem = memory.data(&store);
            if result_ptr + result_len > mem.len() {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    "WASM result pointer out of bounds".to_string(),
                );
            }
            mem[result_ptr..result_ptr + result_len].to_vec()
        } else {
            return TaskResult::failure(
                task.task_id,
                self.agent_id.clone(),
                "WASM module missing 'memory' export (on result read)".to_string(),
            );
        };

        // Free the result memory if dealloc is available
        if let Ok(dealloc) =
            instance.get_typed_func::<(i32, i32), ()>(&mut store, "antikythera_dealloc")
        {
            let _ = dealloc.call(&mut store, (result_ptr as i32, result_len as i32));
        }

        // ----------------------------------------------------------------
        // Deserialise the result
        // ----------------------------------------------------------------
        let result_str = match std::str::from_utf8(&result_bytes) {
            Ok(s) => s,
            Err(e) => {
                return TaskResult::failure(
                    task.task_id,
                    self.agent_id.clone(),
                    format!("WASM result is not valid UTF-8: {e}"),
                )
            }
        };

        // Try to parse as a full TaskResult; fall back to wrapping raw string
        match serde_json::from_str::<TaskResult>(result_str) {
            Ok(r) => r,
            Err(_) => {
                // The module returned raw text — wrap it in a successful TaskResult
                TaskResult::success(
                    task.task_id,
                    self.agent_id.clone(),
                    serde_json::Value::String(result_str.to_string()),
                    0,
                    task.session_id.unwrap_or_default(),
                )
            }
        }
    }

    // ----------------------------------------------------------------
    // Memory helpers
    // ----------------------------------------------------------------

    fn read_wasm_string(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Option<String> {
        let memory = caller.get_export("memory").and_then(|e| {
            if let Extern::Memory(m) = e { Some(m) } else { None }
        })?;
        // Reborrow as immutable after get_export's &mut borrow has ended
        let data = memory.data(&*caller);
        let start = ptr as usize;
        let end = start + len as usize;
        if end > data.len() {
            return None;
        }
        std::str::from_utf8(&data[start..end]).ok().map(|s| s.to_string())
    }

    fn write_wasm_bytes(
        caller: &mut Caller<'_, HostState>,
        ptr: i32,
        bytes: &[u8],
    ) -> Result<(), ()> {
        let memory = caller.get_export("memory").and_then(|e| {
            if let Extern::Memory(m) = e { Some(m) } else { None }
        });
        let memory = memory.ok_or(())?;
        let data = memory.data_mut(caller);
        let start = ptr as usize;
        let end = start + bytes.len();
        if end > data.len() {
            return Err(());
        }
        data[start..end].copy_from_slice(bytes);
        Ok(())
    }
}

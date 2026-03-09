use antikhitera_mcp_client::application::client::{ClientConfig, McpClient};
use antikhitera_mcp_client::application::services::chat::ChatService;
use antikhitera_mcp_client::config::AppConfig;
use antikhitera_mcp_client::infrastructure::model::DynamicModelProvider;
use std::sync::Arc;
use std::time::Instant;
use tokio::task;

#[tokio::test]
async fn test_high_concurrency_execution() {
    let file_config = AppConfig::default();

    if file_config.providers.is_empty() {
        println!("Skipping real integration execution: No providers found in config.");
        return;
    }

    let provider = match DynamicModelProvider::from_configs(&file_config.providers) {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping due to provider init failure: {}", e);
            return;
        }
    };

    let client_config = ClientConfig::new(
        file_config.default_provider.clone(),
        file_config.model.clone(),
    )
    .with_tools(file_config.tools.clone())
    .with_servers(file_config.servers.clone())
    .with_prompts(file_config.prompts.clone())
    .with_providers(file_config.providers.clone());

    let client = Arc::new(McpClient::new(provider, client_config));
    let service = Arc::new(ChatService::new(client));

    let mut tasks = vec![];
    let concurrency_spawns = 15;

    let start_time = Instant::now();
    for i in 0..concurrency_spawns {
        let svc = service.clone();

        // This prompt specifically instructs the model to utilize parallel dispatch JSON logic
        let prompt = format!(
            "Execute a massive search. I need you to invoke exactly 3 tools simultaneously. Use your tools via the `call_tools` array response. Session variant: {}",
            i
        );

        let handle = task::spawn(async move {
            let res = svc
                .process_request(
                    prompt,
                    vec![],
                    None,
                    Some("high-concurrency-shared-session".to_string()),
                    true, // agent enabled
                    Some(1),
                    true, // debug mode
                )
                .await;

            res
        });
        tasks.push(handle);
    }

    let mut success_count = 0;
    for handle in tasks {
        let result = handle.await.expect("Task panicked during execution");
        if let Ok(_) = result {
            success_count += 1;
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let requests_per_sec = (concurrency_spawns as f64) / elapsed;
    println!(
        "Completed concurrency batch in {:.2}s. Throughput: {:.2} req/sec.",
        elapsed, requests_per_sec
    );
    println!(
        "Successful autonomous loops: {} / {}",
        success_count, concurrency_spawns
    );
}

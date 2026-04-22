#[tokio::test]
async fn test_high_concurrency_execution() {
    let file_config = AppConfig::default();

    // Load providers from the postcard binary config (CLI layer owns provider definitions).
    let providers = load_app_config(None)
        .map(|pc| providers_from_postcard(&pc.providers))
        .unwrap_or_default();

    if providers.is_empty() {
        println!("Skipping real integration execution: No providers found in config.");
        return;
    }

    let client = match build_runtime_client(&file_config, &providers) {
        Ok(c) => c,
        Err(e) => {
            println!("Skipping due to provider init failure: {}", e);
            return;
        }
    };
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
                    String::new(),
                    String::new(),
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

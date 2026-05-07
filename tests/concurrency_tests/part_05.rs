#[tokio::test]
async fn test_high_concurrency_execution() {
    let file_config = AppConfig::default();

    let providers = load_app_config(None)
        .map(|pc| providers_from_postcard(&pc.providers))
        .unwrap_or_default();

    if providers.is_empty() {
        println!("Skipping real integration execution: No providers found in config.");
        return;
    }

    let client = match build_runtime_client(&file_config, &providers, std::collections::HashMap::new()) {
        Ok(c) => c,
        Err(e) => {
            println!("Skipping due to provider init failure: {}", e);
            return;
        }
    };

    let mut tasks = vec![];
    let concurrency_spawns = 15;

    let start_time = Instant::now();
    for i in 0..concurrency_spawns {
        let c = client.clone();

        let prompt = format!(
            "Execute a massive search. Session variant: {}",
            i
        );

        let handle = task::spawn(async move {
            let res = c
                .chat(ChatRequest {
                    prompt,
                    attachments: vec![],
                    system_prompt: None,
                    session_id: Some("high-concurrency-shared-session".to_string()),
                    raw_mode: false,
                    bypass_template: false,
                    force_json: true,
                })
                .await;

            res
        });
        tasks.push(handle);
    }

    let mut success_count = 0;
    for handle in tasks {
        let result = handle.await.expect("Task panicked during execution");
        if result.is_ok() {
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

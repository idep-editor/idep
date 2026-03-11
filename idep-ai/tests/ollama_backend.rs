use httpmock::{Method::POST, MockServer};
use idep_ai::backends::{ollama::OllamaBackend, Backend};

#[tokio::test]
async fn streams_tokens_and_ends_on_done() {
    let server = MockServer::start_async().await;

    let _mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/api/generate");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"response":"Hello ","done":false}
{"response":"world","done":false}
{"response":"","done":true}
"#,
                );
        })
        .await;

    let backend = OllamaBackend::new(server.base_url(), "test-model".to_string());
    let result = backend.complete("Hi", 16).await.unwrap();

    _mock.assert_async().await;
    assert_eq!(result, "Hello world");
}

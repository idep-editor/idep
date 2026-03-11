use httpmock::{Method::POST, MockServer};
use idep_ai::backends::{anthropic::AnthropicBackend, Backend};

#[tokio::test]
async fn streams_sse_and_collects_text() {
    let server = MockServer::start_async().await;

    let _mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "text/event-stream")
                .body(
                    "data: {\"delta\":{\"text\":\"Hello \"}}\n\
data: {\"delta\":{\"text\":\"world\"}}\n\
data: [DONE]\n",
                );
        })
        .await;

    let backend = AnthropicBackend::new_with_base_url(
        "test-key".into(),
        "claude-test".into(),
        64,
        server.base_url(),
    );

    let result = backend.complete("Hi", 32).await.unwrap();

    _mock.assert_async().await;
    assert_eq!(result, "Hello world");
}

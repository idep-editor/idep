use httpmock::{Method::POST, MockServer};
use idep_ai::backends::{openai_compat::OpenAiCompatBackend, Backend};

#[tokio::test]
async fn streams_chat_completion_tokens() {
    let server = MockServer::start_async().await;

    let _mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200)
                .header("content-type", "text/event-stream")
                .body(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"Hello \"}}]}\n\
data: {\"choices\":[{\"delta\":{\"content\":\"world\"}}]}\n\
data: [DONE]\n",
                );
        })
        .await;

    let backend = OpenAiCompatBackend::new(
        server.base_url(),
        Some("test-key".into()),
        "gpt-4o-mini".into(),
    );

    let result = backend.complete("Hi", 32).await.unwrap();

    _mock.assert_async().await;
    assert_eq!(result, "Hello world");
}

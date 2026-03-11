use httpmock::{Method::POST, MockServer};
use idep_ai::backends::{huggingface::HuggingFaceBackend, Backend};

#[tokio::test]
async fn returns_generated_text() {
    let server = MockServer::start_async().await;

    let _mock = server
        .mock_async(|when, then| {
            when.method(POST);
            then.status(200).json_body(vec![serde_json::json!({
                "generated_text": "Hello world, this is a test response"
            })]);
        })
        .await;

    let backend = HuggingFaceBackend::new(
        "test-token".into(),
        "bigcode/starcoder2-15b".into(),
        Some(server.base_url()),
    );

    let result = backend.complete("Hi", 32).await.unwrap();

    _mock.assert_async().await;
    assert_eq!(result, "Hello world, this is a test response");
}

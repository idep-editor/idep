// Integration test for v0.1.4 chat panel streaming contract
//
// Gate: send a question and verify streaming callback receives response tokens.

use idep_ai::{backends::mock::MockBackend, chat::ChatSession};

#[tokio::test]
async fn send_question_receives_streaming_response() {
    let backend = Box::new(MockBackend::with_response(
        "streamed answer from mock backend",
    ));
    let mut session = ChatSession::new(backend);

    let mut streamed = String::new();
    let response = session
        .send_streaming_with_context(
            "What does this file do?",
            "File: src/main.rs\nContext: fn main() {}",
            |tok| streamed.push_str(tok),
        )
        .await
        .expect("chat streaming request should succeed");

    assert!(!response.is_empty(), "response should not be empty");
    assert!(
        !streamed.is_empty(),
        "streaming callback should receive at least one token"
    );
    assert_eq!(
        streamed, response,
        "mock backend should deliver full response through callback"
    );
}

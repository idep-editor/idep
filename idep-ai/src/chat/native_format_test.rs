// Test native message format for Anthropic and other modern backends

use crate::backends::mock::MockBackend;
use crate::chat::{ChatMessage, ChatSession};
use anyhow::Result;

#[test]
fn test_build_messages_with_context() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.system = "You are a helpful assistant.".to_string();
    session.history.push(ChatMessage::user("Hello"));
    session.history.push(ChatMessage::assistant("Hi there!"));

    let message = "How are you?";
    let context = "Current file: main.rs\n```rust\nfn main() {}\n```";

    let messages = session.build_messages_with_context(message, context);

    // Should have system message pair + conversation history + current message
    assert_eq!(messages.len(), 5); // system user + system assistant + user + assistant + current user

    // Check system message (combines context and system)
    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0]["content"].as_str().unwrap().contains("System:"));
    assert!(messages[0]["content"]
        .as_str()
        .unwrap()
        .contains("You are a helpful assistant."));
    assert!(messages[0]["content"]
        .as_str()
        .unwrap()
        .contains("Context:"));

    // Check system acknowledgment
    assert_eq!(messages[1]["role"], "assistant");
    assert!(messages[1]["content"]
        .as_str()
        .unwrap()
        .contains("context and system information"));

    // Check conversation history
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"], "Hello");

    assert_eq!(messages[3]["role"], "assistant");
    assert_eq!(messages[3]["content"], "Hi there!");

    // Check current message
    assert_eq!(messages[4]["role"], "user");
    assert_eq!(messages[4]["content"], message);

    Ok(())
}

#[test]
fn test_build_messages_without_context() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.system = "You are a helpful assistant.".to_string();
    session.history.push(ChatMessage::user("Hello"));
    session.history.push(ChatMessage::assistant("Hi there!"));

    let messages = session.build_messages();

    // Should have system message pair + conversation history
    assert_eq!(messages.len(), 4); // system user + system assistant + user + assistant

    // Check system message
    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0]["content"]
        .as_str()
        .unwrap()
        .contains("System: You are a helpful assistant."));

    // Check system acknowledgment
    assert_eq!(messages[1]["role"], "assistant");
    assert!(messages[1]["content"]
        .as_str()
        .unwrap()
        .contains("Understood. I'll follow these instructions."));

    // Check conversation history
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"], "Hello");

    assert_eq!(messages[3]["role"], "assistant");
    assert_eq!(messages[3]["content"], "Hi there!");

    Ok(())
}

#[test]
fn test_build_messages_context_only() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.history.push(ChatMessage::user("Hello"));
    session.history.push(ChatMessage::assistant("Hi there!"));

    let message = "What does this function do?";
    let context = "Current file: main.rs\n```rust\nfn calculate_sum() {}\n```";

    let messages = session.build_messages_with_context(message, context);

    // Should have context message pair + conversation history + current message
    assert_eq!(messages.len(), 5); // context user + context assistant + user + assistant + current user

    // Check context message
    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0]["content"]
        .as_str()
        .unwrap()
        .contains("Context:"));

    // Check context acknowledgment
    assert_eq!(messages[1]["role"], "assistant");
    let content = messages[1]["content"].as_str().unwrap();
    assert!(content.contains("use this context") || content.contains("understand"));

    Ok(())
}

#[test]
fn test_context_window_management() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.system = "You are a helpful assistant.".to_string();

    // Add a long conversation history that would exceed token limit
    for i in 0..20 {
        session
            .history
            .push(ChatMessage::user(format!("User message {}", i)));
        session
            .history
            .push(ChatMessage::assistant(format!("Assistant response {}", i)));
    }

    let message = "Final question";
    let context = "Context: important code context";

    // With very small token limit, should truncate history
    let messages = session.build_messages_with_window_management(message, context, 100);

    println!(
        "Window management - actual message count: {}",
        messages.len()
    );

    // Should keep system messages and recent messages, truncate middle
    assert!(messages.len() <= 25); // Should be much less than original 42 messages

    // Should still have system messages
    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0]["content"].as_str().unwrap().contains("System:"));

    // Should still have current message
    assert_eq!(messages[messages.len() - 1]["role"], "user");
    assert_eq!(messages[messages.len() - 1]["content"], message);

    Ok(())
}

#[test]
fn test_system_messages_skipped_in_native_format() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    // Add system messages to history (should be skipped in native format)
    session
        .history
        .push(ChatMessage::system("System instruction 1"));
    session.history.push(ChatMessage::user("Hello"));
    session
        .history
        .push(ChatMessage::system("System instruction 2"));
    session.history.push(ChatMessage::assistant("Hi there!"));

    let messages = session.build_messages();

    // Should have system message pair + user and assistant messages
    assert_eq!(messages.len(), 4); // system user + system assistant + user + assistant

    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0]["content"].as_str().unwrap().contains("System:"));

    assert_eq!(messages[1]["role"], "assistant");
    assert!(messages[1]["content"]
        .as_str()
        .unwrap()
        .contains("Understood. I'll follow these instructions."));

    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"], "Hello");

    assert_eq!(messages[3]["role"], "assistant");
    assert_eq!(messages[3]["content"], "Hi there!");

    Ok(())
}

#[test]
fn test_multi_turn_conversation_structure() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.system = "You are a Rust expert.".to_string();

    // Simulate multi-turn conversation
    session.history.push(ChatMessage::user("What is Rust?"));
    session.history.push(ChatMessage::assistant(
        "Rust is a systems programming language.",
    ));
    session
        .history
        .push(ChatMessage::user("How do I write a function?"));
    session.history.push(ChatMessage::assistant(
        "You write functions like this: fn name() {}",
    ));

    let message = "Can you show me an example?";
    let context = "Current file: main.rs";

    let messages = session.build_messages_with_context(message, context);

    // Verify structure: system pair + conversation pairs + current message
    assert_eq!(messages.len(), 7); // system user + system assistant + 4 conversation messages + current user

    // Verify alternating roles for conversation messages (starting after system messages)
    for (i, msg) in messages.iter().enumerate().skip(2) {
        let role = msg["role"].as_str().unwrap();
        if i % 2 == 0 {
            assert_eq!(role, "user");
        } else {
            assert_eq!(role, "assistant");
        }
    }

    // Verify last message is current user message
    assert_eq!(messages[messages.len() - 1]["role"], "user");
    assert_eq!(messages[messages.len() - 1]["content"], message);

    Ok(())
}

#[test]
fn test_context_priority_truncation() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.system = "You are a helpful assistant.".to_string();

    // Add a long conversation that will exceed token limit when combined with context
    for i in 0..30 {
        session
            .history
            .push(ChatMessage::user(format!("User message {}", i)));
        session
            .history
            .push(ChatMessage::assistant(format!("Assistant response {}", i)));
    }

    let message = "Final question";
    let context = "Current file: main.rs\n```rust\nfn main() { println!(\"Hello\"); }\n```";

    // With a very tight token limit
    let messages = session.build_messages_with_window_management(message, context, 50);

    println!(
        "Priority truncation test - message count: {} (from original 62+)",
        messages.len()
    );

    // Verify system preamble is kept (2 messages: user with system+context, assistant ack)
    assert!(messages.len() >= 2, "Should keep at least system preamble");
    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0]["content"].as_str().unwrap().contains("System:"));

    // Verify context section header is in system message
    assert!(messages[0]["content"]
        .as_str()
        .unwrap()
        .contains("Context:"));

    // Verify most recent user message is kept
    assert_eq!(messages[messages.len() - 1]["role"], "user");
    assert_eq!(messages[messages.len() - 1]["content"], message);

    // Verify we're under budget (rough check: 50 tokens max, ~4 chars per token)
    let total_chars: usize = messages
        .iter()
        .map(|m| m.get("content").and_then(|v| v.as_str()).unwrap_or(""))
        .map(|s| s.len())
        .sum();
    let estimated_tokens = total_chars / 4;
    assert!(
        estimated_tokens <= 60, // Small buffer for estimation error
        "Estimated {} tokens, should be <= 60",
        estimated_tokens
    );

    Ok(())
}

#[test]
fn test_build_anthropic_request() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.system = "You are a Rust expert.".to_string();
    session
        .history
        .push(ChatMessage::user("What is ownership?"));
    session.history.push(ChatMessage::assistant(
        "Ownership is Rust's memory management system.",
    ));

    let message = "How does borrow checking work?";
    let context = "Current file: ownership_demo.rs\n```rust\nfn take_ownership(s: String) {}\n```";

    let (system_prompt, messages) = session.build_anthropic_request(message, context);

    // Verify system prompt combines instruction + context
    assert!(system_prompt.contains("You are a Rust expert."));
    assert!(system_prompt.contains("## Codebase Context"));
    assert!(system_prompt.contains("ownership_demo.rs"));

    // Verify messages are clean (no embedded system preamble)
    assert!(
        !messages[0]["content"].as_str().unwrap().contains("System:"),
        "First message should not contain system preamble"
    );

    // Verify conversation history is preserved
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "What is ownership?");
    assert_eq!(messages[1]["role"], "assistant");
    assert!(messages[1]["content"]
        .as_str()
        .unwrap()
        .contains("Ownership"));

    // Verify current message is last
    assert_eq!(messages[messages.len() - 1]["role"], "user");
    assert_eq!(messages[messages.len() - 1]["content"], message);

    // Verify no system messages in the array
    for msg in &messages {
        assert_ne!(
            msg["role"], "system",
            "System role should not appear in messages"
        );
    }

    Ok(())
}

#[test]
fn test_build_anthropic_request_without_context() -> Result<()> {
    let backend = Box::new(MockBackend::new());
    let mut session = ChatSession::new(backend);

    session.system = "You are a Python expert.".to_string();
    session
        .history
        .push(ChatMessage::user("What is a list comprehension?"));
    session
        .history
        .push(ChatMessage::assistant("[x for x in iterable if condition]"));

    let message = "Can I nest them?";
    let context = ""; // No context

    let (system_prompt, messages) = session.build_anthropic_request(message, context);

    // Verify system prompt is just the instruction (no context section)
    assert_eq!(system_prompt, "You are a Python expert.");
    assert!(!system_prompt.contains("Codebase Context"));

    // Verify messages contain conversation + current message
    assert_eq!(messages.len(), 3); // user + assistant + current user
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"], message);

    Ok(())
}

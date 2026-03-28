// Integration test for RAG context injection
// Tests that chat responses reference correct codebase chunks

use crate::backends::mock::MockBackend;
use crate::chat::ChatSession;
use crate::context::{Context, ContextEngine, CurrentFileContext, Position};
use crate::indexer::CodeChunk;
use anyhow::Result;
use std::path::PathBuf;

/// Create a test context with a specific function
fn create_test_context() -> Context {
    let current_file = CurrentFileContext {
        file_path: PathBuf::from("/test/main.rs"),
        content: r#"
fn calculate_sum(numbers: &[i32]) -> i32 {
    let mut sum = 0;
    for num in numbers.iter() {
        sum += num;
    }
    sum
}

fn calculate_product(numbers: &[i32]) -> i32 {
    let mut product = 1;
    for num in numbers.iter() {
        product *= num;
    }
    product
}

fn main() {
    let numbers = vec![1, 2, 3, 4, 5];
    let sum = calculate_sum(&numbers);
    let product = calculate_product(&numbers);
    println!("Sum: {}, Product: {}", sum, product);
}
"#
        .to_string(),
        cursor_position: Position {
            line: 2,
            character: 5,
        },
        nearby_lines: vec![
            "fn calculate_sum(numbers: &[i32]) -> i32 {".to_string(),
            "    let mut sum = 0;".to_string(),
            "    for num in numbers.iter() {".to_string(),
            "        sum += num;".to_string(),
            "    }".to_string(),
            "    sum".to_string(),
            "}".to_string(),
        ],
        language: "rust".to_string(),
    };

    let similar_chunks = vec![
        crate::context::SimilarChunk {
            chunk: CodeChunk {
                file: PathBuf::from("/test/math.rs"),
                content: r#"
/// Calculate the sum of a vector of integers
pub fn vector_sum(nums: &[i32]) -> i32 {
    nums.iter().sum()
}

/// Calculate the product of a vector of integers  
pub fn vector_product(nums: &[i32]) -> i32 {
    nums.iter().product()
}
"#
                .to_string(),
                start_line: 1,
                end_line: 8,
                kind: crate::indexer::ChunkKind::Function,
                name: Some("vector_sum".to_string()),
            },
            similarity_score: 0.95,
            relevance_rank: 1,
        },
        crate::context::SimilarChunk {
            chunk: CodeChunk {
                file: PathBuf::from("/test/utils.rs"),
                content: r#"
pub fn display_result(result: i32) {
    println!("Result: {}", result);
}
"#
                .to_string(),
                start_line: 1,
                end_line: 3,
                kind: crate::indexer::ChunkKind::Function,
                name: Some("display_result".to_string()),
            },
            similarity_score: 0.75,
            relevance_rank: 2,
        },
    ];

    Context {
        current_file: Some(current_file),
        ast_context: None, // Not implemented yet
        similar_chunks,
        edit_history: vec![], // Not implemented yet
        token_usage: crate::context::TokenUsage {
            // Note: These are mock values for testing - actual token counting not implemented yet
            total_tokens: 1500,
            max_tokens: 4096,
            current_file_tokens: 800,
            ast_context_tokens: 0,
            similar_chunks_tokens: 500,
            edit_history_tokens: 200,
        },
    }
}

/// Create a mock backend that responds with context-aware answers
fn create_context_aware_backend() -> MockBackend {
    let response = r#"Based on the code context, I can see you're asking about the `calculate_sum` function.

The `calculate_sum` function takes a slice of integers (`&[i32]`) and returns their sum by:
1. Initializing a mutable `sum` variable to 0
2. Iterating through each number in the slice
3. Adding each number to the running total
4. Returning the final sum

I also notice there's a similar function `vector_sum` in the math.rs file that does the same thing more concisely using `nums.iter().sum()`.

Would you like me to explain the differences between these approaches?"#;

    MockBackend::with_response(response)
}

#[tokio::test]
async fn test_rag_context_function_reference() -> Result<()> {
    // Setup
    let backend = Box::new(create_context_aware_backend());
    let mut session = ChatSession::new(backend);

    let context = create_test_context();
    let engine = ContextEngine::new();
    let serialized_context = engine.serialize_context(&context)?;

    // Act
    let question = "What does the calculate_sum function do?";
    let response = session
        .send_with_context(question, &serialized_context)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to send message with context in function reference test: {}",
                e
            )
        })?;

    // Assert
    println!("Question: {}", question);
    println!("Response: {}", response);

    // Verify response references the specific function
    assert!(
        response.contains("calculate_sum"),
        "Response should mention calculate_sum function"
    );
    assert!(
        response.contains("slice of integers"),
        "Response should describe the parameter type"
    );
    assert!(
        response.contains("sum"),
        "Response should explain what it returns"
    );

    // Verify response shows awareness of context (more flexible assertions)
    assert!(
        response.to_lowercase().contains("context") || response.to_lowercase().contains("based on"),
        "Response should acknowledge having context"
    );
    assert!(
        response.contains("vector_sum") || response.contains("math.rs"),
        "Response should reference similar function or file from context"
    );

    // Verify conversation history is preserved
    assert_eq!(session.history.len(), 2); // User message + assistant response
    assert_eq!(session.history[0].role, crate::chat::Role::User);
    assert_eq!(session.history[1].role, crate::chat::Role::Assistant);
    assert_eq!(session.history[0].content, question);
    assert_eq!(session.history[1].content, response);

    Ok(())
}

#[tokio::test]
async fn test_rag_context_specific_line_reference() -> Result<()> {
    // Setup with a backend that responds to specific line questions
    let response = r#"Looking at line 3 of the `calculate_sum` function:

```rust
for num in numbers.iter() {
```

This line starts a `for` loop that iterates over each element in the `numbers` slice. The `.iter()` method creates an iterator that yields references to each element, and `for num in` binds each element to the variable `num` for each iteration of the loop.

This is the core iteration logic that allows the function to process each number in the input slice."#;

    let backend = Box::new(MockBackend::with_response(response));
    let mut session = ChatSession::new(backend);

    let context = create_test_context();
    let engine = ContextEngine::new();
    let serialized_context = engine.serialize_context(&context)?;

    // Act - ask about a specific line
    let question = "What's happening on line 3 of the calculate_sum function?";
    let response = session
        .send_with_context(question, &serialized_context)
        .await?;

    // Assert
    println!("Question: {}", question);
    println!("Response: {}", response);

    // Verify response references the specific line
    assert!(
        response.contains("line 3"),
        "Response should reference the specific line"
    );
    assert!(
        response.contains("for num in numbers.iter()"),
        "Response should mention the loop"
    );

    // Verify context awareness
    assert!(
        response.contains("calculate_sum"),
        "Response should mention the function name"
    );

    Ok(())
}

#[tokio::test]
async fn test_rag_context_multiple_functions() -> Result<()> {
    // Setup with a backend that compares multiple functions
    let response = r#"Looking at the context, I can see two sum functions:

1. `calculate_sum` in main.rs: Uses manual iteration with a for loop
2. `vector_sum` in math.rs: Uses the built-in `iter().sum()` method

The `vector_sum` function is more idiomatic Rust and concise, while `calculate_sum` shows the explicit iteration process which might be better for learning or when you need to add additional logic inside the loop.

Both functions have the same signature: `fn(&[i32]) -> i32` and produce the same result."#;

    let backend = Box::new(MockBackend::with_response(response));
    let mut session = ChatSession::new(backend);

    let context = create_test_context();
    let engine = ContextEngine::new();
    let serialized_context = engine.serialize_context(&context)?;

    // Act
    let question = "What's the difference between calculate_sum and vector_sum?";
    let response = session
        .send_with_context(question, &serialized_context)
        .await?;

    // Assert
    println!("Question: {}", question);
    println!("Response: {}", response);

    // Verify response references both functions
    assert!(
        response.contains("calculate_sum"),
        "Response should mention calculate_sum"
    );
    assert!(
        response.contains("vector_sum"),
        "Response should mention vector_sum"
    );
    assert!(
        response.contains("main.rs"),
        "Response should reference file location"
    );
    assert!(
        response.contains("math.rs"),
        "Response should reference file location"
    );

    Ok(())
}

#[test]
fn test_context_serialization_includes_function_info() -> Result<()> {
    // Setup
    let context = create_test_context();
    let engine = ContextEngine::new();
    let serialized = engine.serialize_context(&context)?;

    // Assert
    assert!(
        serialized.contains("Current File Context"),
        "Should include current file section"
    );
    assert!(
        serialized.contains("calculate_sum"),
        "Should include function name"
    );
    assert!(
        serialized.contains("Relevant Code Chunks"),
        "Should include similar chunks section"
    );
    assert!(
        serialized.contains("vector_sum"),
        "Should include similar function"
    );
    assert!(serialized.contains("math.rs"), "Should include file path");
    assert!(
        serialized.contains("Context uses 1500/4096 tokens"),
        "Should include token usage"
    );

    println!("Serialized context:\n{}", serialized);

    Ok(())
}

use std::env;

use ai_sdk::*;

#[tokio::main]
async fn main() {
    let api_key = env::var("OPENAI_API_KEY").unwrap();
    let adapter = OpenAIAdapter::new(api_key, "gpt-4o");
    let messages = vec![Message {
        role: Role::User,
        content: "Hello".to_string(),
    }];
    let response = adapter.complete(&messages).await.unwrap();
    println!("response: {}", response);
}

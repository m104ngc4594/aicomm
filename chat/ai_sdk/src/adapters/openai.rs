use crate::{AiAdapter, AiService, Message};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAiAdapter {
    host: String,
    api_key: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
pub struct OpenAIChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct OpenAIChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub system_fingerprint: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
}

#[derive(Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub logprobs: Option<i64>,
    pub finish_reason: String,
}

#[derive(Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub completion_tokens_details: Option<OpenAICompletionTokensDetails>,
}

#[derive(Deserialize)]
pub struct OpenAICompletionTokensDetails {
    pub reasoning_tokens: u32,
}

impl OpenAiAdapter {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let client = Client::new();
        Self {
            host: "https://api.openai.com/v1".to_string(),
            api_key: api_key.into(),
            model: model.into(),
            client,
        }
    }
}

impl AiService for OpenAiAdapter {
    async fn complete(&self, messages: &[Message]) -> Result<String> {
        let request = OpenAIChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.iter().map(|m| m.into()).collect(),
        };

        let url = format!("{}/chat/completions", self.host);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        let mut data: OpenAIChatCompletionResponse = response.json().await?;
        let content = data
            .choices
            .pop()
            .ok_or(anyhow!("No response"))?
            .message
            .content;
        Ok(content)
    }
}

impl From<OpenAiAdapter> for AiAdapter {
    fn from(adapter: OpenAiAdapter) -> Self {
        Self::OpenAi(adapter)
    }
}

impl From<&Message> for OpenAIMessage {
    fn from(message: &Message) -> Self {
        Self {
            role: message.role.to_string(),
            content: message.content.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::Role;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn openai_complete_should_work() {
        let api_key = env::var("OPENAI_API_KEY").unwrap();
        let adapter = OpenAiAdapter::new(api_key, "gpt-4o");
        let messages = vec![Message {
            role: Role::User,
            content: "Hello".to_string(),
        }];
        let response = adapter.complete(&messages).await.unwrap();
        assert!(!response.is_empty());
    }
}

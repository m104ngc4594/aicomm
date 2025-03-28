use crate::{AppConfig, VECTOR_SIZE};
use chat_core::Message;
use serde::{Deserialize, Serialize};
use sqlx::{
    PgPool,
    postgres::{PgListener, PgPoolOptions},
};
use std::collections::HashSet;
use swiftide::{
    integrations,
    query::{Pipeline, answers, query_transformers, response_transformers},
    traits::{EmbeddingModel, SimplePrompt},
};
use swiftide_pgvector::PgVectorBuilder;
use tokio_stream::StreamExt;
use tracing::info;

#[allow(dead_code)]
#[derive(Debug)]
struct Notification {
    // users being impacted, so we should send the notification to them
    bot_id: i64,
    event: Message,
}

// pg_notify('chat_message_created', row_to_json(NEW)::text);
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageCreated {
    message: Message,
    members: HashSet<i64>,
}

pub async fn setup_pg_listener(config: &AppConfig) -> anyhow::Result<()> {
    let db_url = &config.server.db_url;
    let mut listener = PgListener::connect(db_url).await?;
    listener.listen("chat_message_created").await?;
    info!("Listening for chat_message_created");

    let pool = PgPoolOptions::new().connect(db_url).await?;
    let bots = get_bots(&pool).await?;

    let client = integrations::ollama::Ollama::default()
        .with_default_prompt_model("deepseek-r1:7b")
        .to_owned();
    let ollama_embed = integrations::ollama::Ollama::default()
        .with_default_embed_model("bge-m3:latest")
        .to_owned();

    let mut stream = listener.into_stream();

    while let Some(Ok(notif)) = stream.next().await {
        info!("Received notification: {:?}", notif);
        if let Some(notification) = Notification::load(notif.channel(), notif.payload(), &bots) {
            let pool = pool.clone();
            let client = client.clone();
            let ollama_embed = ollama_embed.clone();
            tokio::spawn(async move { notification.process(&pool, client, ollama_embed).await });
        }
    }

    Ok(())
}

impl Notification {
    fn load(r#type: &str, payload: &str, bots: &HashSet<i64>) -> Option<Self> {
        match r#type {
            "chat_message_created" => {
                let payload: ChatMessageCreated = serde_json::from_str(payload).ok()?;
                let mut members = payload.members;
                members.remove(&payload.message.sender_id);

                // only process if it's a direct message
                if members.len() == 1 {
                    let bot_id = members.iter().next().unwrap();
                    if bots.contains(bot_id) {
                        return Some(Self {
                            bot_id: *bot_id,
                            event: payload.message,
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    async fn process(
        self,
        pool: &PgPool,
        client: impl SimplePrompt + Clone + 'static,
        ollama_embed: impl EmbeddingModel + Clone + 'static,
    ) -> anyhow::Result<()> {
        let store = PgVectorBuilder::default()
            .pool(pool.clone())
            .vector_size(VECTOR_SIZE as _)
            .build()?;
        let pipeline = Pipeline::default()
            .then_transform_query(query_transformers::GenerateSubquestions::from_client(
                client.clone(),
            ))
            .then_transform_query(query_transformers::Embed::from_client(ollama_embed))
            .then_retrieve(store)
            .then_transform_response(response_transformers::Summary::from_client(client.clone()))
            .then_answer(answers::Simple::from_client(client.clone()));
        info!("Processing notification: {:?}", self.event.id);
        let ret = pipeline.query(&self.event.content).await?;
        let answer = ret.answer();
        info!("Got answer. Writing to db...");

        sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (chat_id, sender_id, content)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(self.event.chat_id)
        .bind(self.bot_id)
        .bind(answer)
        .fetch_one(pool)
        .await?;

        Ok(())
    }
}

async fn get_bots(pool: &PgPool) -> anyhow::Result<HashSet<i64>> {
    let bots = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT id
        FROM users
        WHERE is_bot = true
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(bots.into_iter().map(|(id,)| id).collect())
}

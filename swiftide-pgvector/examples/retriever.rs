use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use swiftide::{
    integrations,
    query::{self, answers, query_transformers, response_transformers},
};
use swiftide_pgvector::PgVector;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{Layer as _, fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt};

const VECTOR_SIZE: usize = 1024;

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let pool = PgPoolOptions::new()
        .connect("postgres://postgres:postgres@localhost:5432/swiftide_rag")
        .await?;
    let client = integrations::ollama::Ollama::default()
        .with_default_prompt_model("deepseek-r1:7b")
        .to_owned();
    let ollama_embed = integrations::ollama::Ollama::default()
        .with_default_embed_model("bge-m3:latest")
        .to_owned();
    let store = PgVector::try_new(pool, VECTOR_SIZE as _).await?;

    let pipeline = query::Pipeline::default()
        .then_transform_query(query_transformers::GenerateSubquestions::from_client(
            client.clone(),
        ))
        .then_transform_query(query_transformers::Embed::from_client(ollama_embed))
        .then_retrieve(store)
        .then_transform_response(response_transformers::Summary::from_client(client.clone()))
        .then_answer(answers::Simple::from_client(client.clone()));

    let result = pipeline
        .query("这个代码在做什么事情？请用中文简单回答")
        .await?;

    println!("{:#?}", result.answer());

    Ok(())
}

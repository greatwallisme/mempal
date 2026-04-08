use std::path::{Path, PathBuf};
use std::sync::Arc;

use mempal_core::{
    db::Database,
    types::{Drawer, SourceType},
};
use mempal_embed::Embedder;
use mempal_mcp::{EmbedderFactory, MempalMcpServer};
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
};
use serde_json::Value;
use tempfile::tempdir;

#[derive(Default)]
struct TestEmbedder;

#[async_trait::async_trait]
impl Embedder for TestEmbedder {
    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|text| fake_embedding(text)).collect())
    }

    fn dimensions(&self) -> usize {
        384
    }

    fn name(&self) -> &str {
        "test"
    }
}

#[derive(Clone, Default)]
struct TestEmbedderFactory;

#[async_trait::async_trait]
impl EmbedderFactory for TestEmbedderFactory {
    async fn build(&self) -> anyhow::Result<Box<dyn Embedder>> {
        Ok(Box::new(TestEmbedder))
    }
}

fn fake_embedding(text: &str) -> Vec<f32> {
    let mut embedding = vec![0.0_f32; 384];
    for (index, byte) in text.bytes().enumerate() {
        embedding[index % 384] += f32::from(byte) / 255.0;
    }
    embedding
}

fn open_db(path: &Path) -> Database {
    Database::open(path).expect("database should open")
}

fn insert_drawer(db: &Database, id: &str, content: &str, wing: &str, room: Option<&str>) {
    db.insert_drawer(&Drawer {
        id: id.to_string(),
        content: content.to_string(),
        wing: wing.to_string(),
        room: room.map(ToOwned::to_owned),
        source_file: Some(format!("/tmp/{id}.md")),
        source_type: SourceType::Project,
        added_at: "1712640000".to_string(),
        chunk_index: Some(0),
    })
    .expect("drawer insert should succeed");

    let vector_json =
        serde_json::to_string(&fake_embedding(content)).expect("vector JSON should serialize");
    db.conn()
        .execute(
            "INSERT INTO drawer_vectors (id, embedding) VALUES (?1, vec_f32(?2))",
            (id, vector_json.as_str()),
        )
        .expect("vector insert should succeed");
}

async fn spawn_server(db_path: PathBuf) -> anyhow::Result<rmcp::service::RunningService<rmcp::RoleClient, ()>> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let server = MempalMcpServer::new_with_factory(db_path, Arc::new(TestEmbedderFactory));
    tokio::spawn(async move {
        let service = server.serve(server_transport).await?;
        service.waiting().await?;
        anyhow::Ok(())
    });

    let client = ().serve(client_transport).await?;
    Ok(client)
}

#[tokio::test]
async fn test_mcp_server_start() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let client = spawn_server(dir.path().join("palace.db")).await?;

    let tools = client.list_all_tools().await?;
    let names = tools.iter().map(|tool| tool.name.as_ref()).collect::<Vec<_>>();

    assert_eq!(names.len(), 4);
    assert!(names.contains(&"mempal_status"));
    assert!(names.contains(&"mempal_search"));
    assert!(names.contains(&"mempal_ingest"));
    assert!(names.contains(&"mempal_taxonomy"));

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_mcp_search() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("palace.db");
    let db = open_db(&db_path);
    insert_drawer(
        &db,
        "drawer_auth",
        "We chose Clerk for the auth decision.",
        "myapp",
        Some("auth"),
    );
    let client = spawn_server(db_path).await?;

    let result = client
        .call_tool(
            CallToolRequestParams::new("mempal_search").with_arguments(
                serde_json::json!({
                    "query": "auth decision clerk",
                    "top_k": 5
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        )
        .await?;

    let payload = result.structured_content.expect("structured result should exist");
    let first = payload
        .get("results")
        .and_then(Value::as_array)
        .and_then(|results| results.first())
        .expect("search should return one result");
    assert!(first.get("drawer_id").is_some());
    assert!(first.get("source_file").is_some());
    assert!(first.get("similarity").is_some());

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_mcp_ingest() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("palace.db");
    let client = spawn_server(db_path.clone()).await?;

    let result = client
        .call_tool(
            CallToolRequestParams::new("mempal_ingest").with_arguments(
                serde_json::json!({
                    "content": "decided to use Clerk",
                    "wing": "myapp",
                    "room": "auth",
                    "source": "/tmp/decision.md"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        )
        .await?;

    let payload = result.structured_content.expect("structured result should exist");
    let drawer_id = payload
        .get("drawer_id")
        .and_then(Value::as_str)
        .expect("drawer_id should be returned");

    let db = open_db(&db_path);
    let count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM drawers WHERE id = ?1",
        [drawer_id],
        |row| row.get(0),
    )?;
    assert_eq!(count, 1);

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_mcp_status_and_taxonomy() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("palace.db");
    let db = open_db(&db_path);
    insert_drawer(&db, "drawer_auth", "auth decision", "myapp", Some("auth"));
    db.conn().execute(
        "INSERT INTO taxonomy (wing, room, display_name, keywords) VALUES (?1, ?2, ?3, ?4)",
        ("myapp", "auth", "auth", "[\"auth\",\"login\"]"),
    )?;
    let client = spawn_server(db_path).await?;

    let status = client
        .call_tool(CallToolRequestParams::new("mempal_status"))
        .await?;
    let status_payload = status.structured_content.expect("status payload should exist");
    assert_eq!(
        status_payload.get("drawer_count").and_then(Value::as_i64),
        Some(1)
    );

    let taxonomy = client
        .call_tool(
            CallToolRequestParams::new("mempal_taxonomy").with_arguments(
                serde_json::json!({
                    "action": "list"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        )
        .await?;
    let taxonomy_payload = taxonomy
        .structured_content
        .expect("taxonomy payload should exist");
    assert_eq!(
        taxonomy_payload
            .get("entries")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );

    client.cancel().await?;
    Ok(())
}

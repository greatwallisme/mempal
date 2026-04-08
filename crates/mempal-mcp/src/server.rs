use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use mempal_core::{
    config::Config,
    db::Database,
    types::{Drawer, RouteDecision, SearchResult, SourceType, TaxonomyEntry},
};
use mempal_embed::{Embedder, EMBEDDING_DIMENSIONS, api::ApiEmbedder, onnx::OnnxEmbedder};
use mempal_search::{filter::build_filter_clause, route::route_query};
use rmcp::{
    ErrorData, Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::tools::{
    IngestRequest, IngestResponse, ScopeCount, SearchRequest, SearchResponse, SearchResultDto,
    StatusResponse, TaxonomyEntryDto, TaxonomyRequest, TaxonomyResponse, build_drawer_id,
    current_timestamp,
};

#[async_trait]
pub trait EmbedderFactory: Send + Sync {
    async fn build(&self) -> Result<Box<dyn Embedder>>;
}

#[derive(Clone)]
pub struct ConfiguredEmbedderFactory {
    config: Config,
}

impl ConfiguredEmbedderFactory {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EmbedderFactory for ConfiguredEmbedderFactory {
    async fn build(&self) -> Result<Box<dyn Embedder>> {
        match self.config.embed.backend.as_str() {
            "onnx" => Ok(Box::new(
                OnnxEmbedder::new_or_download()
                    .await
                    .context("failed to initialize ONNX embedder")?,
            )),
            "api" => Ok(Box::new(ApiEmbedder::new(
                self.config
                    .embed
                    .api_endpoint
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434/api/embeddings".to_string()),
                self.config.embed.api_model.clone(),
                EMBEDDING_DIMENSIONS,
            ))),
            backend => bail!("unsupported embed backend: {backend}"),
        }
    }
}

#[derive(Clone)]
pub struct MempalMcpServer {
    db_path: PathBuf,
    embedder_factory: Arc<dyn EmbedderFactory>,
    tool_router: ToolRouter<Self>,
}

impl MempalMcpServer {
    pub fn new(db_path: PathBuf, config: Config) -> Self {
        Self::new_with_factory(db_path, Arc::new(ConfiguredEmbedderFactory::new(config)))
    }

    pub fn new_with_factory(db_path: PathBuf, embedder_factory: Arc<dyn EmbedderFactory>) -> Self {
        Self {
            db_path,
            embedder_factory,
            tool_router: Self::tool_router(),
        }
    }

    pub async fn serve_stdio(
        self,
    ) -> Result<rmcp::service::RunningService<rmcp::RoleServer, Self>>
    {
        self.serve(rmcp::transport::stdio())
            .await
            .context("failed to initialize MCP stdio transport")
    }

    fn open_db(&self) -> std::result::Result<Database, ErrorData> {
        Database::open(&self.db_path).map_err(|error| {
            ErrorData::internal_error(format!("failed to open database: {error}"), None)
        })
    }

    async fn build_embedder(&self) -> std::result::Result<Box<dyn Embedder>, ErrorData> {
        self.embedder_factory.build().await.map_err(|error| {
            ErrorData::internal_error(format!("failed to build embedder: {error}"), None)
        })
    }
}

#[tool_router(router = tool_router)]
impl MempalMcpServer {
    #[tool(name = "mempal_status", description = "Return drawer counts, taxonomy counts, database size, and scope breakdown.")]
    async fn mempal_status(&self) -> std::result::Result<Json<StatusResponse>, ErrorData> {
        let db = self.open_db()?;
        let drawer_count = query_count(&db, "drawers")?;
        let taxonomy_count = query_count(&db, "taxonomy")?;
        let db_size_bytes = db.database_size_bytes().map_err(db_error)?;
        let scopes = scope_counts(&db)?;

        Ok(Json(StatusResponse {
            drawer_count,
            taxonomy_count,
            db_size_bytes,
            scopes,
        }))
    }

    #[tool(name = "mempal_search", description = "Search project memory with optional wing and room filters.")]
    async fn mempal_search(
        &self,
        Parameters(request): Parameters<SearchRequest>,
    ) -> std::result::Result<Json<SearchResponse>, ErrorData> {
        let embedder = self.build_embedder().await?;
        let query_vector = embedder
            .embed(&[request.query.as_str()])
            .await
            .map_err(|error| ErrorData::internal_error(format!("embedding failed: {error}"), None))?
            .into_iter()
            .next()
            .ok_or_else(|| ErrorData::internal_error("embedder returned no query vector", None))?;
        let db = self.open_db()?;
        let route = resolve_route(
            &db,
            &request.query,
            request.wing.as_deref(),
            request.room.as_deref(),
        )?;
        let results = run_search(
            &db,
            &query_vector,
            route,
            request.top_k.unwrap_or(10),
        )
        .map_err(|error| ErrorData::internal_error(format!("search failed: {error}"), None))?;

        Ok(Json(SearchResponse {
            results: results.into_iter().map(SearchResultDto::from).collect(),
        }))
    }

    #[tool(name = "mempal_ingest", description = "Store a single memory drawer from raw content.")]
    async fn mempal_ingest(
        &self,
        Parameters(request): Parameters<IngestRequest>,
    ) -> std::result::Result<Json<IngestResponse>, ErrorData> {
        let embedder = self.build_embedder().await?;
        let vector = embedder
            .embed(&[request.content.as_str()])
            .await
            .map_err(|error| {
                ErrorData::internal_error(format!("embedding failed: {error}"), None)
            })?
            .into_iter()
            .next()
            .ok_or_else(|| ErrorData::internal_error("embedder returned no vector", None))?;
        let db = self.open_db()?;
        let room = request.room.as_deref();
        let drawer_id = build_drawer_id(&request.wing, room, &request.content);

        if !drawer_exists(&db, &drawer_id)? {
            db.insert_drawer(&Drawer {
                id: drawer_id.clone(),
                content: request.content,
                wing: request.wing,
                room: request.room,
                source_file: request.source,
                source_type: SourceType::Manual,
                added_at: current_timestamp(),
                chunk_index: Some(0),
            })
            .map_err(db_error)?;
            insert_vector(&db, &drawer_id, &vector)?;
        }

        Ok(Json(IngestResponse { drawer_id }))
    }

    #[tool(name = "mempal_taxonomy", description = "List or edit taxonomy entries.")]
    async fn mempal_taxonomy(
        &self,
        Parameters(request): Parameters<TaxonomyRequest>,
    ) -> std::result::Result<Json<TaxonomyResponse>, ErrorData> {
        let db = self.open_db()?;
        match request.action.as_str() {
            "list" => {
                let entries = db
                    .taxonomy_entries()
                    .map_err(db_error)?
                    .into_iter()
                    .map(TaxonomyEntryDto::from)
                    .collect();
                Ok(Json(TaxonomyResponse {
                    action: "list".to_string(),
                    entries,
                }))
            }
            "edit" => {
                let wing = request
                    .wing
                    .ok_or_else(|| ErrorData::invalid_params("missing wing", None))?;
                let room = request
                    .room
                    .ok_or_else(|| ErrorData::invalid_params("missing room", None))?;
                let keywords = request
                    .keywords
                    .ok_or_else(|| ErrorData::invalid_params("missing keywords", None))?;
                let entry = TaxonomyEntry {
                    wing,
                    room,
                    display_name: None,
                    keywords,
                };
                db.upsert_taxonomy_entry(&entry).map_err(db_error)?;
                Ok(Json(TaxonomyResponse {
                    action: "edit".to_string(),
                    entries: vec![TaxonomyEntryDto::from(entry)],
                }))
            }
            action => Err(ErrorData::invalid_params(
                format!("unsupported taxonomy action: {action}"),
                None,
            )),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for MempalMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Mempal MCP server exposing memory search, ingest, taxonomy, and status tools.")
    }
}

fn query_count(db: &Database, table: &str) -> std::result::Result<i64, ErrorData> {
    db.conn()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
        .map_err(|error| {
            ErrorData::internal_error(format!("failed to count {table}: {error}"), None)
        })
}

fn scope_counts(db: &Database) -> std::result::Result<Vec<ScopeCount>, ErrorData> {
    let mut statement = db
        .conn()
        .prepare(
            r#"
            SELECT wing, room, COUNT(*)
            FROM drawers
            GROUP BY wing, room
            ORDER BY wing, room
            "#,
        )
        .map_err(|error| {
            ErrorData::internal_error(format!("failed to prepare scope query: {error}"), None)
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(ScopeCount {
                wing: row.get(0)?,
                room: row.get(1)?,
                drawer_count: row.get(2)?,
            })
        })
        .map_err(|error| {
            ErrorData::internal_error(format!("failed to execute scope query: {error}"), None)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| {
            ErrorData::internal_error(format!("failed to collect scope rows: {error}"), None)
        })?;

    Ok(rows)
}

fn drawer_exists(db: &Database, drawer_id: &str) -> std::result::Result<bool, ErrorData> {
    let exists: i64 = db
        .conn()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM drawers WHERE id = ?1)",
            [drawer_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            ErrorData::internal_error(format!("failed to check drawer existence: {error}"), None)
        })?;
    Ok(exists == 1)
}

fn insert_vector(
    db: &Database,
    drawer_id: &str,
    vector: &[f32],
) -> std::result::Result<(), ErrorData> {
    let vector_json = serde_json::to_string(vector).map_err(|error| {
        ErrorData::internal_error(format!("failed to serialize vector: {error}"), None)
    })?;
    db.conn()
        .execute(
            "INSERT INTO drawer_vectors (id, embedding) VALUES (?1, vec_f32(?2))",
            (drawer_id, vector_json.as_str()),
        )
        .map_err(|error| {
            ErrorData::internal_error(format!("failed to insert vector: {error}"), None)
        })?;

    Ok(())
}

fn db_error(error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(format!("{error}"), None)
}

fn run_search(
    db: &Database,
    query_vector: &[f32],
    route: RouteDecision,
    top_k: usize,
) -> Result<Vec<SearchResult>> {
    if top_k == 0 {
        return Ok(Vec::new());
    }

    let applied_wing = route.wing.as_deref();
    let applied_room = route.room.as_deref();

    let count_sql = format!(
        "SELECT COUNT(*) FROM drawers d {}",
        build_filter_clause("d", 1, 2)
    );
    let candidate_count: i64 = db
        .conn()
        .query_row(&count_sql, (applied_wing, applied_room), |row| row.get(0))
        .context("failed to count candidate drawers")?;
    if candidate_count == 0 {
        return Ok(Vec::new());
    }

    let total_count: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM drawers", [], |row| row.get(0))
        .context("failed to count total drawers")?;
    let query_json = serde_json::to_string(query_vector).context("failed to serialize query vector")?;
    let top_k = i64::try_from(top_k).context("top_k does not fit into i64")?;

    let search_sql = format!(
        r#"
        WITH matches AS (
            SELECT id, distance
            FROM drawer_vectors
            WHERE embedding MATCH vec_f32(?1)
              AND k = ?2
        )
        SELECT d.id, d.content, d.wing, d.room, d.source_file, matches.distance
        FROM matches
        JOIN drawers d ON d.id = matches.id
        {}
        ORDER BY matches.distance ASC
        LIMIT ?5
        "#,
        build_filter_clause("d", 3, 4)
    );

    let mut statement = db.conn().prepare(&search_sql)?;
    let results = statement
        .query_map(
            (
                query_json.as_str(),
                total_count,
                applied_wing,
                applied_room,
                top_k,
            ),
            |row| {
                let distance: f64 = row.get(5)?;
                Ok(SearchResult {
                    drawer_id: row.get(0)?,
                    content: row.get(1)?,
                    wing: row.get(2)?,
                    room: row.get(3)?,
                    source_file: row.get(4)?,
                    similarity: (1.0_f64 - distance) as f32,
                    route: route.clone(),
                })
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(results)
}

fn resolve_route(
    db: &Database,
    query: &str,
    wing: Option<&str>,
    room: Option<&str>,
) -> std::result::Result<RouteDecision, ErrorData> {
    if wing.is_some() || room.is_some() {
        let scope = match (wing, room) {
            (Some(wing), Some(room)) => format!("{wing}/{room}"),
            (Some(wing), None) => wing.to_string(),
            (None, Some(room)) => format!("room={room}"),
            (None, None) => "global".to_string(),
        };
        return Ok(RouteDecision {
            wing: wing.map(ToOwned::to_owned),
            room: room.map(ToOwned::to_owned),
            confidence: 1.0,
            reason: format!("explicit filters provided: {scope}"),
        });
    }

    let taxonomy = db.taxonomy_entries().map_err(db_error)?;
    let route = route_query(query, &taxonomy);
    if route.confidence >= 0.5 {
        return Ok(route);
    }

    Ok(RouteDecision {
        wing: None,
        room: None,
        confidence: route.confidence,
        reason: route.reason,
    })
}

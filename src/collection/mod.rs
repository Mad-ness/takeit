mod models;
mod document;
mod collection;
pub mod handlers;
pub use self::collection::{Collection, CollectionError};

use std::{sync::Arc, collections::HashMap, str::FromStr};
use tokio::sync::RwLock;
use tower::{Layer, ServiceBuilder};
use axum::{
    Json,
    Router,
    RouterService,
    ServiceExt,
    http::{Request},
    routing::{get, IntoMakeService},
    // handler::Handler,
    extract::{Path, State, Query},
    response::{Result, Response, IntoResponse},
    middleware::{self, Next},
};
use self::document::{DocumentError, DocumentOverrides};

#[derive(Clone)]
pub struct SharedCollection(Arc<RwLock<Collection>>);

impl From<Collection> for SharedCollection {
    fn from(collection: Collection) -> Self {
        Self(Arc::new(RwLock::new(collection)))
    }
}

///
/// Collection API
///
/// /collection
/// /collection/<name>
/// /collection/<name>/attrs        get attributes needed to look up values of all documents from the collection
/// /collection/<name>/values       look up values from documents in the collection
/// /collection/<name>/document
/// /collection/<name>/document/<name>/value
/// /collection/<name>/document/<name1,name2,...>/value
///
pub fn collection_router() -> Router<SharedCollection> {
    let router = Router::new() // with_state(collection)
        .route("/", get(handlers::get_collections))
        .route("/stat", get(handlers::get_collections_stat))
        .route("/:collection_name", get(handlers::get_collection))
        .route("/:collection_name/attrs", get(handlers::get_collection_attrs))
        .route("/:collection_name/values", get(handlers::get_collection_values))
        .route("/:collection_name/document", get(handlers::get_documents))
        .route("/:collection_name/document/:document_name", get(handlers::get_document))
        .route("/:collection_name/document/:document_name/attrs", get(handlers::get_document_attrs))
        .route("/:collection_name/document/:document_name/value", get(handlers::get_document_value))
        .route("/:collection_name/document/:document_name/overrides", get(handlers::get_document_overrides));
    tracing::info!("collection API initialized");
    router
}

pub async fn remove_trailing_slash<B>(mut req: Request<B>, next: Next<B>) -> Response {
    *req.uri_mut() = http::uri::Uri::from_str(req.uri().path().trim_end_matches('/'))
        .unwrap_or_else(|_| req.uri().clone());
    next.run(req).await
}

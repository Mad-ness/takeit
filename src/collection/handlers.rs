use std::{sync::Arc, collections::HashMap, str::FromStr};
use tokio::sync::RwLock;
use super::{
    models,
    Collection, SharedCollection, CollectionError,
    document::{Document, DocumentError, DocumentOverrides},
};
use axum::{
    Json,
    Router,
    RouterService,
    ServiceExt,
    http::{Request},
    routing::{get, IntoMakeService},
    handler::Handler,
    extract::{Path, State, Query},
    response::{Result, Response, IntoResponse},
    middleware::{self, Next},
};

pub async fn remove_trailing_slash<B>(mut req: Request<B>, next: Next<B>) -> Response {
    *req.uri_mut() = http::uri::Uri::from_str(req.uri().path().trim_end_matches('/'))
        .unwrap_or_else(|_| req.uri().clone());
    next.run(req).await
}

#[derive(serde::Serialize)]
pub struct CollectionsStat {
    ping: &'static str,
    total_collections: usize,
    total_documents: usize,
}

pub async fn get_collections_stat(State(collections): State<SharedCollection>) -> Json<CollectionsStat> {
    let collections = &*collections.0.read().await;
    let (total_c, total_d) = (collections.total_collections(), collections.total_documents());
    Json(CollectionsStat { ping: "pong", total_collections: total_c, total_documents: total_d })
}


pub async fn get_documents(Path(collection_name): Path<String>, State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    models::CollectionInfo::try_from((&*collection.0.read().await, &collection_name))
        .map_or_else(
            |___| Err(models::CollectionResponse::CollectionNotFound(collection_name.clone())),
            |col| Ok(models::CollectionResponse::CollectionInfo(col))
        )
}

/// Get a `DocumentInfo` by `collection_name` and `document_name`.
pub async fn get_document(Path((collection_name, document_name)): Path<(String, String)>,
                      State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    (&*collection.0.read().await)
        .get_document(&collection_name, &document_name)
        .map_or_else(
            |   | Err(models::CollectionResponse::DocumentNotFound(collection_name.clone(), document_name.clone())),
            |doc| Ok(models::CollectionResponse::DocumentInfo(models::DocumentInfo::from(doc)))
        )
}

/// Get a `Document`'s attributes expected for value lookup
pub async fn get_document_attrs(Path((collection_name, document_name)): Path<(String, String)>,
                            State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    (&*collection.0.read().await)
        .get_document(&collection_name, &document_name)
        .map_or_else(
            |   | Err(models::CollectionResponse::DocumentNotFound(collection_name.clone(), document_name.clone())),
            |doc| Ok(models::CollectionResponse::DocumentAttrs(models::DocumentAttrs::from(doc)))
        )
}

/// Lookup a `Document`'s value.
pub async fn get_document_value(Path((collection_name, document_name)): Path<(String, String)>,
                            Query(query): Query<HashMap<String, String>>,
                            State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    (&*collection.0.read().await)
        .get_document(&collection_name, &document_name)
        .map_or_else(
            |   | Err(models::CollectionResponse::DocumentNotFound(collection_name.clone(), document_name.clone())),
            |doc| Ok(models::CollectionResponse::DocumentValue(doc.get_value(&query)))
        )
}

///
/// Lookup a `Document`'s overrides.
///
pub async fn get_document_overrides(Path((collection_name, document_name)): Path<(String, String)>,
                                Query(query): Query<HashMap<String, String>>,
                                State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    (&*collection.0.read().await)
        .get_document(&collection_name, &document_name)
        .map_or_else(
            |   | Err(models::CollectionResponse::DocumentNotFound(collection_name.clone(), document_name.clone())),
            |doc| Ok(models::CollectionResponse::DocumentOverrides(models::DocumentOverrides::from(doc)))
        )
}

/// Get a list of `CollectionInfo`.
pub async fn get_collections(State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    let collections: Vec<models::CollectionInfo> = (&*collection.0.read().await
        .documents
        .iter()
        .map(|(name, documents)| models::CollectionInfo::from((documents, name)))
        .collect::<Vec<models::CollectionInfo>>()).to_vec();

    Ok(models::CollectionResponse::Collections(models::CollectionList::from(collections)))
}

///
/// Get a list of attributes from all documents found in the collection needed to look up values.
///
pub async fn get_collection_attrs(Path(collection_name): Path<String>, State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    let collection_info = models::CollectionInfo::try_from((&*collection.0.read().await, &collection_name))
        .map_err(|_| models::CollectionResponse::CollectionNotFound(collection_name.clone()))?;
    Ok(models::CollectionResponse::CollectionAttrs(collection_info.attrs()))
}

pub async fn get_collection_values(Path(collection_name): Path<String>,
                               Query(query): Query<HashMap<String, String>>,
                               State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    (&*collection.0.read().await).get_values(&collection_name, &query)
        .map_or_else(
            || Err(models::CollectionResponse::CollectionNotFound(collection_name.clone())),
            |values| Ok(models::CollectionResponse::CollectionValues(values))
        )
}

pub async fn get_collection(Path(collection_name): Path<String>, State(collection): State<SharedCollection>)
    -> Result<models::CollectionResponse, models::CollectionResponse>
{
    let info = models::CollectionInfo::try_from((&*collection.0.read().await, &collection_name))
        .map_err(|_| models::CollectionResponse::CollectionNotFound(collection_name.clone()))?;
    Ok(models::CollectionResponse::CollectionInfo(info))
}

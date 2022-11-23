///
///
/// The module provides structures for getting short information
/// about `Document`s and `Collection`s.
///
///
use super::{
    document::{Document, ParamValue, DocumentOverrides as DocOverrides, DocumentValueType},
    collection::{Collection, CollectionError},
};
use serde::Serialize;
use axum::{
    Json,
    response::{Response, IntoResponse},
    http::StatusCode
};
use std::collections::HashMap;

#[derive(Clone, Serialize)]
pub struct DocumentValue(ParamValue);

#[derive(Clone, Serialize)]
pub struct DocumentAttrs(Vec<String>);

#[derive(Clone, Serialize)]
pub struct DocumentOverrides(DocOverrides);

#[derive(Clone, Serialize)]
pub struct DocumentInfo {
    enabled: bool,
    document: String,
    collection: String,
    description: String,
    total_overrides: usize,
    override_order: Vec<String>,
    default_value: ParamValue,
    value_type: DocumentValueType,
}

#[derive(Clone, Serialize)]
pub struct CollectionInfo {
    collection: String,
    total_documents: usize,
    documents: Vec<DocumentInfo>,
}

#[derive(Clone, Serialize)]
pub struct CollectionList {
    total_collections: usize,
    total_documents: usize,
    collections: Vec<CollectionInfo>,
}

impl From<&Document> for DocumentAttrs {
    fn from(document: &Document) -> Self {
        Self(document.override_attrs())
    }
}

impl From<&Document> for DocumentOverrides {
    fn from(document: &Document) -> Self {
        Self(document.get_overrides())
    }
}

impl From<&Document> for DocumentInfo {
    fn from(document: &Document) -> Self {
        Self {
            document: document.name.clone(),
            collection: document.collection.clone(),
            description: document.description.clone(),
            enabled: document.enabled,
            total_overrides: document.total_overrides(),
            default_value: document.default_value.clone(),
            override_order: document.override_order(),
            value_type: document.value_type.clone(),
        }
    }
}

impl TryFrom<(&Collection, &String, &String)> for DocumentInfo {
    type Error = CollectionError;
    fn try_from(input: (&Collection, &String, &String)) -> Result<Self, Self::Error> {
        let (collection, collection_name, document_name) = input;
        let doc = collection
            .get_document(&collection_name, &document_name)
            .ok_or_else(|| CollectionError::DocumentNotFound(collection_name.clone(), document_name.clone()))?;
        Ok(DocumentInfo::from(doc))
    }
}

///
/// Build a `CollectionInfo` for a specific `collection_name`.
///
impl From<(&Vec<Document>, &String)> for CollectionInfo {
    fn from(request: (&Vec<Document>, &String)) -> Self {
        let (documents, collection_name) = request;
        Self {
            collection: collection_name.clone(),
            total_documents: documents.len(),
            documents: documents.iter().map(|d| DocumentInfo::from(d)).collect::<Vec<DocumentInfo>>(),
        }
    }
}

impl TryFrom<(&Collection, &String)> for CollectionInfo {
    type Error = CollectionError;
    ///
    /// Get a collection's documents or return `CollectionError::CollectionNotFound`
    /// if the collection does not exist.
    ///
    fn try_from(request: (&Collection, &String)) -> Result<Self, Self::Error> {
        let (collection, collection_name) = request;
        let documents = collection.get_documents(&collection_name)
                            .ok_or_else(|| CollectionError::CollectionNotFound(collection_name.clone()))?
                            .iter()
                            .map(|doc| DocumentInfo::from(doc))
                            .collect::<Vec<DocumentInfo>>();
        Ok(Self {
            collection: collection_name.clone(),
            total_documents: documents.len(),
            documents: documents,
        })
    }
}

impl CollectionInfo {
    ///
    /// Get a list of attributes which found from documents in the collection.
    ///
    pub fn attrs(&self) -> Vec<String> {
        let mut attrs = std::collections::HashSet::<&str>::new();
        self.documents.iter().for_each(|doc| {
            doc.override_order.iter().for_each(|order_item|
                order_item.split_terminator(',')
                    .for_each(|attr| { attrs.insert(attr); })
            )
        });
        attrs.iter().map(|it| (*it).into()).collect::<Vec<String>>()
    }
}

impl From<Vec<CollectionInfo>> for CollectionList {
    fn from(list: Vec<CollectionInfo>) -> Self {
        Self {
            total_collections: list.len(),
            total_documents: list.iter().map(|it| it.documents.len()).sum(),
            collections: list,
        }
    }
}

pub enum CollectionResponse {
    DocumentInfo(DocumentInfo),
    DocumentValue(ParamValue),
    DocumentAttrs(DocumentAttrs),
    DocumentNotFound(String, String),   // collection name, document name
    DocumentOverrides(DocumentOverrides),      // document overrides
    CollectionInfo(CollectionInfo),
    CollectionAttrs(Vec<String>),       // list of attributes to look up values from all documents in the collection
    CollectionValues(HashMap<String, ParamValue>),
    Collections(CollectionList),   // all collections
    CollectionNotFound(String),         // collection name
}

impl IntoResponse for CollectionResponse {
    fn into_response(self) -> Response {
        match self {
            CollectionResponse::DocumentInfo(info) => (StatusCode::OK, Json(info)).into_response(),
            CollectionResponse::DocumentValue(value) => (StatusCode::OK, Json(value)).into_response(),
            CollectionResponse::DocumentAttrs(attrs) => (StatusCode::OK, Json(attrs)).into_response(),
            CollectionResponse::DocumentOverrides(overrides) => (StatusCode::OK, Json(overrides)).into_response(),
            CollectionResponse::DocumentNotFound(_, _) => (StatusCode::NOT_FOUND).into_response(),
            CollectionResponse::CollectionInfo(info) => (StatusCode::OK, Json(info)).into_response(),
            CollectionResponse::CollectionAttrs(attrs) => (StatusCode::OK, Json(attrs)).into_response(),
            CollectionResponse::CollectionValues(values) => (StatusCode::OK, Json(values)).into_response(),
            CollectionResponse::Collections(collections) => (StatusCode::OK, Json(collections)).into_response(),
            CollectionResponse::CollectionNotFound(_) => (StatusCode::NOT_FOUND).into_response(),
        }
    }
}

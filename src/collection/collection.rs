use super::{document::{Document, ParamValue, DocumentError}};
use std::{path, collections::HashMap, convert::TryFrom, fmt, iter};
use walkdir::WalkDir;


#[derive(Debug, Clone)]
pub struct Collection {
    // key is a document module, values are documents are elements of the module
    pub documents: HashMap<String, Vec<Document>>,
}

impl Collection {
    pub fn total_collections(&self) -> usize {
        self.documents.len()
    }

    pub fn total_documents(&self) -> usize {
        self.documents.iter().map(|(_, docs)| docs.len()).sum()
    }
    ///
    /// Get a document by a collection name (`module`) and document name (`name`).
    ///
    pub fn get_document(&self, collection_name: &String, name: &String) -> Option<&Document> {
        self.documents.get(collection_name)
            .map_or(None, |docs| docs.iter().find(|d| &d.name == name))
    }
    ///
    /// Get a list of documents by a collection `name`.
    ///
    pub fn get_documents(&self, name: &String) -> Option<&Vec<Document>> {
        self.documents.get(name)
    }

    ///
    /// Look up values for all documents in the collection with name `collection_name`.
    ///
    pub fn get_values(&self, collection_name: &String, attrs: &HashMap<String, String>) -> Option<HashMap<String, ParamValue>> {
        match self.get_documents(&collection_name) {
            Some(documents) => {
                Some(documents.iter().map(|doc| (doc.name.clone(), doc.get_value(&attrs))).collect())
            }
            _ => None,
        }
    }
}

impl TryFrom<(&path::PathBuf, bool)> for Collection {
    type Error = CollectionError;
    ///
    /// Load documents from specified directory.
    /// If `ignore_bad` is true it will return `CollectionError::DocumentError`
    /// otherwise errors will be ignored.
    /// If none documents loaded then `CollectionError::DocumentsNotFound` will be returned.
    ///
    fn try_from(item: (&path::PathBuf, bool)) -> Result<Self, Self::Error> {
        let follow_links = true;
        let (path, ignore_bad) = item;
        let mut this = Self { documents: HashMap::new() };
        let mut total: usize = 0;
        for entry in WalkDir::new(path)
            .follow_links(follow_links)
            .into_iter()
            .filter_map(|e| e.ok()) {
            let f_name = entry.file_name().to_string_lossy();
            if (f_name.ends_with(".yml") || f_name.ends_with(".yaml")) && ! f_name.starts_with(".") {
                match Document::try_from(entry.path()) {
                    Ok(doc) => {
                        total += 1;
                        let documents = this.documents.entry(doc.collection.clone()).or_insert(Vec::new());
                        documents.push(doc);
                    },
                    Err(err) => {
                        tracing::error!("Could not load document {:?} {:?}", &entry, &err);
                        if ! ignore_bad {
                            return Err(CollectionError::DocumentError(err));
                        }
                    }
                }
            }
        }
        match total {
            0 => Err(CollectionError::DocumentsNotFound),
            _ => Ok(this),
        }
    }
}

#[derive(Debug)]
pub enum CollectionError {
    DocumentError(DocumentError),
    DocumentNotFound(String, String),   // collection name, document name
    DocumentsNotFound,
    CollectionNotFound(String),
}

impl From<DocumentError> for CollectionError {
    fn from(inner: DocumentError) -> Self {
        CollectionError::DocumentError(inner)
    }
}

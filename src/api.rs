use crate::{
    config,
    collection::{
        Collection,
        CollectionError,
        collection_router,
        SharedCollection,
    },
};
use tracing::{Level, Span};
use axum::{
    response::{Html},
    routing::{get, IntoMakeService},
    Router,
    Extension,
    middleware::{self, Next},
};
use std::time::{Duration, Instant};
use http::{Response, Request};
use tower_http::{
    trace::{TraceLayer, DefaultOnRequest, DefaultOnResponse, DefaultMakeSpan},
    compression::CompressionLayer,
};
use tower::ServiceBuilder;
use hyper::Body;

pub async fn run_server(args: &config::CliArgs) -> Result<(), ApiError> {
    let log_service = || {
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::DEBUG))
                .on_request(DefaultOnRequest::new().level(Level::DEBUG))
                .on_response(DefaultOnResponse::new().level(Level::INFO)))
    };
    tracing::info!("loading collection from {:?} ...", &args.collection_dir);
    let start_time = Instant::now();
    let collections = Collection::try_from((&args.collection_dir, args.ignore_bad_documents))
        .map_err(|e| ApiError::from(e))?;
    tracing::info!(
        "loaded {} documents from {} collections in {:?}",
        collections.total_documents(), collections.total_collections(), &start_time.elapsed()
    );
    let (total_collections, total_documents) = (collections.total_collections(), collections.total_documents());
    let collections = SharedCollection::from(collections);
    let app = Router::new()
        .nest("/collection", collection_router())
        .layer(log_service())
        .layer(CompressionLayer::new())
        .with_state(collections);
        //.layer(middleware::from_fn(remove_trailing_slash));
    tracing::info!("running server on {:?}", &args.bind);
    axum::Server::bind(&args.bind)
        // .serve(app.route_service())
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn hello() -> Html<&'static str> {
    Html("Hello, world")
}

#[derive(Debug)]
pub enum ApiError {
    CollectionError(CollectionError),
}

impl From<CollectionError> for ApiError {
    fn from(inner: CollectionError) -> Self {
        ApiError::CollectionError(inner)
    }
}

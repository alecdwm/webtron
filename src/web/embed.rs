use rust_embed::RustEmbed;
use warp::http::header::HeaderValue;
use warp::path::Tail;
use warp::reply::Response;
use warp::{Filter, Rejection, Reply};

use crate::web::errors::InternalServerError;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/client/public"]
struct Asset;

pub fn embed() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let index = warp::path::end().and_then(serve_index);
    let path = warp::path::tail().and_then(serve_path);

    index.or(path)
}

async fn serve_index() -> Result<impl Reply, Rejection> {
    serve("index.html")
}

async fn serve_path(path: Tail) -> Result<impl Reply, Rejection> {
    serve(path.as_str())
}

fn serve(path: &str) -> Result<impl Reply, Rejection> {
    let asset = Asset::get(path).ok_or_else(warp::reject::not_found)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut res = Response::new(asset.into());
    res.headers_mut().insert(
        "content-type",
        HeaderValue::from_str(mime.as_ref())
            .map_err(|_| warp::reject::custom(InternalServerError))?,
    );

    Ok(res)
}

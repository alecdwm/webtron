use warp::http::StatusCode;
use warp::reject::Reject;
use warp::{Rejection, Reply};

#[derive(Debug)]
pub struct InternalServerError;
impl Reject for InternalServerError {}

#[derive(Debug)]
pub struct BadGateway;
impl Reject for BadGateway {}

pub async fn handle_rejection(error: Rejection) -> Result<impl Reply, Rejection> {
    if error.is_not_found() {
        return Ok(warp::reply::with_status("Not Found", StatusCode::NOT_FOUND));
    }

    if let Some(InternalServerError) = error.find() {
        return Ok(warp::reply::with_status(
            "Internal Server Error",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    if let Some(BadGateway) = error.find() {
        return Ok(warp::reply::with_status(
            "Bad Gateway",
            StatusCode::BAD_GATEWAY,
        ));
    }

    Err(error)
}

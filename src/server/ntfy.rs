use anyhow::{Context, Error, bail};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use log::warn;

use crate::get_error_chain;

type NtfyClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

///
/// Posts messages to an ntfy topic, such as `https://ntfy.sh/mytopic`.
/// Posts nothing when no topic is set.
///
#[derive(Debug, Clone, Default)]
pub struct Ntfy {
    target: Option<(NtfyClient, Uri)>,
}

impl Ntfy {
    pub fn new(topic_url: Option<&str>) -> Result<Self, Error> {
        let Some(topic_url) = topic_url else {
            return Ok(Self::default());
        };
        let uri: Uri = topic_url
            .parse()
            .with_context(|| format!("Invalid ntfy topic url {:?}", topic_url))?;
        if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.host().is_none() {
            bail!("Invalid ntfy topic url {:?}", topic_url);
        }

        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new()).build(connector);

        Ok(Self {
            target: Some((client, uri)),
        })
    }

    /// Posts `message` to the topic in the background, logging a failure.
    /// Must be called from within a tokio runtime when a topic is set.
    pub fn send(&self, message: String) {
        let Some((client, uri)) = self.target.clone() else {
            return;
        };
        tokio::spawn(async move {
            if let Err(error) = post(&client, uri, message).await {
                warn!("Failed to send ntfy message: {}", get_error_chain(error));
            }
        });
    }
}

async fn post(client: &NtfyClient, uri: Uri, message: String) -> Result<(), Error> {
    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .body(Full::new(Bytes::from(message)))?;
    let response = client.request(request).await?;
    if !response.status().is_success() {
        bail!("ntfy responded with {}", response.status());
    }
    Ok(())
}

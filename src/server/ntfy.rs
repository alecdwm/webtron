use anyhow::{Context, Error, bail};
use bytes::Bytes;
use http_body_util::Full;
use hyper::header::CONTENT_TYPE;
use hyper::{Method, Request, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use log::warn;
use serde_derive::Serialize;

use crate::get_error_chain;

type NtfyClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

///
/// Posts notifications to an ntfy topic, such as `https://ntfy.sh/mytopic`.
/// Posts nothing when no topic is set.
///
#[derive(Debug, Clone, Default)]
pub struct Ntfy {
    target: Option<Target>,
}

#[derive(Debug, Clone)]
struct Target {
    client: NtfyClient,
    /// The server's url that JSON notifications are posted to, the topic url
    /// without its last path segment.
    server: Uri,
    topic: String,
    /// The url that tapping a notification opens.
    click: Option<String>,
    /// The url of the image shown beside a notification.
    icon: Option<String>,
}

/// Starts every notification's title.
const TITLE_PREFIX: &str = "Webtron: ";
/// The client's icon, relative to the public url. ntfy's Android app shows
/// only PNG and JPEG icons.
const ICON_PATH: &str = "logo192.png";

/// One notification.
#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub priority: Priority,
}

/// How insistently ntfy notifies: `Low` makes no sound or vibration.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Priority {
    Low,
    Default,
}

impl serde::Serialize for Priority {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(match self {
            Priority::Low => 2,
            Priority::Default => 3,
        })
    }
}

#[derive(Serialize)]
struct Body<'a> {
    topic: &'a str,
    title: String,
    message: &'a str,
    priority: Priority,
    #[serde(skip_serializing_if = "Option::is_none")]
    click: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<&'a str>,
}

impl<'a> Body<'a> {
    fn new(target: &'a Target, notification: &'a Notification) -> Self {
        Self {
            topic: &target.topic,
            title: format!("{}{}", TITLE_PREFIX, notification.title),
            message: &notification.message,
            priority: notification.priority,
            click: target.click.as_deref(),
            icon: target.icon.as_deref(),
        }
    }
}

impl Ntfy {
    /// Posts to `topic_url`. Tapping a notification opens `public_url`, and
    /// the notification's icon is the client's icon there.
    pub fn new(topic_url: Option<&str>, public_url: Option<&str>) -> Result<Self, Error> {
        let Some(topic_url) = topic_url else {
            return Ok(Self::default());
        };
        let invalid = || format!("Invalid ntfy topic url {:?}", topic_url);
        let uri: Uri = topic_url.parse().with_context(invalid)?;
        let (Some(scheme @ ("http" | "https")), Some(authority)) =
            (uri.scheme_str(), uri.authority())
        else {
            bail!(invalid());
        };
        let path = uri.path().trim_end_matches('/');
        let Some((prefix, topic)) = path.rsplit_once('/').filter(|(_, topic)| !topic.is_empty())
        else {
            bail!(invalid());
        };
        let server: Uri = format!("{}://{}{}/", scheme, authority, prefix)
            .parse()
            .with_context(invalid)?;

        if let Some(url) = public_url {
            check_url(url).with_context(|| format!("Invalid public url {:?}", url))?;
        }
        let click = public_url.map(str::to_string);
        let icon = public_url.map(|url| format!("{}/{}", url.trim_end_matches('/'), ICON_PATH));

        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new()).build(connector);

        Ok(Self {
            target: Some(Target {
                client,
                server,
                topic: topic.to_string(),
                click,
                icon,
            }),
        })
    }

    /// Posts `notification` to the topic in the background, logging a
    /// failure. Must be called from within a tokio runtime when a topic is
    /// set.
    pub fn send(&self, notification: Notification) {
        let Some(target) = self.target.clone() else {
            return;
        };
        tokio::spawn(async move {
            if let Err(error) = post(&target, &notification).await {
                warn!("Failed to send ntfy message: {}", get_error_chain(error));
            }
        });
    }
}

/// Fails unless `url` is an absolute http or https url.
fn check_url(url: &str) -> Result<(), Error> {
    let uri: Uri = url.parse()?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.host().is_none() {
        bail!("not an http or https url");
    }
    Ok(())
}

async fn post(target: &Target, notification: &Notification) -> Result<(), Error> {
    let body = serde_json::to_vec(&Body::new(target, notification))?;
    let request = Request::builder()
        .method(Method::POST)
        .uri(target.server.clone())
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))?;
    let response = target.client.request(request).await?;
    if !response.status().is_success() {
        bail!("ntfy responded with {}", response.status());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(topic_url: &str) -> Option<(String, String)> {
        let target = Ntfy::new(Some(topic_url), None).ok()?.target?;
        Some((target.server.to_string(), target.topic))
    }

    #[test]
    fn topic_urls_split_into_the_server_and_the_topic() {
        let split = |server: &str, topic: &str| Some((server.to_string(), topic.to_string()));
        assert_eq!(
            target("https://ntfy.sh/mytopic"),
            split("https://ntfy.sh/", "mytopic")
        );
        assert_eq!(
            target("https://ntfy.sh/mytopic/"),
            split("https://ntfy.sh/", "mytopic")
        );
        assert_eq!(
            target("http://example.com:8080/ntfy/mytopic"),
            split("http://example.com:8080/ntfy/", "mytopic")
        );
        assert_eq!(target("https://ntfy.sh/"), None);
        assert_eq!(target("ftp://ntfy.sh/mytopic"), None);
    }

    #[test]
    fn notifications_are_posted_as_json_with_the_topic() {
        let notification = Notification {
            title: "Zoë connected".to_string(),
            message: "Nobody else is online".to_string(),
            priority: Priority::Low,
        };
        let body = |ntfy: Ntfy| {
            serde_json::to_string(&Body::new(&ntfy.target.unwrap(), &notification)).unwrap()
        };

        let ntfy = Ntfy::new(Some("https://ntfy.sh/mytopic"), None).unwrap();
        assert_eq!(
            body(ntfy),
            r#"{"topic":"mytopic","title":"Webtron: Zoë connected","message":"Nobody else is online","priority":2}"#
        );

        let ntfy = Ntfy::new(
            Some("https://ntfy.sh/mytopic"),
            Some("https://webtron.example.com/"),
        )
        .unwrap();
        assert_eq!(
            body(ntfy),
            r#"{"topic":"mytopic","title":"Webtron: Zoë connected","message":"Nobody else is online","priority":2,"click":"https://webtron.example.com/","icon":"https://webtron.example.com/logo192.png"}"#
        );

        let ntfy = Ntfy::new(
            Some("https://ntfy.sh/mytopic"),
            Some("https://webtron.example.com"),
        )
        .unwrap();
        assert!(body(ntfy).ends_with(
            r#""click":"https://webtron.example.com","icon":"https://webtron.example.com/logo192.png"}"#
        ));
    }

    #[test]
    fn public_urls_must_be_http_urls() {
        assert!(Ntfy::new(Some("https://ntfy.sh/mytopic"), Some("webtron")).is_err());
        assert!(Ntfy::new(Some("https://ntfy.sh/mytopic"), Some("ftp://example.com")).is_err());
    }
}

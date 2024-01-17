use std::{path::PathBuf, net::SocketAddr};

use axum::{Router, http::{StatusCode, Uri}, BoxError, response::Redirect, extract::Host, handler::HandlerWithoutStateExt};
use axum_server::tls_rustls::RustlsConfig;
use tower_http::services::ServeDir;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod error;
mod search;
mod web;
use crate::error::{Error, Result};

pub fn get_article_path() -> Result<PathBuf> {
    let current_dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            warn!("ERROR: {e}");
            return Err(Error::NotFound);
        }
    };
    Ok(current_dir.join("articles"))
}

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let ports = Ports {
        http: 7878,
        https: 6969,
    };
    tokio::spawn(redirect_http_to_https(ports));
    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("key.pem"),
    )
    .await
    .unwrap();

    info!("Creating Router...");
    let curr_dir_path = std::env::current_dir().unwrap();
    let app = Router::new()
        .merge(web::web::route())
        .nest("/api", web::api::route())
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", curr_dir_path.to_str().unwrap())),
        )
        .nest_service(
            "/images",
            ServeDir::new(format!("{}/images", curr_dir_path.to_str().unwrap())),
        );

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], ports.https));

    info!("router initialized, now listening on port {}", addr);

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}


async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> std::result::Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}

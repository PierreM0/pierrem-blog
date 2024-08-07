use std::{net::SocketAddr, path::PathBuf};

use axum::{
    extract::Host,
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::Redirect,
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use tower_http::services::ServeDir;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod error;
mod search;
mod web;
use crate::error::Result;
use clap::Parser;

/// A simple blog engine with a search functionnality
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Disable TLS encryption and run the engine as HTTP
    #[arg(long, default_value_t = false)]
    notls: bool,

    /// The path to the website folder, there should be a subfolder for article named `articles`, one for images named `images, and one for assets named `assets`
    #[arg(short, long, default_value = ".")]
    path: String,
}

pub fn get_article_path() -> Result<PathBuf> {
    let path = ARGS.get_or_init(Args::parse).path.clone();
    let path = format!("{path}/articles");
    Ok(PathBuf::from(std::ffi::OsStr::new(&path)))
}

const ARGS: std::cell::OnceCell<Args> = std::cell::OnceCell::new();

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ARGS.get_or_init(Args::parse);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let ports = Ports {
        http: 7878,
        https: 6969,
    };

    info!("Creating Router...");
    let app = Router::new()
        .merge(web::web::route())
        .nest("/api", web::api::route())
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", ARGS.get_or_init(Args::parse).path)),
        )
        .nest_service(
            "/images",
            ServeDir::new(format!("{}/images", ARGS.get_or_init(Args::parse).path)),
        );

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], ports.https));
    info!("router initialized, now listening on port {}", addr);

    if !ARGS.get_or_init(Args::parse).notls {
        tokio::spawn(redirect_http_to_https(ports));
        let cert = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("cert.pem");
        let key = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("key.pem");

        if !cert.exists() {
            panic!(
                "The cert file `{}` does not exists",
                cert.to_str().expect("it must be Some")
            );
        };
        if !key.exists() {
            panic!(
                "The key file `{}` does not exists",
                key.to_str().expect("it must be Some")
            );
        };

        let config = RustlsConfig::from_pem_file(cert, key).await.unwrap();

        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await?;
    } else {
        axum_server::bind(addr)
            .serve(app.into_make_service())
            .await?;
    }

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

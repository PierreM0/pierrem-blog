use askama::Template;
use axum::{extract::Path, response::IntoResponse, routing::get, Router};
use tracing::info;

use crate::web::get_name_from_article_path;

use super::HtmlTemplate;

pub fn route() -> Router {
    info!("WEB routing");
    Router::new()
        .route("/articles/:name", get(article_by_name))
        .route("/", get(handle_home))
        .route("/me", get(handle_me))
}

async fn article_by_name(Path(article_file_name): Path<String>) -> impl IntoResponse {
    info!("Handeling web::article_by_name: {article_file_name}");
    #[derive(Template)]
    #[template(path = "article.html")]
    struct ArticleTemplate {
        article_name: String,
        file_name: String,
    }

    let template = ArticleTemplate {
        article_name: get_name_from_article_path(&article_file_name),
        file_name: article_file_name,
    };
    HtmlTemplate(template)
}

async fn handle_home() -> impl IntoResponse {
    info!("Handeling handle_home");
    #[derive(Template)]
    #[template(path = "home.html")]
    struct HomeTemplate;
    HtmlTemplate(HomeTemplate)
}


async fn handle_me() -> impl IntoResponse {
    info!("Handeling handle_me");
    #[derive(Template)]
    #[template(path = "me.html")]
    struct MeTemplate;
    HtmlTemplate(MeTemplate)
}

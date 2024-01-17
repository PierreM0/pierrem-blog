use std::path::{self};

use askama::Template;
use axum::{extract::Path, routing::{get, post}, Router, Form};
use pulldown_cmark::{Options, Parser};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{
    error::{Error, Result},
    web::{article_to_miniarticle, HtmlTemplate},
};

pub fn route() -> Router {
    info!("API routing");
    Router::new()
        .route("/articles/:name", get(article_by_name))
        .route("/last-articles", get(last_articles))
        .route("/search", post(search))
}

#[derive(Template)]
#[template(path="search.html", escape = "none")]
struct SearchTemplate {
    articles: String,
}

#[derive(Debug, Deserialize)]
struct SearchParam {
    terms: Option<String>,
}

async fn search(
    Form(form): Form<SearchParam>
) -> Result<HtmlTemplate<SearchTemplate>> {
    info!("Handeling search with query: {form:?} ");
    let search_terms = form.terms.unwrap_or("defaut search".to_string());
    let mut res = crate::search::search(search_terms.split(" ").map(|s| s.to_owned()).collect::<_>())?;
    let mut html = String::new();
    
    for _ in 0..res.len() {
        let (article, _score) = res.pop().expect("it does exists");
        let article = path::Path::new(&article);
        let miniarticle = article_to_miniarticle(article.to_path_buf())?;
        html.push_str(&miniarticle)
    }

    let template = SearchTemplate {
        articles: html,
    };

    Ok(HtmlTemplate(template))
}

async fn last_articles() -> Result<axum::response::Html<String>> {
    info!("Handeling last_article");

    let current_dir = crate::get_article_path()?
        .as_path()
        .read_dir()
        .map_err(|e| {
            warn!("{e}");
            Error::NotFound
        })?;
    let mut articles = current_dir
        .filter_map(|res| {
            if res.is_ok() {
                Some(res.expect("is_ok"))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    articles.sort_by(|a, b| a.file_name().to_str().cmp(&b.file_name().to_str()));

    let mut html = String::new();
    for article in articles.iter() {
        let mini_article = article_to_miniarticle(article.path())?;
        html.push_str(&mini_article);
    }

    Ok(axum::response::Html(html))
}

async fn article_by_name(
    Path(article_file_name): Path<String>,
) -> Result<axum::response::Html<String>> {
    info!("Handeling api::article_by_name: {article_file_name}");
    let article_path = crate::get_article_path()?.join(article_file_name);

    let article = match std::fs::read_to_string(article_path) {
        Ok(a) => a,
        Err(e) => {
            warn!("ERROR: {e}");
            Err(Error::NotFound)
        }?,
    };

    let parser = Parser::new_ext(&article, Options::all());
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    Ok(axum::response::Html(html_output))
}

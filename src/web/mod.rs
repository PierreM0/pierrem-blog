use std::path::PathBuf;

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use tracing::warn;

use crate::error::{Error, Result};

pub mod api;
pub mod web;

#[derive(Template)]
#[template(path = "mini-article.html")]
struct MiniArticleTemplate {
    content: String,
    name: String,
    file_name: String,
    image_source: String,
    image_alt: String,
}

fn article_to_miniarticle(article: PathBuf) -> Result<String> {
    use pulldown_cmark::CowStr;
    use pulldown_cmark::Event::Start;
    use pulldown_cmark::LinkType;
    use pulldown_cmark::Tag::Image;
    let article_content = std::fs::read_to_string(article.clone()).expect("Read rights");
    let article_content = article_content.chars().take(200).collect::<String>();
    let default_img = Image(LinkType::Inline, CowStr::Borrowed(""), CowStr::Borrowed(""));
    let start_img = pulldown_cmark::Parser::new(&article_content)
        .skip_while(|event| match event {
            Start(Image(_, _, _)) => false,
            _ => true,
        })
        .next()
        .unwrap_or(Start(default_img.clone()));
    let img = match start_img {
        Start(img) => img,
        _ => default_img,
    };

    let Image(_, dest_url, title) = img else {
        panic!("it must be an image")
    };

    let template = MiniArticleTemplate {
        content: format!("{article_content}..."),
        name: get_name_from_article_path(article.file_name().expect("there is a file_name").to_str().expect("name is utf-8")),
        file_name: article.file_name().expect("there is a file_name").to_str().expect("file name is utf-8").to_owned(),
        image_source: dest_url.to_string(),
        image_alt: title.to_string(),
    };

    template.render().map_err(|e| {
        warn!("{e}");
        Error::NotFound
    })
}

fn get_name_from_article_path(path: &str) -> String {
    PathBuf::from(&path)
        .as_path()
        .with_extension("")
        .to_str()
        .expect("modern operating system (utf8 needed)")
        .replace("_", " ")
        .to_string()
}

struct HtmlTemplate<T>(T);
impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> axum::response::Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                warn!("ERROR: Failed to render template: {e}.");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "FAILED TO RENDER TEMPLATE",
                )
                    .into_response()
            }
        }
    }
}

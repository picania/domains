use scraper::Html;
use scraper::Selector;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Site {
    pub title: Option<String>,
    pub charset: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
}

pub fn parse_title(doc: &Html) -> Option<String> {
    let tag = Selector::parse("title").unwrap();

    doc.select(&tag)
        .map(|x| {
            x.text().collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .next()
}

pub fn parse_metadata(doc: &Html) -> Site {
    let meta = Selector::parse("meta").unwrap();
    let mut metadata = Site::default();

    for tag in doc.select(&meta) {
        if let Some(charset) = tag.value().attr("charset") {
            metadata.charset = Some(charset.into());
        }

        if let Some(name) = tag.value().attr("name") {
            if let Some(content) = tag.value().attr("content") {
                if name.eq_ignore_ascii_case("description") {
                    metadata.description = Some(content.into());
                }

                if name.eq_ignore_ascii_case("keywords") {
                    metadata.keywords = Some(content.into());
                }
            }
        }
    }

    metadata
}

#[cfg(test)]
mod test {
    use scraper::Html;
    use super::{parse_title, parse_metadata};

    #[test]
    fn title_exists() {
        let html = r#"
        <html>
            <head>
                <title>Title</title>
            </head>
            <body/>
        </html>
        "#;
        let doc = Html::parse_document(html);

        let title = parse_title(&doc);

        assert_eq!(Some(String::from("Title")), title);
    }

    #[test]
    fn title_not_exists() {
        let html = r#"
        <html>
            <head/>
            <body/>
        </html>
        "#;
        let doc = Html::parse_document(html);

        let title = parse_title(&doc);

        assert_eq!(None, title);
    }

    #[test]
    fn empty_title() {
        let html = r#"
        <html>
            <head/>
                <title/>
            <body/>
        </html>
        "#;
        let doc = Html::parse_document(html);

        let title = parse_title(&doc);

        assert_eq!(None, title);
    }

    #[test]
    fn meta_not_exists() {
        let html = r#"
        <html>
            <head/>
            <body/>
        </html>
        "#;
        let doc = Html::parse_document(html);

        let metadata = parse_metadata(&doc);

        assert_eq!(None, metadata.charset);
        assert_eq!(None, metadata.description);
        assert_eq!(None, metadata.keywords);
    }

    #[test]
    fn meta_exists() {
        let html = r#"
        <html>
            <head>
                <meta charset="utf-8">
                <meta name="description" content="description">
                <meta name="keywords" content="keywords">
                <meta name="description" content="description">
                <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
            </head>
            <body/>
        </html>
        "#;
        let doc = Html::parse_document(html);

        let metadata = parse_metadata(&doc);

        assert_eq!(Some(String::from("utf-8")), metadata.charset);
        assert_eq!(Some(String::from("description")), metadata.description);
        assert_eq!(Some(String::from("keywords")), metadata.keywords);
    }
}

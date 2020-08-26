use reqwest::blocking::Response;
use reqwest::StatusCode;
use reqwest::Result;
use scraper::Html;
use scraper::Selector;

// 1. берем доменное имя
// 2. проверяем наличие сайта на 80 порту
// 3. проверяем наличие сайта на 443 порту
// 4. по редиректам не переходим, отлавливаем адрес перенаправления
// 5. если сайт есть, получаем стартовую страницу
// 6. из стартовой страницы выбираем заголовок и метаинформацию
// 7. метаинформацию разбираем на составляющие
// 8. сохраняем последний обработанный сайт, чтобы продолжить в случае поломок
// 9. сохраняем полученную информацию

struct Meta {
    charset: Option<String>,
    description: Option<String>,
    keywords: Option<String>,
}

fn parse_title(doc: &Html) -> Option<String> {
    let tag = Selector::parse("title").unwrap();

    doc.select(&tag)
        .map(|x| {
            x.text().collect::<String>()
        })
        .next()
}

fn parse_metadata(doc: &Html) -> Meta {
    let meta = Selector::parse("meta").unwrap();
    let mut metadata = Meta{keywords: None, description: None, charset: None};

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
            } else {
                break;
            }
        }
    }

    metadata
}

fn dispatch(response: Response) {
    let url: String = response.url().as_str().into();

    match response.status() {
        StatusCode::OK => {
            if let Ok(text) = response.text() {
                let doc = Html::parse_document(&text);
                let title = parse_title(&doc);
                let meta = parse_metadata(&doc);

                print!("{} ", url);
                title.map_or_else(|| {}, |s| println!("title: {}", s));
                meta.keywords.map_or_else(|| {}, |s| println!("keywords: {}", s));
                meta.description.map_or_else(|| {}, |s| println!("description: {}", s));
                meta.charset.map_or_else(|| {}, |s| println!("charset: {}", s));
            }
        },
        code if code.as_u16() >= 300 && code.as_u16() < 400 => {
            let location = response.headers().get("Location").unwrap();
            println!("{} Redirect code: {} {}", url, code, location.to_str().unwrap());
        },
        code if code.as_u16() >= 400 && code.as_u16() < 500 => {
            println!("{} Client side error: {}", url, code);
        },
        code if code.as_u16() >= 500 && code.as_u16() < 600 => {
            println!("{} Server side error: {}", url, code);
        },
        code => {
            println!("{} Information: {}", url, code);
        }
    }
}

fn main() -> Result<()> {
    let domain = "mvideo.ru";

    match reqwest::blocking::get(format!("https://{}", domain).as_str()) {
        Ok(response) => dispatch(response),
        Err(e) => eprintln!("{} {}", domain, e),
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use scraper::Html;
    use crate::{parse_title, parse_metadata};

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

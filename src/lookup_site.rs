use ::zones::parse_metadata;
use ::zones::parse_title;
use ::zones::Site;
use bson::{doc, Document};
use log::{error, info, warn};
use mongodb::options::FindOptions;
use reqwest::Response;
use reqwest::StatusCode;
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::fmt::{Display, Formatter};

const HTTP: &str = "http://";
const HTTPS: &str = "https://";

type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// 1. Берем из базы 1 доменное имя без поля "lookup".
// 2. Стучимся по адресу по протоколам http и https.
// 3. По каждому протоколу записываем ответ: url, title, charset, description, keywords.
// 4. Сохраняем полученную информацию с отметкой lookup: true, success: true.
// 5. Если веб сервера на доменном имени нет, то отмечаем в базе lookup: true, success: false.

#[derive(Debug, Default, Deserialize, Serialize)]
struct Domain {
    #[serde(default)]
    url: String,
    #[serde(default)]
    http: Option<Site>,
    #[serde(default)]
    https: Option<Site>,
    #[serde(default)]
    lookup: bool,
    #[serde(default)]
    success: bool,
}

impl Domain {
    fn set_http_site(&mut self, site: Site) {
        self.http = Some(site);
        self.success = true;
    }

    fn set_https_site(&mut self, site: Site) {
        self.https = Some(site);
        self.success = true;
    }
}

impl Display for Domain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.url.is_empty() {
            writeln!(f, "url:")?;
        } else {
            writeln!(f, "url: {}", self.url)?;
        }

        writeln!(f, "http:")?;
        if let Some(ref http) = self.http {
            writeln!(f, "    url: {}", http.url)?;
            write!(f, "    title: ")?;
            if let Some(ref title) = http.title {
                writeln!(f, "{}", title)?;
            } else {
                writeln!(f)?;
            }
            write!(f, "    charset: ")?;
            if let Some(ref charset) = http.charset {
                writeln!(f, "{}", charset)?;
            } else {
                writeln!(f)?;
            }
            write!(f, "    description: ")?;
            if let Some(ref desc) = http.description {
                writeln!(f, "{}", desc)?;
            } else {
                writeln!(f)?;
            }
            write!(f, "    keywords: ")?;
            if let Some(ref keywords) = http.keywords {
                writeln!(f, "{}", keywords)?;
            } else {
                writeln!(f)?;
            }
        }

        writeln!(f, "https:")?;
        if let Some(ref https) = self.https {
            writeln!(f, "    url: {}", https.url)?;
            write!(f, "    title: ")?;
            if let Some(ref title) = https.title {
                writeln!(f, "{}", title)?;
            } else {
                writeln!(f)?;
            }
            write!(f, "    charset: ")?;
            if let Some(ref charset) = https.charset {
                writeln!(f, "{}", charset)?;
            } else {
                writeln!(f)?;
            }
            write!(f, "    description: ")?;
            if let Some(ref desc) = https.description {
                writeln!(f, "{}", desc)?;
            } else {
                writeln!(f)?;
            }
            write!(f, "    keywords: ")?;
            if let Some(ref keywords) = https.keywords {
                writeln!(f, "{}", keywords)?;
            } else {
                writeln!(f)?;
            }
        }

        writeln!(f, "lookup: {}", self.lookup)?;
        writeln!(f, "success: {}", self.success)
    }
}

async fn dispatch(response: Response) -> Option<Site> {
    let url = response.url().to_string();
    match response.status() {
        StatusCode::OK => {
            if let Ok(text) = response.text().await {
                let doc = Html::parse_document(&text);
                let mut site = parse_metadata(&doc);

                site.url = url;
                site.title = parse_title(&doc);

                Some(site)
            } else {
                None
            }
        }
        _ => {
            warn!(
                "{} -- Get response with code: {}",
                url,
                response.status().as_u16()
            );
            None
        }
    }
}

fn error_dispatch(domain: &str, err: reqwest::Error) {
    if err.is_request() {
        error!("{} -- FAILED -- Error sending request", domain);
    }

    if err.is_timeout() {
        error!("{} -- FAILED -- Time out", domain);
    }

    if err.is_status() {
        error!(
            "{} -- FAILED -- Http {} error",
            domain,
            err.status().unwrap().as_u16()
        );
    }
}

async fn lookup_site(client: reqwest::Client, proto: &str, url: String) -> Option<Site> {
    let url = format!("{}{}", proto, url);
    let request = client.get(&url).build().unwrap();

    match client.execute(request).await {
        Ok(response) => dispatch(response).await,
        Err(err) => {
            error_dispatch(&url, err);
            None
        }
    }
}
// async fn join_all<T>(mut handlers: Vec<T>)
// where
//     T: FusedFuture + Future + Unpin,
// {
//     loop {
//         for handle in &mut handlers {
//             handle.await;
//         }
//
//         let remain = handlers.iter().filter(|x| !x.is_terminated()).count();
//         if remain == 0 {
//             break;
//         }
//     }
// }

async fn lookup_sites(client: mongodb::sync::Client, db: &str) {
    let www = reqwest::ClientBuilder::default()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let db = client.database(db);
    let coll = db.collection("domains");
    let options = FindOptions::builder().limit(Some(1)).build();
    let filter = doc! {"lookup": {"$exists": false}, "removed": {"$exists": false}};

    let mut db_errors: usize = 0;
    loop {
        let cursor = coll.find(filter.clone(), options.clone());

        match cursor {
            Ok(cursor) => {
                let domain = cursor
                    .filter_map(Result::ok)
                    .filter_map(|doc: Document| {
                        bson::from_bson::<Domain>(bson::Bson::Document(doc)).ok()
                    })
                    .next();

                if let Some(mut domain) = domain {
                    let timer = tokio::time::delay_for(Duration::from_millis(100));

                    let http = tokio::spawn(lookup_site(www.clone(),HTTP, domain.url.clone()));
                    let https = tokio::spawn(lookup_site(www.clone(),HTTPS, domain.url.clone()));

                    let (http, https) = tokio::join!(http, https);
                    timer.await; // Задержка на 100 мс, чтобы было не более 10 запросов в секунду.

                    if let Ok(Some(site)) = http {
                        domain.set_http_site(site);
                    }

                    if let Ok(Some(site)) = https {
                        domain.set_https_site(site);
                    }

                    domain.lookup = true;

                    if domain.success {
                        info!("Look up domain {}\n{}", domain.url, domain);
                    }

                    let query = doc! {"url": &domain.url};
                    let update = bson::to_document(&domain).unwrap();
                    let result = coll.update_one(query, update, None);
                    if let Err(err) = result {
                        warn!("{}", err);
                        db_errors += 1;
                    }
                }
            }
            Err(err) => {
                error!("{}", err);
                db_errors += 1;
            }
        }

        if db_errors == 10 {
            break;
        }
    }
}

#[tokio::main(core_threads = 4)]
async fn main() -> BoxResult<()> {
    pretty_env_logger::init_timed();

    let uri = std::env::var("MONGODB_URI").map_err(|x| {
        error!("You must set MONGODB_URI environment variable");
        x
    })?;
    let client = mongodb::sync::Client::with_uri_str(&uri)?;

    let ru = tokio::spawn(lookup_sites(client.clone(), "ru_zone"));
    let su = tokio::spawn(lookup_sites(client.clone(), "su_zone"));
    let rf = tokio::spawn(lookup_sites(client.clone(), "rf_zone"));

    let (ru, su, rf) = tokio::join!(ru, su, rf);
    let _ = ru?;
    let _ = su?;
    let _ = rf?;

    Ok(())
}

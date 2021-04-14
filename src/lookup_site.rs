use reqwest::Response;
use reqwest::StatusCode;
use scraper::Html;
use log::{error, info, warn};
use bson::Document;
use serde::{Deserialize, Serialize};
use futures::stream::StreamExt;
use ::zones::Site;
use ::zones::parse_title;
use ::zones::parse_metadata;
use mongodb::Collection;
use std::time::Duration;

// 1. Берем из базы 10 непроверенных доменных имен. Допустим, проверенные будут обозначаться полем "lookup": true.
// 2. Стучимся по адресу по протоколам http и https.
// 3. По каждому протоколу записываем url откуда пришел ответ. Это поможет отследить перенаправления.
// 4. Если сервер ответил, то из стартовой страницы выбираем заголовок и метаинформацию
// 5. Метаинформацию разбираем на составляющие
// 6. Сохраняем полученную информацию с отметкой lookup

#[derive(Debug, Default, Deserialize, Serialize)]
struct Domain {
    #[serde(default)]
    url: String,
    #[serde(default)]
    redirect: Option<Site>,
    #[serde(default)]
    http: Option<Site>,
    #[serde(default)]
    https: Option<Site>,
    #[serde(default)]
    lookup: bool,
}

async fn dispatch(response: Response) {
    match response.status() {
        StatusCode::OK => {
            if let Ok(text) = response.text().await {
                let doc = Html::parse_document(&text);
                let mut site = parse_metadata(&doc);
                site.title = parse_title(&doc);

                info!("{:?}", site);
            }
        },
        _ => warn!("Get response with code: {}", response.status().as_u16()),
        // code if code >= StatusCode::MULTIPLE_CHOICES && code < StatusCode::BAD_REQUEST => {
        //     let location = response.headers().get("Location").unwrap();
        //     info!("Redirect: {} {}", code, location.to_str().unwrap());
        // },
        // code if code >= StatusCode::BAD_REQUEST && code < StatusCode::INTERNAL_SERVER_ERROR => {
        //     info!("Client side error: {}", code);
        // },
        // code if code >= StatusCode::INTERNAL_SERVER_ERROR && code <= StatusCode::NETWORK_AUTHENTICATION_REQUIRED => {
        //     info!("Server side error: {}", code);
        // },
        // code => {
        //     info!("Information: {}", code);
        // }
    }
}

type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn error_dispatch(proto: &str, err: reqwest::Error) {
    if err.is_request() {
        error!("{} -- FAILED -- Error sending request", proto);
    }

    if err.is_timeout() {
        error!("{} -- FAILED -- Time out", proto);
    }

    if err.is_status() {
        error!("{} -- FAILED -- Http {} error", proto, err.status().unwrap().as_u16());
    }
}

fn lookup_domain(proto: &str, domain: &Domain) {
    info!("Look up domain {}\n{:#?}", &format!("{}/{}", proto, domain.url), domain);
}

async fn batch_requests(coll: Collection) {
    const BATCH_SIZE: usize = 10; // Не более 10 запросов в секунду
    let timer = tokio::time::delay_for(Duration::from_secs(1));

    let records = coll.find(None, None).await;

    match records {
        Ok(cursor) => {
            cursor.take(BATCH_SIZE)
                .filter_map(|x| {
                    async { x.ok() }
                })
                .filter_map(|doc: Document| {
                    async move {
                        bson::from_bson::<Domain>(bson::Bson::Document(doc)).ok()
                    }
                })
                .for_each(|domain| {
                    async move {
                        //info!("Connect to '{}' server ...", domain.url);
                        let http = "http://";
                        let https = "https://";

                        lookup_domain(http, &domain);
                        lookup_domain(https, &domain);
                        // match reqwest::get(&format!("{}{}", http, &domain.url)).await {
                        //     Ok(response) => dispatch(response).await,
                        //     Err(err) => error_dispatch(http, err),
                        // }
                        //
                        // match reqwest::get(&format!("{}{}", https, &domain.url)).await {
                        //     Ok(response) => dispatch(response).await,
                        //     Err(err) => error_dispatch(https, err),
                        // }
                    }
                }).await;
        }
        Err(err) => {
            error!("{}", err);
        }
    }

    timer.await;
}

async fn lookup_sites(client: mongodb::Client, db: &str) {
    let db = client.database(db);
    let coll = db.collection("domains");

    batch_requests(coll).await;
}

#[tokio::main(core_threads=4)]
async fn main() -> BoxResult<()> {
    pretty_env_logger::init_timed();

    let uri = std::env::var("MONGODB_URI").map_err(|x|{
        error!("You must set MONGODB_URI environment variable");
        x
    })?;
    let client = mongodb::Client::with_uri_str(&uri).await?;

    let ru = tokio::spawn(lookup_sites(client.clone(), "ru_zone"));
    let su = tokio::spawn(lookup_sites(client.clone(), "su_zone"));
    let rf = tokio::spawn(lookup_sites(client.clone(), "rf_zone"));

    let (r1, r2, r3) = tokio::join!(ru, su, rf);

    Ok(())
}

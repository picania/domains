#![allow(dead_code)]

use reqwest::{blocking, IntoUrl, Url};
use std::fs::File;
use std::io::{self, BufWriter, BufReader, BufRead};
use std::path::Path;
use std::fmt::Display;
use std::str::FromStr;
use bson::doc;
use flate2::read::GzDecoder;
use log::{info, error};

type BoxResult<T, E = Box<dyn std::error::Error>> = Result<T, E>;

fn download_database<U, P>(url: U, path: P) -> BoxResult<()>
where
    U: IntoUrl + Display,
    P: AsRef<Path>,
{
    info!("Start download {} ... ", url);
    let mut response = blocking::get(url)?;

    response.content_length().map_or_else(|| {}, |x| info!("Content {} bytes length.", x));

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let bytes = io::copy(&mut response, &mut writer)?;

    info!("Download {} bytes", bytes);

    Ok(())
}

mod collections {
    pub const DOMAINS: &str = "domains";
    pub const UNRESOLVED: &str = "unresolved";
    pub const RESOLVED: &str = "resolved";
    pub const WITH_ERRORS: &str = "with_errors";
}

const MONGODB: &str = "mongodb://192.168.1.137";
const MONGODB_CHUNK_SIZE: usize = 100_000;

fn update_database<P>(base: &str, path: P) -> BoxResult<()>
where
    P: AsRef<Path>,
{
    info!("Connect to '{}' database ...", base);
    let client = mongodb::sync::Client::with_uri_str(MONGODB)?;
    let database = client.database(base);
    let domains = database.collection(collections::DOMAINS);
    let reader = BufReader::new(GzDecoder::new(File::open(path)?));

    // сбрасываем коллекцию
    info!("Reset '{}' collection ...", collections::DOMAINS);
    if let Err(e) = domains.drop(None) {
        error!("{}", e);
    }
    let domains = database.collection(collections::DOMAINS);

    info!("Prepare documents ...");
    let docs = reader.lines()
        .filter_map(|x| x.ok())
        .map(|s| {
            s.split_ascii_whitespace().take(1).collect::<String>()
        })
        .map(|domain| {
            doc!{"url": domain}
        })
        .collect::<Vec<_>>();

    info!("Insert documents ...");
    let mut records = 0 as usize;
    docs.into_iter().as_slice().chunks(MONGODB_CHUNK_SIZE)
        .for_each(|chunk| {
            match domains.insert_many(chunk.to_vec(), None) {
                Ok(res) => {
                    records += res.inserted_ids.len();
                },
                Err(e) => {
                    error!("{}", e);
                }
            }
        });

    info!("Inserted {} records into {}", records, base);

    Ok(())
}

fn main() -> BoxResult<()> {
    pretty_env_logger::init_timed();

    let urls = vec![
        (Url::from_str("https://ru-tld.ru/files/RU_Domains_ru-tld.ru.gz")?, Path::new("zones/ru_domains.gz")),
        (Url::from_str("https://ru-tld.ru/files/SU_Domains_ru-tld.ru.gz")?, Path::new("zones/su_domains.gz")),
        (Url::from_str("https://ru-tld.ru/files/RF_Domains_ru-tld.ru.gz")?, Path::new("zones/rf_domains.gz")),
    ];

    for (url, path) in urls {
        download_database(url, path)?;
    }

    let bases = vec![
        ("ru_zone", Path::new("zones/ru_domains.gz")),
        ("su_zone", Path::new("zones/su_domains.gz")),
        ("rf_zone", Path::new("zones/rf_domains.gz")),
    ];

    for (base, path) in bases {
        update_database(base, path)?;
    }

    Ok(())
}

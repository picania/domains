use ::zones::symmetric_diff;
use bson::{doc, Document};
use flate2::read::GzDecoder;
use log::{error, info};
use mongodb::options::FindOptions;
use mongodb::sync::{Client, Collection};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

type BoxResult<T, E = Box<dyn std::error::Error>> = Result<T, E>;

const DOMAINS: &str = "domains";
const REMOVED: &str = "removed";
const FIND_KEY: &str = "url";
const MONGODB_CHUNK_SIZE: usize = 100_000;

fn update_database<P>(client: &Client, base: &str, path: P) -> BoxResult<()>
where
    P: AsRef<Path>,
{
    info!("Connect to '{}' database ...", base);

    let db = client.database(base);
    let coll = db.collection(DOMAINS);
    let reader = BufReader::new(GzDecoder::new(File::open(path)?));

    // Получаем курсор на коллекцию в базе данных. Сортируем выдачу по полю 'url' по алфавиту.
    let options = FindOptions::builder()
        .sort(Some(doc! {FIND_KEY: 1}))
        .build();
    let records = coll
        .find(None, Some(options))?
        .filter_map(Result::ok)
        .map(|doc: Document| {
            doc.iter()
                .filter(|(key, _)| **key == FIND_KEY)
                .filter_map(|(_, value)| value.as_str())
                .collect::<String>()
        });

    // Получаем итератор на строки в файле. Строки в файле уже отсортированы по алфавиту.
    let registry = reader.lines().filter_map(Result::ok).map(|s: String| {
        s.split_ascii_whitespace()
            .take(1)
            .next()
            .unwrap()
            .to_string()
    });

    let (added, removed) = symmetric_diff(registry, records);

    upload_records(&coll, &added);
    update_removed_records(&coll, &removed);

    Ok(())
}

fn update_removed_records(coll: &Collection, domains: &[String]) {
    let update = doc! {"$set": {REMOVED: true}};
    let mut updated = 0;

    domains.iter().for_each(|domain| {
        let query = doc! {FIND_KEY: domain};

        let result = coll.update_one(query, update.clone(), None);

        if let Ok(result) = result {
            updated += result.modified_count;
        }
    });

    info!("Updated {} domains", updated);
}

fn upload_records(coll: &Collection, domains: &[String]) {
    let mut added = 0;

    domains.chunks(MONGODB_CHUNK_SIZE).for_each(|domains| {
        let docs = domains
            .iter()
            .map(|domain| doc! {FIND_KEY: domain})
            .collect::<Vec<_>>();

        let result = coll.insert_many(docs, None);

        if let Ok(result) = result {
            added += result.inserted_ids.len();
        }
    });

    info!("Added {} domains", added);
}

fn main() -> BoxResult<()> {
    pretty_env_logger::init_timed();

    let uri = std::env::var("MONGODB_URI").map_err(|x|{
        error!("You must set MONGODB_URI environment variable");
        x
    })?;
    let client = mongodb::sync::Client::with_uri_str(&uri)?;

    let bases = vec![
        ("ru_zone", Path::new("zones/ru_domains.gz")),
        ("su_zone", Path::new("zones/su_domains.gz")),
        ("rf_zone", Path::new("zones/rf_domains.gz")),
    ];

    for (base, path) in &bases {
        update_database(&client, base, path)?;
    }

    Ok(())
}

use reqwest::blocking;
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;
use log::{info};

type BoxResult<T, E = Box<dyn std::error::Error>> = Result<T, E>;

fn download_database(url: &str, path: &Path) -> BoxResult<()>
{
    info!("Start download {} ... ", url);
    let mut response = blocking::get(url)?;

    response.content_length()
        .map_or_else(|| {}, |x| info!("Content {} bytes length.", x));

    let mut writer = BufWriter::new(File::create(path)?);
    let bytes = io::copy(&mut response, &mut writer)?;

    info!("Download {} bytes", bytes);

    Ok(())
}

fn main() -> BoxResult<()> {
    pretty_env_logger::init_timed();

    let urls = vec![
        ("https://ru-tld.ru/files/RU_Domains_ru-tld.ru.gz", Path::new("zones/ru_domains.gz")),
        ("https://ru-tld.ru/files/SU_Domains_ru-tld.ru.gz", Path::new("zones/su_domains.gz")),
        ("https://ru-tld.ru/files/RF_Domains_ru-tld.ru.gz", Path::new("zones/rf_domains.gz")),
    ];

    for (url, path) in urls {
        download_database(url, path)?;
    }

    Ok(())
}

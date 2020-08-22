#![allow(dead_code)]

use reqwest::blocking;
use std::fs::File;
use std::io::BufWriter;

fn download_database(url: &str, name: &str) {
    print!("Start download {} ... ", url);
    let mut resp = blocking::get(url).unwrap();

    match resp.content_length() {
        Some(size) => println!("Content {} bytes length.", size),
        None => println!(),
    }

    let file = File::create(format!("zones/{}", name)).unwrap();
    let mut writer = BufWriter::new(file);

    let bytes = resp.copy_to(&mut writer).unwrap();

    println!("Download {} bytes", bytes);
}

fn main() {
    let urls = vec![
        ("https://ru-tld.ru/files/RU_Domains_ru-tld.ru.gz", "ru_domains.gz"),
        ("https://ru-tld.ru/files/SU_Domains_ru-tld.ru.gz", "su_domains.gz"),
        ("https://ru-tld.ru/files/RF_Domains_ru-tld.ru.gz", "rf_domains.gz"),
    ];

    for (url, name) in urls {
        download_database(url, name);
    }
}

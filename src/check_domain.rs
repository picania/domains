use std::fs::File;
use std::io::{Read, BufReader, BufRead, BufWriter, Write};
use std::cmp::max;
use flate2::read::GzDecoder;
use trust_dns_resolver::Resolver;
use trust_dns_resolver::config::*;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

const DECOMPRESS_SIZE: usize = 400 * 1024 * 1024; // 400 Mb

// корневые сервера России
// f.root (Москва — 2 шт.);
// i.root (Санкт-Петербург);
// j.root (Москва, Санкт-Петербург);
// k.root (Москва, Санкт-Петербург, Новосибирск);
// l.root (Москва, Ростов-на-Дону, Екатеринбург).
// корневой DNS в Новосибирске
// IPv4	193.0.14.129
const NOVOSIBIRSK: &str = "193.0.14.129";
// IPv6	2001:7fd::1
// корневой DNS в Екатеринбурге
// IPv4	199.7.83.42
const EKATERINBURG: &str = "199.7.83.42";
// IPv6	2001:500:9f::42

fn main() {
    // загружаем архив в память
    let mut file = File::open("zones/ru_domains.gz").unwrap();
    let file_len = file.metadata().unwrap().len() as usize;
    let mut gz_buffer = Vec::<u8>::with_capacity(file_len);

    let total = file.read_to_end(&mut gz_buffer).unwrap();
    println!("Archived size: {}", total);

    // распаковываем архив в буфер
    let mut dec = GzDecoder::new(gz_buffer.as_slice());
    let mut out_buffer = String::with_capacity(DECOMPRESS_SIZE);

    let unpacked = dec.read_to_string(&mut out_buffer).unwrap();
    println!("Unpacked size: {}", unpacked);

    // читаем из буфера построчно
    let reader = BufReader::new(out_buffer.as_bytes());

    let out = File::create("zones/ru_domains.txt").unwrap();
    let mut writer = BufWriter::new(out);

    let mut max_domain = 0usize;
    let mut domains_count = 0usize;

    // создаем резолвер для DNS
    let root1 = IpAddr::V4(Ipv4Addr::from_str(NOVOSIBIRSK).unwrap());
    let root2 = IpAddr::V4(Ipv4Addr::from_str(EKATERINBURG).unwrap());
    let group = NameServerConfigGroup::from_ips_clear(&[root1, root2], 53);
    let mut config = ResolverConfig::from_parts(None, vec![], group);
    let resolver = Resolver::new(config, ResolverOpts::default()).unwrap();

    reader.lines()
        .filter_map(|x| x.ok())
        .map(|s| {
            s.chars().take_while(|x| !x.is_ascii_whitespace()).collect::<String>()
        })
        .take(1000)
        .for_each(|domain| {
            let bytes = domain.len();
            max_domain = max(max_domain, bytes);
            writer.write_all(domain.as_bytes()).unwrap();
            writer.write_all("\n".as_bytes()).unwrap();
            domains_count += 1;

            // разрешаем доменное имя в адрес
            print!("Resolve DNS for '{}' ... ", domain);
            std::io::stdout().flush().unwrap();
            match resolver.lookup_ip(&domain) {
                Ok(ips) => {
                    let addr_count = ips.iter().count();
                    println!("Found {} ip-addresses", addr_count);
                    if addr_count > 0 {
                        print!("IP-addr for '{}':        ", domain);
                        for x in ips {
                            print!("{} ", x.to_string());
                        }
                        println!();
                    }
                },
                Err(e) => println!("{}", e),
            }

            //println!("{}", x);
        });

    println!("Domains: {}", domains_count);
    println!("Max domain name: {}", max_domain);
}

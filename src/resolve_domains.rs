use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::IpAddr;
use trust_dns_resolver::config::*;
use trust_dns_resolver::Resolver;

// Root servers located in Russian Federation
const E_ROOT_V4: [u8; 4] = [192, 203, 230, 10];
// E IPv6	2001:500:a8::e
const F_ROOT_V4: [u8; 4] = [192, 5, 5, 241];
// F IPv6	2001:500:2f::f
const J_ROOT_V4: [u8; 4] = [192, 58, 128, 30];
// J IPv6	2001:503:c27::2:30
const I_ROOT_V4: [u8; 4] = [192, 36, 148, 17];
// I IPv6	2001:7fe::53
const K_ROOT_V4: [u8; 4] = [193, 0, 14, 129];
// IPv6	2001:7fd::1
const L_ROOT_V4: [u8; 4] = [199, 7, 83, 42];
// IPv6	2001:500:9f::42

fn create_resolver() -> io::Result<Resolver> {
    let ips = [
        IpAddr::V4(E_ROOT_V4.into()),
        IpAddr::V4(F_ROOT_V4.into()),
        IpAddr::V4(J_ROOT_V4.into()),
        IpAddr::V4(I_ROOT_V4.into()),
        IpAddr::V4(K_ROOT_V4.into()),
        IpAddr::V4(L_ROOT_V4.into()),
    ];
    let group = NameServerConfigGroup::from_ips_clear(&ips, 53);
    let options = ResolverOpts {
        ip_strategy: LookupIpStrategy::Ipv4Only,
        attempts: ips.len(),
        rotate: true,
        ..ResolverOpts::default()
    };
    let config = ResolverConfig::from_parts(None, vec![], group);

    Resolver::new(config, options)
}

fn extract_domains(from: &str, to: &str) -> io::Result<()> {
    let input = File::open(from)?;
    let output = File::create(to)?;
    let decoder = GzDecoder::new(input);
    let reader = BufReader::new(decoder);
    let mut writer = BufWriter::new(output);

    reader
        .lines()
        .filter_map(|x| x.ok())
        .map(|s| s.split_ascii_whitespace().take(1).collect::<String>())
        .for_each(|domain| {
            writer.write_fmt(format_args!("{}\n", domain)).unwrap();
        });

    Ok(())
}

fn resolve_domains(unresolved: &str, resolved: &str) -> io::Result<()> {
    let resolver = create_resolver()?;
    let input = File::open(unresolved)?;
    let output = File::create(resolved)?;
    let reader = BufReader::new(input);
    let mut writer = BufWriter::new(output);
    let mut unr_writer = BufWriter::new(File::create("zones/unresolved.ru.tmp")?);

    reader
        .lines()
        .filter_map(|x| x.ok())
        .take(1000)
        .for_each(|domain| match resolver.lookup_ip(&domain) {
            Ok(ips) => {
                writer.write_all(domain.as_bytes()).unwrap();
                print!("{}", &domain);
                for ip in ips {
                    writer.write_fmt(format_args!(" {}", &ip)).unwrap();
                    print!(" {}", &ip);
                }
                writer.write_all(b"\n").unwrap();
                println!();
                io::stdout().flush().unwrap();
            }
            Err(_) => {
                unr_writer.write_fmt(format_args!("{}\n", domain)).unwrap();
            }
        });

    fs::copy("zones/unresolved.ru.tmp", "zones/unresolved.ru")?;
    fs::remove_file("zones/unresolved.ru.tmp")?;

    Ok(())
}

fn main() -> io::Result<()> {
    loop {
        if fs::metadata("zones/unresolved.ru").is_ok() {
            resolve_domains("zones/unresolved.ru", "zones/resolved.ru")?;
            break;
        } else {
            extract_domains("zones/ru_domains.gz", "zones/domains.ru")?;
            fs::copy("zones/domains.ru", "zones/unresolved.ru")?;
        }
    }

    Ok(())
}

use std::net::IpAddr;

use clap::{crate_name, crate_version};

use crate::ip_retriever;

#[derive(Debug)]
pub struct Args {
    pub record: String,
    pub domain: String,
    pub token: String,
    pub ip: IpAddr,
    pub rtype: String,
    pub ttl: u16,
    pub quiet: bool,
    pub dry_run: bool,
}

impl Args {
    pub fn parse_args() -> Args {
        let matches = clap::Command::new(crate_name!())
            .version(crate_version!())
            .author("Chris Lieb")
            .arg(
                clap::Arg::new("RECORD")
                    .required(true)
                    .takes_value(true)
                    .help("The DNS record within the domain to update"),
            )
            .arg(
                clap::Arg::new("DOMAIN")
                    .required(true)
                    .takes_value(true)
                    .help("The domain that has the record to update"),
            )
            .arg(
                clap::Arg::new("token")
                    .short('t')
                    .long("token")
                    .takes_value(true)
                    .env("DIGITAL_OCEAN_TOKEN")
                    .help("The API token to use to auth with DigitalOcean"),
            )
            .arg(
                clap::Arg::new("local")
                    .short('l')
                    .long("local")
                    .takes_value(false)
                    .conflicts_with("ip")
                    .help("Use the local IP address connected to the internet"),
            )
            .arg(
                clap::Arg::new("ip")
                    .long("ip")
                    .takes_value(true)
                    .conflicts_with("local")
                    .validator(|val| match val.parse::<IpAddr>() {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e.to_string()),
                    })
                    .help("Use this IP address when updating the record"),
            )
            .arg(
                clap::Arg::new("rtype")
                    .long("rtype")
                    .takes_value(true)
                    .possible_values(&["A", "AAAA"])
                    .default_value("A")
                    .help("The type of DNS record to set"),
            )
            .arg(
                clap::Arg::new("ttl")
                    .long("ttl")
                    .takes_value(true)
                    .default_value("60")
                    .validator(|val| match val.parse::<u16>() {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e.to_string()),
                    })
                    .help("The TTL for the new DNS record"),
            )
            .arg(
                clap::Arg::new("quiet")
                    .short('q')
                    .long("quiet")
                    .takes_value(false)
                    .help("Only display output on IP change"),
            )
            .arg(
                clap::Arg::new("dry_run")
                    .short('n')
                    .long("dry-run")
                    .takes_value(false)
                    .help("Do everything except actually set the record"),
            )
            .get_matches();

        let literal_ip = matches
            .value_of("ip")
            .map(|x| x.parse::<IpAddr>().expect("Unable to parse IP address"));
        let local = matches.is_present("local");
        let rtype = matches.value_of("rtype").unwrap().to_string();

        let ip = if let Some(lit) = literal_ip {
            lit
        } else if local {
            ip_retriever::get_local_ip().expect("Unable to retrieve local IP address")
        } else {
            let ip =
                ip_retriever::get_external_ip().expect("Unable to retrieve external IP address");
            if (ip.is_ipv4() && rtype != "A") || (ip.is_ipv6() && rtype != "AAAA") {
                panic!("Expected Rtype {} but got {:?}", rtype, ip)
            }
            ip
        };
        info!("Will publish IP address: {:?}", ip);

        Args {
            record: matches.value_of("RECORD").unwrap().to_string(),
            domain: matches.value_of("DOMAIN").unwrap().to_string(),
            token: matches.value_of("token").unwrap().to_string(),
            ip,
            rtype,
            ttl: matches
                .value_of("ttl")
                .unwrap()
                .parse::<u16>()
                .expect("Must provide integer for ttl"),
            quiet: matches.is_present("quiet"),
            dry_run: matches.is_present("dry_run"),
        }
    }
}

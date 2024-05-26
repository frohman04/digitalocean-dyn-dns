use std::net::IpAddr;

use clap::{crate_name, crate_version};
use tracing::info;

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
                    .num_args(1)
                    .help("The DNS record within the domain to update"),
            )
            .arg(
                clap::Arg::new("DOMAIN")
                    .required(true)
                    .num_args(1)
                    .help("The domain that has the record to update"),
            )
            .arg(
                clap::Arg::new("token")
                    .short('t')
                    .long("token")
                    .num_args(1)
                    .env("DIGITAL_OCEAN_TOKEN")
                    .help("The API token to use to auth with DigitalOcean"),
            )
            .arg(
                clap::Arg::new("local")
                    .short('l')
                    .long("local")
                    .num_args(0)
                    .conflicts_with("ip")
                    .help("Use the local IP address connected to the internet"),
            )
            .arg(
                clap::Arg::new("ip")
                    .long("ip")
                    .num_args(1)
                    .conflicts_with("local")
                    .value_parser(clap::value_parser!(IpAddr))
                    .help("Use this IP address when updating the record"),
            )
            .arg(
                clap::Arg::new("rtype")
                    .long("rtype")
                    .num_args(1)
                    .value_parser(["A", "AAAA"])
                    .default_value("A")
                    .help("The type of DNS record to set"),
            )
            .arg(
                clap::Arg::new("ttl")
                    .long("ttl")
                    .num_args(1)
                    .default_value("60")
                    .value_parser(clap::value_parser!(u16))
                    .help("The TTL for the new DNS record"),
            )
            .arg(
                clap::Arg::new("quiet")
                    .short('q')
                    .long("quiet")
                    .num_args(0)
                    .help("Only display output on IP change"),
            )
            .arg(
                clap::Arg::new("dry_run")
                    .short('n')
                    .long("dry-run")
                    .num_args(0)
                    .help("Do everything except actually set the record"),
            )
            .get_matches();

        let literal_ip = matches.get_one::<IpAddr>("ip");
        let local = matches.get_flag("local");
        let rtype = matches.get_one::<String>("rtype").unwrap().clone();

        let ip = if let Some(lit) = literal_ip {
            info!("Using user-provided IP address: {}", lit);
            *lit
        } else if local {
            info!("Getting local IP address of machine...");
            ip_retriever::get_local_ip().expect("Unable to retrieve local IP address")
        } else {
            info!("Getting public IP address of machine...");
            let ip =
                ip_retriever::get_external_ip().expect("Unable to retrieve external IP address");
            if (ip.is_ipv4() && rtype != "A") || (ip.is_ipv6() && rtype != "AAAA") {
                panic!("Expected Rtype {rtype} but got {ip:?}")
            }
            ip
        };
        info!("Will publish IP address: {:?}", ip);

        Args {
            record: matches.get_one::<String>("RECORD").unwrap().clone(),
            domain: matches.get_one::<String>("DOMAIN").unwrap().clone(),
            token: matches.get_one::<String>("token").unwrap().clone(),
            ip,
            rtype,
            ttl: *matches
                .get_one::<u16>("ttl")
                .expect("Must provide integer for ttl"),
            quiet: matches.get_flag("quiet"),
            dry_run: matches.get_flag("dry_run"),
        }
    }
}

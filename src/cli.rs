use std::net::IpAddr;

use clap::{crate_name, crate_version};

#[derive(Debug)]
pub struct Args {
    pub record: String,
    pub domain: String,
    pub token: String,
    pub local: bool,
    pub ip: Option<IpAddr>,
    pub rtype: String,
    pub ttl: u16,
    pub quiet: bool,
    pub dry_run: bool,
}

impl Args {
    pub fn parse_args() -> Args {
        let matches = clap::App::new(crate_name!())
            .version(crate_version!())
            .author("Chris Lieb")
            .arg(
                clap::Arg::with_name("RECORD")
                    .required(true)
                    .takes_value(true)
                    .help("The DNS record within the domain to update"),
            )
            .arg(
                clap::Arg::with_name("DOMAIN")
                    .required(true)
                    .takes_value(true)
                    .help("The domain that has the record to update"),
            )
            .arg(
                clap::Arg::with_name("token")
                    .short("t")
                    .long("token")
                    .takes_value(true)
                    .env("DIGITAL_OCEAN_TOKEN")
                    .help("The API token to use to auth with DigitalOcean"),
            )
            .arg(
                clap::Arg::with_name("local")
                    .short("l")
                    .long("local")
                    .takes_value(false)
                    .conflicts_with("ip")
                    .help("Use the local IP address connected to the internet"),
            )
            .arg(
                clap::Arg::with_name("ip")
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
                clap::Arg::with_name("rtype")
                    .long("rtype")
                    .takes_value(true)
                    .possible_values(&["A", "AAAA"])
                    .default_value("A")
                    .help("The type of DNS record to set"),
            )
            .arg(
                clap::Arg::with_name("ttl")
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
                clap::Arg::with_name("quiet")
                    .short("q")
                    .long("quiet")
                    .takes_value(false)
                    .help("Only display output on IP change"),
            )
            .arg(
                clap::Arg::with_name("dry_run")
                    .short("n")
                    .long("dry-run")
                    .takes_value(false)
                    .help("Do everything except actually set the record"),
            )
            .get_matches();

        Args {
            record: matches.value_of("RECORD").unwrap().to_string(),
            domain: matches.value_of("DOMAIN").unwrap().to_string(),
            token: matches.value_of("token").unwrap().to_string(),
            local: matches.is_present("local"),
            ip: matches
                .value_of("ip")
                .map(|x| x.parse::<IpAddr>().expect("Unable to parse IP address")),
            rtype: matches.value_of("rtype").unwrap().to_string(),
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

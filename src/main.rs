#![forbid(unsafe_code)]

extern crate clap;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate simplelog;

mod cli;
mod digitalocean;
mod ip_retriever;

use simplelog::{CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use std::net::IpAddr;

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stderr,
    )])
    .unwrap();

    let args = cli::Args::parse_args();
    let client = digitalocean::DigitalOceanClient::new(args.token);

    client
        .get_domain(&args.domain)
        .expect("Error while contacting DigitalOcean")
        .expect("Unable to find domain in account");
    match client
        .get_record(&args.domain, &args.record, &args.rtype)
        .expect("Error while contacting DigitalOcean")
    {
        Some(record) => {
            let record_ip = record
                .data
                .parse::<IpAddr>()
                .expect("Unable to parse {} record for {}.{} as an IP address");
            if record_ip == args.ip {
                info!(
                    "Record {}.{} ({}) already set to {}",
                    args.record, args.domain, args.rtype, args.ip
                );
            } else {
                info!(
                    "Will update record {}.{} ({}) to {}",
                    args.record, args.domain, args.rtype, args.ip
                );
                client
                    .update_record(&args.domain, &record, &args.ip)
                    .expect("Unable to update record");
                info!("Successfully updated record!");
            }
        }
        None => {
            info!(
                "Will create new record {}.{} ({}) -> {}",
                args.record, args.domain, args.rtype, args.ip
            );
            let record = client
                .create_record(&args.domain, &args.record, &args.rtype, &args.ip)
                .expect("Unable to create new record");
            info!("Successfully created new record! ({})", record.id);
        }
    };
}

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
        .get_domain(args.domain.clone())
        .expect("Error while contacting DigitalOcean")
        .expect("Unable to find domain in account");
    let record = client
        .get_record(args.domain, args.record, args.rtype)
        .expect("Error while contacting DigitalOcean")
        .expect("Unable to find record for domain with desired type");
    info!("Will update record: {:?}", record.id);
    print!("{:?}", record)
}

#![forbid(unsafe_code)]

extern crate clap;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde;
#[cfg(not(test))]
extern crate serde_json;
extern crate simplelog;

#[cfg(test)]
extern crate mockito;
#[cfg(test)]
#[macro_use]
extern crate serde_json;

mod cli;
mod digitalocean;
mod ip_retriever;

use simplelog::{CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use std::fmt::Formatter;
use std::net::IpAddr;

use crate::digitalocean::{DigitalOceanClient, DomainRecord};

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stderr,
    )])
    .unwrap();

    let args = cli::Args::parse_args();
    let client = digitalocean::DigitalOceanClientImpl::new(args.token);

    run(
        Box::new(client),
        args.domain,
        args.record,
        args.rtype,
        args.ip,
    )
    .expect("Encountered error while updating DNS record");
}

fn run(
    client: Box<dyn DigitalOceanClient>,
    domain: String,
    record_name: String,
    rtype: String,
    ip: IpAddr,
) -> Result<DomainRecord, Error> {
    client.get_domain(&domain)?.ok_or(Error::DomainNotFound())?;
    match client.get_record(&domain, &record_name, &rtype)? {
        Some(record) => {
            let record_ip = record.data.parse::<IpAddr>()?;
            if record_ip == ip {
                info!(
                    "Record {}.{} ({}) already set to {}",
                    record_name, domain, rtype, ip
                );
                Ok(record)
            } else {
                info!(
                    "Will update record_name {}.{} ({}) to {}",
                    record_name, domain, rtype, ip
                );
                let record = client.update_record(&domain, &record, &ip)?;
                info!("Successfully updated record!");
                Ok(record)
            }
        }
        None => {
            info!(
                "Will create new record {}.{} ({}) -> {}",
                record_name, domain, rtype, ip
            );
            let record = client.create_record(&domain, &record_name, &rtype, &ip)?;
            info!("Successfully created new record! ({})", record.id);
            Ok(record)
        }
    }
}

#[derive(Debug)]
enum Error {
    Client(digitalocean::Error),
    AddrParseError(std::net::AddrParseError),
    DomainNotFound(),
}

impl From<digitalocean::Error> for Error {
    fn from(e: digitalocean::Error) -> Self {
        Error::Client(e)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Error::AddrParseError(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

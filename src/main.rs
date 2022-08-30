#![forbid(unsafe_code)]

extern crate clap;
extern crate reqwest;
extern crate serde;
#[cfg(not(test))]
extern crate serde_json;
extern crate tracing;
extern crate tracing_subscriber;

#[cfg(test)]
extern crate mockito;
#[cfg(test)]
#[macro_use]
extern crate serde_json;

mod cli;
mod digitalocean;
mod ip_retriever;

use std::fmt::Formatter;
use std::net::IpAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::digitalocean::{DigitalOceanClient, DomainRecord};

fn main() {
    let ansi_enabled = fix_ansi_term();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(ansi_enabled)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

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

#[cfg(target_os = "windows")]
fn fix_ansi_term() -> bool {
    nu_ansi_term::enable_ansi_support().map_or(false, |()| true)
}

#[cfg(not(target_os = "windows"))]
fn fix_ansi_term() -> bool {
    true
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
    AddrParseErr(std::net::AddrParseError),
    DomainNotFound(),
}

impl From<digitalocean::Error> for Error {
    fn from(e: digitalocean::Error) -> Self {
        Error::Client(e)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Error::AddrParseErr(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod test {
    use crate::digitalocean::{DigitalOceanClient, Domain, DomainRecord, Error};
    use crate::run;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_create_record() {
        let id = 123;
        let domain = "google.com".to_string();
        let record_name = "main".to_string();
        let rtype = "A".to_string();
        let ip_addr: IpAddr = Ipv4Addr::new(8, 8, 8, 8).into();

        let client = TestClientImpl {
            id: id.clone(),
            domain: domain.clone(),
            record: record_name.clone(),
            rtype: rtype.clone(),
            ip_addr: ip_addr.clone(),
            get_domain_is_ok: true,
            get_domain_is_some: true,
            get_record_is_ok: true,
            get_record_is_some: false,
            update_record_is_ok: false,
            create_record_is_ok: true,
        };

        let record = run(
            Box::new(client),
            domain.clone(),
            record_name.clone(),
            rtype.clone(),
            ip_addr.clone(),
        );

        assert_eq!(
            record.unwrap(),
            DomainRecord {
                id,
                typ: rtype,
                name: record_name,
                data: ip_addr.to_string(),
                priority: None,
                port: None,
                ttl: 60,
                weight: None,
                flags: None,
                tag: None
            }
        )
    }

    #[test]
    fn test_update_record() {
        let id = 123;
        let domain = "google.com".to_string();
        let record_name = "main".to_string();
        let rtype = "A".to_string();
        let ip_addr: IpAddr = Ipv4Addr::new(8, 8, 8, 8).into();
        let new_ip_addr: IpAddr = Ipv4Addr::new(4, 4, 4, 4).into();

        let client = TestClientImpl {
            id: id.clone(),
            domain: domain.clone(),
            record: record_name.clone(),
            rtype: rtype.clone(),
            ip_addr: ip_addr.clone(),
            get_domain_is_ok: true,
            get_domain_is_some: true,
            get_record_is_ok: true,
            get_record_is_some: true,
            update_record_is_ok: true,
            create_record_is_ok: false,
        };

        let record = run(
            Box::new(client),
            domain.clone(),
            record_name.clone(),
            rtype.clone(),
            new_ip_addr.clone(),
        );

        assert_eq!(
            record.unwrap(),
            DomainRecord {
                id,
                typ: rtype,
                name: record_name,
                data: new_ip_addr.to_string(),
                priority: None,
                port: None,
                ttl: 60,
                weight: None,
                flags: None,
                tag: None
            }
        )
    }

    #[test]
    fn test_no_op() {
        let id = 123;
        let domain = "google.com".to_string();
        let record_name = "main".to_string();
        let rtype = "A".to_string();
        let ip_addr: IpAddr = Ipv4Addr::new(8, 8, 8, 8).into();
        let new_ip_addr: IpAddr = Ipv4Addr::new(8, 8, 8, 8).into();

        let client = TestClientImpl {
            id: id.clone(),
            domain: domain.clone(),
            record: record_name.clone(),
            rtype: rtype.clone(),
            ip_addr: ip_addr.clone(),
            get_domain_is_ok: true,
            get_domain_is_some: true,
            get_record_is_ok: true,
            get_record_is_some: true,
            update_record_is_ok: false,
            create_record_is_ok: false,
        };

        let record = run(
            Box::new(client),
            domain.clone(),
            record_name.clone(),
            rtype.clone(),
            new_ip_addr.clone(),
        );

        assert_eq!(
            record.unwrap(),
            DomainRecord {
                id,
                typ: rtype,
                name: record_name,
                data: new_ip_addr.to_string(),
                priority: None,
                port: None,
                ttl: 60,
                weight: None,
                flags: None,
                tag: None
            }
        )
    }

    struct TestClientImpl {
        id: u32,
        domain: String,
        record: String,
        rtype: String,
        ip_addr: IpAddr,
        get_domain_is_ok: bool,
        get_domain_is_some: bool,
        get_record_is_ok: bool,
        get_record_is_some: bool,
        update_record_is_ok: bool,
        create_record_is_ok: bool,
    }

    impl DigitalOceanClient for TestClientImpl {
        fn get_domain(&self, _: &str) -> Result<Option<Domain>, Error> {
            if self.get_domain_is_ok {
                if self.get_domain_is_some {
                    Ok(Some(Domain {
                        name: self.domain.clone(),
                        ttl: 60,
                        zone_file: "foobar".to_string(),
                    }))
                } else {
                    Ok(None)
                }
            } else {
                Err(Error::Create("foo".to_string()))
            }
        }

        fn get_record(&self, _: &str, _: &str, _: &str) -> Result<Option<DomainRecord>, Error> {
            if self.get_record_is_ok {
                if self.get_record_is_some {
                    Ok(Some(DomainRecord {
                        id: self.id.clone(),
                        typ: self.rtype.clone(),
                        name: self.record.clone(),
                        data: self.ip_addr.to_string(),
                        priority: None,
                        port: None,
                        ttl: 60,
                        weight: None,
                        flags: None,
                        tag: None,
                    }))
                } else {
                    Ok(None)
                }
            } else {
                Err(Error::Create("foo".to_string()))
            }
        }

        fn update_record(
            &self,
            _: &str,
            record: &DomainRecord,
            value: &IpAddr,
        ) -> Result<DomainRecord, Error> {
            if self.update_record_is_ok {
                Ok(DomainRecord {
                    id: record.id.clone(),
                    typ: record.typ.clone(),
                    name: record.name.clone(),
                    data: (*value).to_string(),
                    priority: None,
                    port: None,
                    ttl: record.ttl.clone(),
                    weight: None,
                    flags: None,
                    tag: None,
                })
            } else {
                Err(Error::Update("foo".to_string()))
            }
        }

        fn create_record(
            &self,
            _: &str,
            record: &str,
            rtype: &str,
            value: &IpAddr,
        ) -> Result<DomainRecord, Error> {
            if self.create_record_is_ok {
                Ok(DomainRecord {
                    id: 123,
                    typ: rtype.to_string(),
                    name: record.to_string(),
                    data: (*value).to_string(),
                    priority: None,
                    port: None,
                    ttl: 60,
                    weight: None,
                    flags: None,
                    tag: None,
                })
            } else {
                Err(Error::Create("foo".to_string()))
            }
        }
    }
}

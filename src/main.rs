#![forbid(unsafe_code)]

extern crate clap;
#[cfg(test)]
extern crate mockito;
extern crate reqwest;
extern crate serde;
#[cfg(not(test))]
extern crate serde_json;
#[cfg(test)]
#[macro_use]
extern crate serde_json;
extern crate tracing;
extern crate tracing_subscriber;

use std::collections::HashMap;
use std::fmt::Formatter;
use std::net::IpAddr;

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Port, SubcmdArgs};
use crate::digitalocean::dns::{DigitalOceanDnsClient, DomainRecord};
use crate::digitalocean::droplet::{DigitalOceanDropletClient, Droplet};
use crate::digitalocean::firewall::{DigitalOceanFirewallClient, Firewall};

mod cli;
mod digitalocean;
mod ip_retriever;

fn main() {
    let ansi_enabled = fix_ansi_term();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(ansi_enabled)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let args = cli::Args::parse_args();
    let client = digitalocean::DigitalOceanClient::new(args.token);

    match args.subcmd_args {
        SubcmdArgs::Dns(dns_args) => {
            run_dns(
                client.dns,
                dns_args.domain,
                dns_args.record,
                dns_args.rtype,
                args.ip,
                dns_args.ttl,
                args.dry_run,
            )
            .expect("Encountered error while updating DNS record");
        }
        SubcmdArgs::Firewall(fw_args) => {
            run_firewall(
                client.firewall,
                client.droplet,
                fw_args.name,
                fw_args.port,
                fw_args.addresses,
                fw_args.droplets,
                args.ip,
                args.dry_run,
            )
            .expect("Enountered error while updating firewall");
        }
    };
}

#[cfg(target_os = "windows")]
fn fix_ansi_term() -> bool {
    nu_ansi_term::enable_ansi_support().map_or(false, |()| true)
}

#[cfg(not(target_os = "windows"))]
fn fix_ansi_term() -> bool {
    true
}

fn run_dns(
    client: Box<dyn DigitalOceanDnsClient>,
    domain: String,
    record_name: String,
    rtype: String,
    ip: IpAddr,
    ttl: u16,
    dry_run: bool,
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
                let record = client.update_record(&domain, &record, &ip, &ttl, &dry_run)?;
                info!("Successfully updated record!");
                Ok(record)
            }
        }
        None => {
            info!(
                "Will create new record {}.{} ({}) -> {}",
                record_name, domain, rtype, ip
            );
            let record =
                client.create_record(&domain, &record_name, &rtype, &ip, &ttl, &dry_run)?;
            info!("Successfully created new record! ({})", record.id);
            Ok(record)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_firewall(
    fw_client: Box<dyn DigitalOceanFirewallClient>,
    droplet_client: Box<dyn DigitalOceanDropletClient>,
    name: String,
    port: Port,
    addresses: Vec<String>,
    droplet_names: Vec<String>,
    _ip: IpAddr,
    _dry_run: bool,
) -> Result<Firewall, Error> {
    match fw_client.get_firewall(name)? {
        Some(firewall) => {
            println!("firewall: {:?}", firewall);

            match port {
                Port::Inbound(inbound_port) => println!(
                    "inbound port: {:?}",
                    match firewall.inbound_rules {
                        Some(ref rules) => rules.iter().find(|x| x.ports == inbound_port),
                        None => panic!("No inbound_rules available"),
                    }
                ),
                Port::Outbound(outbound_port) => println!(
                    "outbound port: {:?}",
                    match firewall.outbound_rules {
                        Some(ref rules) => rules.iter().find(|x| x.ports == outbound_port),
                        None => panic!("No outbound_rules available"),
                    }
                ),
            };

            println!("allowed addresses: {:?}", addresses);

            println!("allowed droplets (name): {:?}", droplet_names);
            if !droplet_names.is_empty() {
                let droplets_by_name = droplet_client
                    .get_droplets()?
                    .into_iter()
                    .map(|d| (d.name.clone(), d))
                    .collect::<HashMap<String, Droplet>>();
                let droplet_ids = droplet_names
                    .into_iter()
                    .map(|name| match droplets_by_name.get(&name) {
                        Some(droplet) => droplet.id,
                        None => panic!("Unable to find droplet with name {}", name),
                    })
                    .collect::<Vec<u32>>();
                println!("allowed droplets (id): {:?}", droplet_ids);
            }

            Ok(firewall)
        }
        None => Err(Error::FirewallNotFound()),
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum Error {
    Client(digitalocean::error::Error),
    AddrParseErr(std::net::AddrParseError),
    DomainNotFound(),
    FirewallNotFound(),
}

impl From<digitalocean::error::Error> for Error {
    fn from(e: digitalocean::error::Error) -> Self {
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
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr};

    use crate::digitalocean::dns::{DigitalOceanDnsClient, Domain, DomainRecord};
    use crate::digitalocean::error::Error;
    use crate::run_dns;

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

        let record = run_dns(
            Box::new(client),
            domain.clone(),
            record_name.clone(),
            rtype.clone(),
            ip_addr.clone(),
            60,
            false,
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

        let record = run_dns(
            Box::new(client),
            domain.clone(),
            record_name.clone(),
            rtype.clone(),
            new_ip_addr.clone(),
            60,
            false,
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

        let record = run_dns(
            Box::new(client),
            domain.clone(),
            record_name.clone(),
            rtype.clone(),
            new_ip_addr.clone(),
            60,
            false,
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

    impl DigitalOceanDnsClient for TestClientImpl {
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
            ttl: &u16,
            _dry_run: &bool,
        ) -> Result<DomainRecord, Error> {
            if self.update_record_is_ok {
                Ok(DomainRecord {
                    id: record.id.clone(),
                    typ: record.typ.clone(),
                    name: record.name.clone(),
                    data: (*value).to_string(),
                    priority: None,
                    port: None,
                    ttl: *ttl,
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
            ttl: &u16,
            _dry_run: &bool,
        ) -> Result<DomainRecord, Error> {
            if self.create_record_is_ok {
                Ok(DomainRecord {
                    id: 123,
                    typ: rtype.to_string(),
                    name: record.to_string(),
                    data: (*value).to_string(),
                    priority: None,
                    port: None,
                    ttl: *ttl,
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

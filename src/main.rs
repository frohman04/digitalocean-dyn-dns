#![forbid(unsafe_code)]

#[cfg(test)]
extern crate approx;
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
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::net::IpAddr;
use std::rc::Rc;

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Direction, SubcmdArgs};
use crate::digitalocean::dns::{DigitalOceanDnsClient, DomainRecord};
use crate::digitalocean::droplet::DigitalOceanDropletClient;
use crate::digitalocean::firewall::{
    DigitalOceanFirewallClient, Firewall, FirewallInboundRule, FirewallOutboundRule,
    FirewallRuleTarget,
};
use crate::digitalocean::kubernetes::DigitalOceanKubernetesClient;
use crate::digitalocean::loadbalancer::DigitalOceanLoadbalancerClient;

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
            let (firewall, inbound_rule, outbound_rule) = build_firewall_args(
                client.firewall.clone(),
                client.droplet,
                client.kubernetes,
                client.load_balancer,
                fw_args.name,
                fw_args.direction,
                fw_args.port,
                fw_args.protocol,
                fw_args.addresses,
                fw_args.droplets,
                fw_args.kubernetes_clusters,
                fw_args.load_balancers,
                args.ip,
            )
            .expect("Encountered error while constructing firewall rules");
            update_firewall(
                client.firewall,
                firewall,
                inbound_rule,
                outbound_rule,
                args.dry_run,
            )
            .expect("Encountered error while updating firewall");
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
    client: Rc<dyn DigitalOceanDnsClient>,
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_firewall_args(
    fw_client: Rc<dyn DigitalOceanFirewallClient>,
    droplet_client: Rc<dyn DigitalOceanDropletClient>,
    kubernetes_client: Rc<dyn DigitalOceanKubernetesClient>,
    load_balancer_client: Rc<dyn DigitalOceanLoadbalancerClient>,
    name: String,
    direction: Direction,
    port: String,
    protocol: String,
    addresses: Option<Vec<String>>,
    droplet_names: Option<Vec<String>>,
    kubernetes_cluster_names: Option<Vec<String>>,
    load_balancer_names: Option<Vec<String>>,
    ip: IpAddr,
) -> Result<
    (
        Firewall,
        Option<(FirewallInboundRule, FirewallInboundRule)>,
        Option<(FirewallOutboundRule, FirewallOutboundRule)>,
    ),
    Error,
> {
    match fw_client.get_firewall(name)? {
        Some(firewall) => {
            let all_addresses = Some({
                let mut all_addresses = match addresses {
                    Some(x) => x.clone(),
                    None => Vec::new(),
                };
                all_addresses.push(ip.to_string());
                all_addresses
            });

            let droplet_ids = names_to_ids(
                || droplet_client.get_droplets(),
                droplet_names,
                |d| d.name.clone(),
                |d| d.id,
            )?;

            let kubernetes_cluster_ids = names_to_ids(
                || kubernetes_client.get_kubernetes_clusters(),
                kubernetes_cluster_names,
                |d| d.name.clone(),
                |d| d.id.clone(),
            )?;

            let load_balancer_ids = names_to_ids(
                || load_balancer_client.get_load_balancers(),
                load_balancer_names,
                |d| d.name.clone(),
                |d| d.id.clone(),
            )?;

            match direction {
                Direction::Inbound => {
                    let inbound_rule = match firewall.inbound_rules {
                        Some(ref rules) => rules
                            .iter()
                            .find(|x| x.ports == port && x.protocol == protocol)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Unable to find firewall rule for port {} and protocol {}",
                                    port, protocol
                                )
                            })
                            .clone(),
                        None => panic!("No inbound_rules available"),
                    };
                    let new_inbound_rule = FirewallInboundRule {
                        protocol: inbound_rule.protocol.clone(),
                        ports: inbound_rule.ports.clone(),
                        sources: FirewallRuleTarget {
                            addresses: all_addresses,
                            droplet_ids,
                            kubernetes_ids: kubernetes_cluster_ids,
                            load_balancer_uids: load_balancer_ids,
                            tags: inbound_rule.sources.tags.clone(),
                        },
                    };

                    Ok((firewall, Some((inbound_rule, new_inbound_rule)), None))
                }
                Direction::Outbound => {
                    let outbound_rule = match firewall.outbound_rules {
                        Some(ref rules) => rules
                            .iter()
                            .find(|x| x.ports == port && x.protocol == protocol)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Unable to find firewall rule for port {} and protocol {}",
                                    port, protocol
                                )
                            })
                            .clone(),
                        None => panic!("No outbound_rules available"),
                    };

                    let new_outbound_rule = FirewallOutboundRule {
                        protocol: outbound_rule.protocol.clone(),
                        ports: outbound_rule.ports.clone(),
                        destinations: FirewallRuleTarget {
                            addresses: all_addresses,
                            droplet_ids,
                            kubernetes_ids: kubernetes_cluster_ids,
                            load_balancer_uids: load_balancer_ids,
                            tags: outbound_rule.destinations.tags.clone(),
                        },
                    };

                    Ok((
                        firewall,
                        None,
                        Some((outbound_rule.clone(), new_outbound_rule)),
                    ))
                }
            }
        }
        None => Err(Error::FirewallNotFound()),
    }
}

fn update_firewall(
    fw_client: Rc<dyn DigitalOceanFirewallClient>,
    firewall: Firewall,
    inbound_rule_replacement: Option<(FirewallInboundRule, FirewallInboundRule)>,
    outbound_rule_replacement: Option<(FirewallOutboundRule, FirewallOutboundRule)>,
    dry_run: bool,
) -> Result<Firewall, Error> {
    let (inbound_rule, new_inbound_rule) = match inbound_rule_replacement {
        Some((ir, nir)) => (Some(vec![ir.clone()]), Some(vec![nir])),
        None => (None, None),
    };
    let (outbound_rule, new_outbound_rule) = match outbound_rule_replacement {
        Some((or, nor)) => (Some(vec![or.clone()]), Some(vec![nor])),
        None => (None, None),
    };

    if inbound_rule.is_some() {
        info!(
            "Deleting inbound rule on firewall {}\n{:#?}",
            firewall.id, inbound_rule
        );
    }
    if outbound_rule.is_some() {
        info!(
            "Deleting outbound rule on firewall {}\n{:#?}",
            firewall.id, outbound_rule
        );
    }
    fw_client.delete_firewall_rule(firewall.id.as_str(), inbound_rule, outbound_rule, &dry_run)?;

    if new_inbound_rule.is_some() {
        info!(
            "Creating inbound rule on firewall {}\n{:#?}",
            firewall.id, new_inbound_rule
        );
    }
    if new_outbound_rule.is_some() {
        info!(
            "Creating outbound rule on firewall {}\n{:#?}",
            firewall.id, new_outbound_rule
        );
    }
    fw_client.add_firewall_rule(
        firewall.id.as_str(),
        new_inbound_rule,
        new_outbound_rule,
        &dry_run,
    )?;

    info!("Fetching updated firewall");
    let updated_firewall = fw_client
        .get_firewall(firewall.name.clone())
        .map(|f| f.expect("Unable to find firewall after modifying!"))?;

    Ok(updated_firewall)
}

fn names_to_ids<K, N, T, OF, KF, NF>(
    get_objects: OF,
    names: Option<Vec<N>>,
    extract_name: NF,
    extract_key: KF,
) -> Result<Option<Vec<K>>, digitalocean::error::Error>
where
    N: Eq + Hash + Display,
    OF: Fn() -> Result<Vec<T>, digitalocean::error::Error>,
    KF: Fn(&T) -> K,
    NF: Fn(&T) -> N,
{
    names
        .map(|ns| {
            get_objects().map(|objects| {
                let by_name = objects
                    .into_iter()
                    .map(|d| (extract_name(&d), d))
                    .collect::<HashMap<N, T>>();
                ns.into_iter()
                    .map(|name| match by_name.get(&name) {
                        Some(d) => extract_key(d),
                        None => panic!("Unable to find object with name {}", name),
                    })
                    .collect::<Vec<K>>()
            })
        })
        .map_or(Ok(None), |r| r.map(Some))
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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr};
    use std::rc::Rc;

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
            Rc::new(client),
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
            Rc::new(client),
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
            Rc::new(client),
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
                Err(Error::CreateDns("foo".to_string()))
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
                Err(Error::CreateDns("foo".to_string()))
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
                Err(Error::UpdateDns("foo".to_string()))
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
                Err(Error::CreateDns("foo".to_string()))
            }
        }
    }
}

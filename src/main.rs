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

use tracing::{Level, info};
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
    nu_ansi_term::enable_ansi_support().is_ok_and(|()| true)
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
                let ip_str = ip.to_string();
                if !all_addresses.contains(&ip_str) {
                    all_addresses.push(ip.to_string());
                }
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
                                    "Unable to find firewall rule for port {port} and protocol {protocol}"
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
                                    "Unable to find firewall rule for port {port} and protocol {protocol}"
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
                        None => panic!("Unable to find object with name {name}"),
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
mod dns_test {
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

        let client = TestDnsClientImpl {
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

        let client = TestDnsClientImpl {
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

        let client = TestDnsClientImpl {
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

    struct TestDnsClientImpl {
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

    impl DigitalOceanDnsClient for TestDnsClientImpl {
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

#[cfg(test)]
mod fw_test {
    use crate::Error::Client;
    use crate::cli::Direction;
    use crate::digitalocean::droplet::{
        DigitalOceanDropletClient, Droplet, DropletImage, DropletNetworks, DropletRegion,
        DropletSize,
    };
    use crate::digitalocean::error::Error;
    use crate::digitalocean::firewall::{
        DigitalOceanFirewallClient, Firewall, FirewallInboundRule, FirewallOutboundRule,
        FirewallRuleTarget,
    };
    use crate::digitalocean::kubernetes::{
        DigitalOceanKubernetesClient, KubernetesCluster, KubernetesClusterStatus,
    };
    use crate::digitalocean::loadbalancer::{
        DigitalOceanLoadbalancerClient, Loadbalancer, LoadbalancerFirewall,
        LoadbalancerHealthCheck, LoadbalancerRegion, LoadbalancerStickySessions,
    };
    use crate::{build_firewall_args, update_firewall};
    use std::net::{IpAddr, Ipv4Addr};
    use std::rc::Rc;

    #[test]
    fn test_translate_args_basic_in() {
        base_translate_inbound_test(None, None, None, None)
    }

    #[test]
    fn test_translate_args_basic_out() {
        let fw_id = "foo".to_string();
        let fw_name = "Foo".to_string();
        let fw_addrs = Some(vec!["1.1.1.1".to_string()]);
        let fw_tags = Some(vec!["bar".to_string()]);
        let host_addr = Ipv4Addr::new(8, 8, 8, 8);
        let expected_addrs = vec![host_addr.to_string()];
        let curr_inbound_rule = None;
        let curr_outbound_rule = FirewallOutboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            destinations: FirewallRuleTarget {
                addresses: fw_addrs.clone(),
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: fw_tags.clone(),
            },
        };
        let firewall = Firewall {
            id: fw_id.clone(),
            status: "succeeded".to_string(),
            created_at: "2024-01-01T00:00Z".to_string(),
            pending_changes: vec![],
            name: fw_name.clone(),
            droplet_ids: None,
            tags: None,
            inbound_rules: curr_inbound_rule,
            outbound_rules: Some(vec![curr_outbound_rule.clone()]),
        };

        let fw_client = TestFwClientImpl {
            expected_get_firewall_name: Some(fw_name.clone()),
            firewall: Some(firewall.clone()),
            expected_delete_firewall_id: None,
            expected_delete_inbound_rules: None,
            expected_delete_outbound_rules: None,
            delete_rule_is_ok: false,
            expected_add_firewall_id: None,
            expected_add_inbound_rules: None,
            expected_add_outbound_rules: None,
            add_rule_is_ok: false,
        };
        let droplet_client = TestDropletClientImpl { droplets: vec![] };
        let kubernetes_client = TestKubeClientImpl { clusters: vec![] };
        let load_balancer_client = TestLbClientImpl {
            loadbalancers: vec![],
        };

        match build_firewall_args(
            Rc::new(fw_client),
            Rc::new(droplet_client),
            Rc::new(kubernetes_client),
            Rc::new(load_balancer_client),
            fw_name,
            Direction::Outbound,
            "80".to_string(),
            "http".to_string(),
            None,
            None,
            None,
            None,
            IpAddr::V4(host_addr.clone()),
        )
        .expect("Unexpected failure in build_firewall_args")
        {
            (actual_fw, None, Some((actual_curr_outbound_rule, actual_new_outbound_rule))) => {
                assert_eq!(firewall, actual_fw);
                assert_eq!(curr_outbound_rule, actual_curr_outbound_rule);
                assert_eq!(
                    FirewallOutboundRule {
                        protocol: curr_outbound_rule.protocol,
                        ports: curr_outbound_rule.ports,
                        destinations: FirewallRuleTarget {
                            addresses: Some(expected_addrs),
                            droplet_ids: None,
                            load_balancer_uids: None,
                            kubernetes_ids: None,
                            tags: curr_outbound_rule.destinations.tags,
                        },
                    },
                    actual_new_outbound_rule
                );
            }
            x => panic!(
                "Failed to get correct return values from build_firewall_args (got {:?}",
                x
            ),
        };
    }

    #[test]
    fn test_translate_args_addresses() {
        base_translate_inbound_test(Some(vec!["1.1.1.1".to_string()]), None, None, None)
    }

    #[test]
    #[allow(deprecated)]
    fn test_translate_args_droplet() {
        let droplet_id: u32 = 42;
        let droplet_name = "snake".to_string();
        base_translate_inbound_test(
            None,
            Some((
                vec![Droplet {
                    id: droplet_id.clone(),
                    name: droplet_name.clone(),
                    memory: 0,
                    vcpus: 0,
                    disk: 0,
                    locked: false,
                    status: "".to_string(),
                    kernel: None,
                    created_at: "".to_string(),
                    features: vec![],
                    backup_ids: vec![],
                    next_backup_window: None,
                    snapshot_ids: vec![],
                    image: DropletImage {
                        id: 0,
                        name: "".to_string(),
                        typ: "".to_string(),
                        distribution: "".to_string(),
                        slug: None,
                        public: false,
                        regions: vec![],
                        created_at: "".to_string(),
                        min_disk_size: None,
                        size_gigabytes: None,
                        description: None,
                        tags: vec![],
                        status: "".to_string(),
                        error_message: None,
                    },
                    volume_ids: vec![],
                    size: DropletSize {
                        slug: "".to_string(),
                        memory: 0,
                        vcpus: 0,
                        disk: 0,
                        transfer: 0.0,
                        price_monthly: 0.0,
                        price_hourly: 0.0,
                        regions: vec![],
                        available: false,
                        description: "".to_string(),
                    },
                    size_slug: "".to_string(),
                    networks: DropletNetworks {
                        v4: vec![],
                        v6: vec![],
                    },
                    region: DropletRegion {
                        name: "".to_string(),
                        slug: "".to_string(),
                        features: vec![],
                        available: false,
                        sizes: vec![],
                    },
                    tags: vec![],
                    vpc_uuid: "".to_string(),
                }],
                vec![droplet_name],
                vec![droplet_id],
            )),
            None,
            None,
        )
    }

    #[test]
    fn test_translate_args_kube() {
        let kube_name = "foo".to_string();
        let kube_id = "123-456-789".to_string();
        base_translate_inbound_test(
            None,
            None,
            Some((
                vec![KubernetesCluster {
                    id: kube_id.clone(),
                    name: kube_name.clone(),
                    region: "".to_string(),
                    version: "".to_string(),
                    cluster_subnet: "".to_string(),
                    service_subnet: "".to_string(),
                    vpc_uuid: "".to_string(),
                    ipv4: None,
                    endpoint: "".to_string(),
                    tags: vec![],
                    node_pools: vec![],
                    maintenance_policy: None,
                    auto_upgrade: false,
                    status: KubernetesClusterStatus {
                        state: "".to_string(),
                        message: None,
                    },
                    created_at: "".to_string(),
                    updated_at: "".to_string(),
                    surge_upgrade: false,
                    ha: false,
                    registry_enabled: false,
                }],
                vec![kube_name],
                vec![kube_id],
            )),
            None,
        )
    }

    #[test]
    #[allow(deprecated)]
    fn test_translate_args_lb() {
        let lb_name = "foo".to_string();
        let lb_id = "123-456-789".to_string();
        base_translate_inbound_test(
            None,
            None,
            None,
            Some((
                vec![Loadbalancer {
                    id: lb_id.clone(),
                    name: lb_name.clone(),
                    project_id: "".to_string(),
                    ip: "".to_string(),
                    size_unit: 0,
                    size: None,
                    algorithm: None,
                    status: "".to_string(),
                    created_at: "".to_string(),
                    forwarding_rules: vec![],
                    health_check: LoadbalancerHealthCheck {
                        protocol: "".to_string(),
                        port: 0,
                        path: "".to_string(),
                        check_interval_seconds: 0,
                        response_timeout_seconds: 0,
                        unhealthy_threshold: 0,
                        healthy_threshold: 0,
                    },
                    sticky_sessions: LoadbalancerStickySessions {
                        typ: "".to_string(),
                        cookie_name: None,
                        cookie_ttl_seconds: None,
                    },
                    redirect_http_to_https: false,
                    enable_proxy_protocol: false,
                    enable_backend_keepalive: false,
                    http_idle_timeout_seconds: 0,
                    vpc_uuid: "".to_string(),
                    disable_lets_encrypt_dns_records: false,
                    firewall: LoadbalancerFirewall {
                        deny: vec![],
                        allow: vec![],
                    },
                    region: LoadbalancerRegion {
                        name: "".to_string(),
                        slug: "".to_string(),
                        features: vec![],
                        available: false,
                        sizes: vec![],
                    },
                    droplet_ids: vec![],
                    tag: "".to_string(),
                }],
                vec![lb_name],
                vec![lb_id],
            )),
        )
    }

    fn base_translate_inbound_test(
        extra_addrs: Option<Vec<String>>,
        droplet_data: Option<(Vec<Droplet>, Vec<String>, Vec<u32>)>,
        kube_data: Option<(Vec<KubernetesCluster>, Vec<String>, Vec<String>)>,
        lb_data: Option<(Vec<Loadbalancer>, Vec<String>, Vec<String>)>,
    ) {
        let (droplets, droplet_names, droplet_ids) = match droplet_data {
            Some((d, n, i)) => (Some(d), Some(n), Some(i)),
            None => (None, None, None),
        };
        let (kube_clusters, kube_cluster_names, kube_cluster_ids) = match kube_data {
            Some((k, n, i)) => (Some(k), Some(n), Some(i)),
            None => (None, None, None),
        };
        let (lbs, lb_names, lb_ids) = match lb_data {
            Some((l, n, i)) => (Some(l), Some(n), Some(i)),
            None => (None, None, None),
        };

        let fw_id = "foo".to_string();
        let fw_name = "Foo".to_string();
        let fw_addrs = Some(vec!["1.1.1.1".to_string()]);
        let fw_tags = Some(vec!["bar".to_string()]);
        let host_addr = Ipv4Addr::new(8, 8, 8, 8);
        let expected_addrs = {
            let mut expected_addrs: Vec<String> = Vec::new();
            match extra_addrs.clone() {
                Some(addrs) => addrs.iter().for_each(|a| expected_addrs.push(a.clone())),
                None => (),
            };
            expected_addrs.push(host_addr.to_string());
            expected_addrs
        };
        let curr_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: fw_addrs.clone(),
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: fw_tags.clone(),
            },
        };
        let curr_outbound_rule = None;
        let firewall = Firewall {
            id: fw_id.clone(),
            status: "succeeded".to_string(),
            created_at: "2024-01-01T00:00Z".to_string(),
            pending_changes: vec![],
            name: fw_name.clone(),
            droplet_ids: None,
            tags: None,
            inbound_rules: Some(vec![curr_inbound_rule.clone()]),
            outbound_rules: curr_outbound_rule,
        };

        let fw_client = TestFwClientImpl {
            expected_get_firewall_name: Some(fw_name.clone()),
            firewall: Some(firewall.clone()),
            expected_delete_firewall_id: None,
            expected_delete_inbound_rules: None,
            expected_delete_outbound_rules: None,
            delete_rule_is_ok: false,
            expected_add_firewall_id: None,
            expected_add_inbound_rules: None,
            expected_add_outbound_rules: None,
            add_rule_is_ok: false,
        };
        let droplet_client = TestDropletClientImpl {
            droplets: droplets.unwrap_or_else(|| vec![]),
        };
        let kubernetes_client = TestKubeClientImpl {
            clusters: kube_clusters.unwrap_or_else(|| vec![]),
        };
        let load_balancer_client = TestLbClientImpl {
            loadbalancers: lbs.unwrap_or_else(|| vec![]),
        };

        match build_firewall_args(
            Rc::new(fw_client),
            Rc::new(droplet_client),
            Rc::new(kubernetes_client),
            Rc::new(load_balancer_client),
            fw_name,
            Direction::Inbound,
            "80".to_string(),
            "http".to_string(),
            extra_addrs,
            droplet_names,
            kube_cluster_names,
            lb_names,
            IpAddr::V4(host_addr.clone()),
        )
        .expect("Unexpected failure in build_firewall_args")
        {
            (actual_fw, Some((actual_curr_inbound_rule, actual_new_inbound_rule)), None) => {
                assert_eq!(firewall, actual_fw);
                assert_eq!(curr_inbound_rule, actual_curr_inbound_rule);
                assert_eq!(
                    FirewallInboundRule {
                        protocol: curr_inbound_rule.protocol,
                        ports: curr_inbound_rule.ports,
                        sources: FirewallRuleTarget {
                            addresses: Some(expected_addrs),
                            droplet_ids,
                            load_balancer_uids: lb_ids,
                            kubernetes_ids: kube_cluster_ids,
                            tags: curr_inbound_rule.sources.tags,
                        },
                    },
                    actual_new_inbound_rule
                );
            }
            x => panic!(
                "Failed to get correct return values from build_firewall_args (got {:?}",
                x
            ),
        };
    }

    #[test]
    fn test_translate_args_no_dupe_addresses() {
        let fw_id = "foo".to_string();
        let fw_name = "Foo".to_string();
        let fw_addrs = Some(vec!["8.8.8.8".to_string()]);
        let fw_tags = Some(vec!["bar".to_string()]);
        let host_addr = Ipv4Addr::new(8, 8, 8, 8);
        let expected_addrs = vec![host_addr.to_string()];
        let curr_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: fw_addrs.clone(),
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: fw_tags.clone(),
            },
        };
        let curr_outbound_rule = None;
        let firewall = Firewall {
            id: fw_id.clone(),
            status: "succeeded".to_string(),
            created_at: "2024-01-01T00:00Z".to_string(),
            pending_changes: vec![],
            name: fw_name.clone(),
            droplet_ids: None,
            tags: None,
            inbound_rules: Some(vec![curr_inbound_rule.clone()]),
            outbound_rules: curr_outbound_rule,
        };

        let fw_client = TestFwClientImpl {
            expected_get_firewall_name: Some(fw_name.clone()),
            firewall: Some(firewall.clone()),
            expected_delete_firewall_id: None,
            expected_delete_inbound_rules: None,
            expected_delete_outbound_rules: None,
            delete_rule_is_ok: false,
            expected_add_firewall_id: None,
            expected_add_inbound_rules: None,
            expected_add_outbound_rules: None,
            add_rule_is_ok: false,
        };
        let droplet_client = TestDropletClientImpl { droplets: vec![] };
        let kubernetes_client = TestKubeClientImpl { clusters: vec![] };
        let load_balancer_client = TestLbClientImpl {
            loadbalancers: vec![],
        };

        match build_firewall_args(
            Rc::new(fw_client),
            Rc::new(droplet_client),
            Rc::new(kubernetes_client),
            Rc::new(load_balancer_client),
            fw_name,
            Direction::Inbound,
            "80".to_string(),
            "http".to_string(),
            Some(vec!["8.8.8.8".to_string()]),
            None,
            None,
            None,
            IpAddr::V4(host_addr.clone()),
        )
        .expect("Unexpected failure in build_firewall_args")
        {
            (actual_fw, Some((actual_curr_inbound_rule, actual_new_inbound_rule)), None) => {
                assert_eq!(firewall, actual_fw);
                assert_eq!(curr_inbound_rule, actual_curr_inbound_rule);
                assert_eq!(
                    FirewallInboundRule {
                        protocol: curr_inbound_rule.protocol,
                        ports: curr_inbound_rule.ports,
                        sources: FirewallRuleTarget {
                            addresses: Some(expected_addrs),
                            droplet_ids: None,
                            load_balancer_uids: None,
                            kubernetes_ids: None,
                            tags: curr_inbound_rule.sources.tags,
                        },
                    },
                    actual_new_inbound_rule
                );
            }
            x => panic!(
                "Failed to get correct return values from build_firewall_args (got {:?}",
                x
            ),
        };
    }

    #[test]
    fn test_update_firewall() {
        let fw_id = "foo".to_string();
        let fw_name = "Foo".to_string();
        let cur_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: None,
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: None,
            },
        };
        let new_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: Some(vec!["1.1.1.1".to_string()]),
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: None,
            },
        };
        let firewall = Firewall {
            id: fw_id.clone(),
            status: "".to_string(),
            created_at: "".to_string(),
            pending_changes: vec![],
            name: fw_name.clone(),
            droplet_ids: None,
            tags: None,
            inbound_rules: Some(vec![cur_inbound_rule.clone()]),
            outbound_rules: None,
        };
        let fw_client = TestFwClientImpl {
            expected_get_firewall_name: Some(fw_name.clone()),
            firewall: Some(firewall.clone()),
            expected_delete_firewall_id: Some(fw_id.clone()),
            expected_delete_inbound_rules: Some(vec![cur_inbound_rule.clone()]),
            expected_delete_outbound_rules: None,
            delete_rule_is_ok: true,
            expected_add_firewall_id: Some(fw_id.clone()),
            expected_add_inbound_rules: Some(vec![new_inbound_rule.clone()]),
            expected_add_outbound_rules: None,
            add_rule_is_ok: true,
        };

        match update_firewall(
            Rc::new(fw_client),
            firewall.clone(),
            Some((cur_inbound_rule, new_inbound_rule)),
            None,
            false,
        ) {
            Ok(new_fw) => assert_eq!(new_fw, firewall),
            Err(e) => panic!("Unexpected error while updating firewall: {:?}", e),
        };
    }

    #[test]
    fn test_update_firewall_delete_fail() {
        let fw_id = "foo".to_string();
        let fw_name = "Foo".to_string();
        let cur_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: None,
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: None,
            },
        };
        let new_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: Some(vec!["1.1.1.1".to_string()]),
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: None,
            },
        };
        let firewall = Firewall {
            id: fw_id.clone(),
            status: "".to_string(),
            created_at: "".to_string(),
            pending_changes: vec![],
            name: fw_name.clone(),
            droplet_ids: None,
            tags: None,
            inbound_rules: Some(vec![cur_inbound_rule.clone()]),
            outbound_rules: None,
        };
        let fw_client = TestFwClientImpl {
            expected_get_firewall_name: Some(fw_name.clone()),
            firewall: Some(firewall.clone()),
            expected_delete_firewall_id: Some(fw_id.clone()),
            expected_delete_inbound_rules: Some(vec![cur_inbound_rule.clone()]),
            expected_delete_outbound_rules: None,
            delete_rule_is_ok: false,
            expected_add_firewall_id: Some(fw_id.clone()),
            expected_add_inbound_rules: Some(vec![new_inbound_rule.clone()]),
            expected_add_outbound_rules: None,
            add_rule_is_ok: true,
        };

        match update_firewall(
            Rc::new(fw_client),
            firewall.clone(),
            Some((cur_inbound_rule, new_inbound_rule)),
            None,
            false,
        ) {
            Ok(_) => panic!("Expected delete call to fail!"),
            Err(Client(Error::DeleteFirewallRule(_))) => (),
            Err(e) => panic!("Unexpected failure reason: {:?}", e),
        };
    }

    #[test]
    fn test_update_firewall_add_fail() {
        let fw_id = "foo".to_string();
        let fw_name = "Foo".to_string();
        let cur_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: None,
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: None,
            },
        };
        let new_inbound_rule = FirewallInboundRule {
            protocol: "http".to_string(),
            ports: "80".to_string(),
            sources: FirewallRuleTarget {
                addresses: Some(vec!["1.1.1.1".to_string()]),
                droplet_ids: None,
                load_balancer_uids: None,
                kubernetes_ids: None,
                tags: None,
            },
        };
        let firewall = Firewall {
            id: fw_id.clone(),
            status: "".to_string(),
            created_at: "".to_string(),
            pending_changes: vec![],
            name: fw_name.clone(),
            droplet_ids: None,
            tags: None,
            inbound_rules: Some(vec![cur_inbound_rule.clone()]),
            outbound_rules: None,
        };
        let fw_client = TestFwClientImpl {
            expected_get_firewall_name: Some(fw_name.clone()),
            firewall: Some(firewall.clone()),
            expected_delete_firewall_id: Some(fw_id.clone()),
            expected_delete_inbound_rules: Some(vec![cur_inbound_rule.clone()]),
            expected_delete_outbound_rules: None,
            delete_rule_is_ok: true,
            expected_add_firewall_id: Some(fw_id.clone()),
            expected_add_inbound_rules: Some(vec![new_inbound_rule.clone()]),
            expected_add_outbound_rules: None,
            add_rule_is_ok: false,
        };

        match update_firewall(
            Rc::new(fw_client),
            firewall.clone(),
            Some((cur_inbound_rule, new_inbound_rule)),
            None,
            false,
        ) {
            Ok(_) => panic!("Expected create/add call to fail!"),
            Err(Client(Error::CreateFirewallRule(_))) => (),
            Err(e) => panic!("Unexpected failure reason: {:?}", e),
        };
    }

    struct TestFwClientImpl {
        expected_get_firewall_name: Option<String>,
        firewall: Option<Firewall>,
        expected_delete_firewall_id: Option<String>,
        expected_delete_inbound_rules: Option<Vec<FirewallInboundRule>>,
        expected_delete_outbound_rules: Option<Vec<FirewallOutboundRule>>,
        delete_rule_is_ok: bool,
        expected_add_firewall_id: Option<String>,
        expected_add_inbound_rules: Option<Vec<FirewallInboundRule>>,
        expected_add_outbound_rules: Option<Vec<FirewallOutboundRule>>,
        add_rule_is_ok: bool,
    }

    impl DigitalOceanFirewallClient for TestFwClientImpl {
        fn get_firewall(&self, name: String) -> Result<Option<Firewall>, Error> {
            match self.expected_get_firewall_name.clone() {
                Some(expected_name) => assert_eq!(name, expected_name),
                None => panic!("Must define expected_get_firewall_name"),
            };

            Ok(self.firewall.clone())
        }

        fn delete_firewall_rule(
            &self,
            id: &str,
            inbound_rules: Option<Vec<FirewallInboundRule>>,
            outbound_rules: Option<Vec<FirewallOutboundRule>>,
            _dry_run: &bool,
        ) -> Result<(), Error> {
            match self.expected_delete_firewall_id.clone() {
                Some(expected_id) => assert_eq!(id, expected_id),
                None => panic!("Must define expected_firewall_delete_id"),
            };
            assert_eq!(inbound_rules, self.expected_delete_inbound_rules);
            assert_eq!(outbound_rules, self.expected_delete_outbound_rules);

            if self.delete_rule_is_ok {
                Ok(())
            } else {
                Err(Error::DeleteFirewallRule("test".to_string()))
            }
        }

        fn add_firewall_rule(
            &self,
            id: &str,
            inbound_rules: Option<Vec<FirewallInboundRule>>,
            outbound_rules: Option<Vec<FirewallOutboundRule>>,
            _dry_run: &bool,
        ) -> Result<(), Error> {
            match self.expected_add_firewall_id.clone() {
                Some(expected_id) => assert_eq!(id, expected_id),
                None => panic!("Must define expected_add_firewall_id"),
            };
            assert_eq!(inbound_rules, self.expected_add_inbound_rules);
            assert_eq!(outbound_rules, self.expected_add_outbound_rules);

            if self.add_rule_is_ok {
                Ok(())
            } else {
                Err(Error::CreateFirewallRule("test".to_string()))
            }
        }
    }

    struct TestDropletClientImpl {
        droplets: Vec<Droplet>,
    }

    impl DigitalOceanDropletClient for TestDropletClientImpl {
        fn get_droplets(&self) -> Result<Vec<Droplet>, Error> {
            Ok(self.droplets.clone())
        }
    }

    struct TestKubeClientImpl {
        clusters: Vec<KubernetesCluster>,
    }

    impl DigitalOceanKubernetesClient for TestKubeClientImpl {
        fn get_kubernetes_clusters(&self) -> Result<Vec<KubernetesCluster>, Error> {
            Ok(self.clusters.clone())
        }
    }

    struct TestLbClientImpl {
        loadbalancers: Vec<Loadbalancer>,
    }

    impl DigitalOceanLoadbalancerClient for TestLbClientImpl {
        fn get_load_balancers(&self) -> Result<Vec<Loadbalancer>, Error> {
            Ok(self.loadbalancers.clone())
        }
    }
}

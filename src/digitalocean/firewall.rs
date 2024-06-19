use crate::digitalocean::api::{DigitalOceanApiClient, Links, Meta};
use crate::digitalocean::error::Error;
use reqwest::Method;
use serde::Deserialize;

pub trait DigitalOceanFirewallClient {
    fn get_firewall(&self, name: String) -> Result<Option<Firewall>, Error>;
}

pub struct DigitalOceanFirewallClientImpl {
    api: DigitalOceanApiClient,
}

impl DigitalOceanFirewallClientImpl {
    pub fn new(api: DigitalOceanApiClient) -> DigitalOceanFirewallClientImpl {
        DigitalOceanFirewallClientImpl { api }
    }
}

impl DigitalOceanFirewallClient for DigitalOceanFirewallClientImpl {
    /// Get the named firewall's current configuration.
    fn get_firewall(&self, name: String) -> Result<Option<Firewall>, Error> {
        let mut url = self.api.get_url("/v2/firewalls");
        let mut exit = false;
        let mut obj: Option<Firewall> = None;

        while !exit {
            let resp = self
                .api
                .get_request_builder(Method::GET, url.clone())
                .send()?
                .json::<crate::digitalocean::firewall::FirewallsResp>()?;

            obj = resp.firewalls.into_iter().find(|f| f.name == *name);
            if obj.is_some() {
                exit = true;
            } else if resp.links.pages.is_some() && resp.links.pages.clone().unwrap().next.is_some()
            {
                url = resp.links.pages.unwrap().next.unwrap();
            } else {
                exit = true;
            }
        }

        Ok(obj)
    }
}

// /v2/firewalls

#[derive(Deserialize, Debug)]
struct FirewallsResp {
    firewalls: Vec<Firewall>,
    #[allow(dead_code)]
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Firewall {
    /// A unique ID that can be used to identify and reference a firewall.
    pub id: String,
    /// A status string indicating the current state of the firewall. This can be "waiting",
    /// "succeeded", or "failed".
    pub status: String,
    /// A time value given in ISO8601 combined date and time format that represents when the
    /// firewall was created.
    pub created_at: String,
    /// An array of objects each containing the fields "droplet_id", "removing", and "status". It is
    /// provided to detail exactly which Droplets are having their security policies updated. When
    /// empty, all changes have been successfully applied.
    pub pending_changes: Vec<FirewallPendingChange>,
    /// A human-readable name for a firewall. The name must begin with an alphanumeric character.
    /// Subsequent characters must either be alphanumeric characters, a period (.), or a dash (-).
    pub name: String,
    /// An array containing the IDs of the Droplets assigned to the firewall.
    pub droplet_ids: Option<Vec<u32>>,
    /// A flat array of tag names as strings to be applied to the resource. Tag names may be for
    /// either existing or new tags.
    pub tags: Option<Vec<String>>,
    pub inbound_rules: Option<Vec<FirewallInboundRule>>,
    pub outbound_rules: Option<Vec<FirewallOutboundRule>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct FirewallPendingChange {
    pub droplet_id: u32,
    pub removing: String,
    pub status: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct FirewallInboundRule {
    /// The type of traffic to be allowed. This may be one of tcp, udp, or icmp.
    pub protocol: String,
    /// The ports on which traffic will be allowed specified as a string containing a single port, a
    /// range (e.g. "8000-9000"), or "0" when all ports are open for a protocol. For ICMP rules this
    /// parameter will always return "0".
    pub ports: String,
    /// An object specifying locations from which inbound traffic will be accepted.
    pub sources: FirewallRuleTarget,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct FirewallOutboundRule {
    /// The type of traffic to be allowed. This may be one of tcp, udp, or icmp.
    pub protocol: String,
    /// The ports on which traffic will be allowed specified as a string containing a single port, a
    /// range (e.g. "8000-9000"), or "0" when all ports are open for a protocol. For ICMP rules this
    /// parameter will always return "0".
    pub ports: String,
    /// An object specifying locations to which outbound traffic that will be allowed.
    pub destinations: FirewallRuleTarget,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct FirewallRuleTarget {
    /// An array of strings containing the IPv4 addresses, IPv6 addresses, IPv4 CIDRs, and/or IPv6
    /// CIDRs to which the firewall will allow traffic.
    pub addresses: Option<Vec<String>>,
    /// An array containing the IDs of the Droplets to which the firewall will allow traffic.
    pub droplet_ids: Option<Vec<u32>>,
    /// An array containing the IDs of the load balancers to which the firewall will allow traffic.
    pub load_balancer_uids: Option<Vec<String>>,
    /// An array containing the IDs of the Kubernetes clusters to which the firewall will allow
    /// traffic.
    pub kubernetes_ids: Option<Vec<String>>,
    /// A flat array of tag names as strings to be applied to the resource. Tag names may be for
    /// either existing or new tags.
    pub tags: Option<Vec<String>>,
}

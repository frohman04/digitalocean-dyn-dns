use crate::digitalocean::api::{DigitalOceanApiClient, ErrorResponse, Links, Meta};
use crate::digitalocean::error::Error;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::info;

pub trait DigitalOceanFirewallClient {
    fn get_firewall(&self, name: String) -> Result<Option<Firewall>, Error>;

    fn delete_firewall_rule(
        &self,
        id: &str,
        inbound_rules: Option<Vec<FirewallInboundRule>>,
        outbound_rules: Option<Vec<FirewallOutboundRule>>,
        dry_run: &bool,
    ) -> Result<(), Error>;

    fn add_firewall_rule(
        &self,
        id: &str,
        inbound_rules: Option<Vec<FirewallInboundRule>>,
        outbound_rules: Option<Vec<FirewallOutboundRule>>,
        dry_run: &bool,
    ) -> Result<(), Error>;
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
        self.api.get_object_by_name(
            name.as_str(),
            self.api.get_url("/v2/firewalls"),
            |r: FirewallsResp| r.firewalls,
            |r: &FirewallsResp| r.links.clone(),
            |t: &Firewall, name: &str| t.name == *name,
        )
    }

    /// Delete the provided rules from the firewall identified by `id`.
    fn delete_firewall_rule(
        &self,
        id: &str,
        inbound_rules: Option<Vec<FirewallInboundRule>>,
        outbound_rules: Option<Vec<FirewallOutboundRule>>,
        dry_run: &bool,
    ) -> Result<(), Error> {
        if *dry_run {
            info!(
                "DRY RUN: Delete following rules from firewall {}\ninbound: {:#?}\noutbound: {:#?}",
                id, inbound_rules, outbound_rules
            );
            Ok(())
        } else {
            let url = self
                .api
                .get_url(format!("/v2/firewalls/{}/rules", id).as_str());

            let resp = self
                .api
                .get_request_builder(Method::DELETE, url)
                .json(&FirewallRuleBody {
                    inbound_rules,
                    outbound_rules,
                })
                .send()?;
            match resp.status() {
                StatusCode::NO_CONTENT => Ok(()),
                code => {
                    let error = resp.json::<ErrorResponse>()?;
                    Err(Error::DeleteFirewallRule(format!(
                        "Got unexpected HTTP error from API ({}): {:?}",
                        code, error
                    )))
                }
            }
        }
    }

    /// Add rules to the firewall identified by `id`.  Note that rules are defined by their entire
    /// definition, so calling this will never overwrite an existing rule.
    fn add_firewall_rule(
        &self,
        id: &str,
        inbound_rules: Option<Vec<FirewallInboundRule>>,
        outbound_rules: Option<Vec<FirewallOutboundRule>>,
        dry_run: &bool,
    ) -> Result<(), Error> {
        if *dry_run {
            info!(
                "DRY RUN: Adding following rules to firewall {}\ninbound: {:?}\noutbound: {:?}",
                id, inbound_rules, outbound_rules
            );
            Ok(())
        } else {
            let url = self
                .api
                .get_url(format!("/v2/firewalls/{}/rules", id).as_str());

            let resp = self
                .api
                .get_request_builder(Method::POST, url)
                .json(&FirewallRuleBody {
                    inbound_rules,
                    outbound_rules,
                })
                .send()?;
            match resp.status() {
                StatusCode::NO_CONTENT => Ok(()),
                code => {
                    let error = resp.json::<ErrorResponse>()?;
                    Err(Error::CreateFirewallRule(format!(
                        "Got unexpected HTTP error from API ({}): {:?}",
                        code, error
                    )))
                }
            }
        }
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

#[derive(Deserialize, Debug, Eq, PartialEq)]
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

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub struct FirewallPendingChange {
    pub droplet_id: u32,
    pub removing: bool,
    pub status: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
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

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
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

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub struct FirewallRuleTarget {
    /// An array of strings containing the IPv4 addresses, IPv6 addresses, IPv4 CIDRs, and/or IPv6
    /// CIDRs to which the firewall will allow traffic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<String>>,
    /// An array containing the IDs of the Droplets to which the firewall will allow traffic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub droplet_ids: Option<Vec<u32>>,
    /// An array containing the IDs of the load balancers to which the firewall will allow traffic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancer_uids: Option<Vec<String>>,
    /// An array containing the IDs of the Kubernetes clusters to which the firewall will allow
    /// traffic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes_ids: Option<Vec<String>>,
    /// A flat array of tag names as strings to be applied to the resource. Tag names may be for
    /// either existing or new tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct FirewallRuleBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbound_rules: Option<Vec<FirewallInboundRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound_rules: Option<Vec<FirewallOutboundRule>>,
}

#[cfg(test)]
mod test {
    use mockito;
    use reqwest::StatusCode;

    use crate::digitalocean::firewall::{Firewall, FirewallInboundRule, FirewallRuleTarget};
    use crate::digitalocean::DigitalOceanClient;

    fn get_firewall_1_json() -> serde_json::Value {
        json!({
            "id": "fw1",
            "status": "succeeded",
            "created_at": "2024-01-01T00:00:00Z",
            "pending_changes": [{
                "droplet_id": 0,
                "removing": false,
                "status": "",
            }],
            "name": "FW 1",
            "droplet_ids": [5],
            "tags": ["foo"],
            "inbound_rules": [{
                "protocol": "tcp",
                "ports": "443",
                "sources": {
                    "addresses": ["1.1.1.1"],
                    "droplet_ids": null,
                    "load_balancer_uuids": null,
                    "kubernetes_ids": null,
                    "tags": null,
                },
            }],
            "outbound_rules": null,
        })
    }

    fn get_firewall_2_obj() -> Firewall {
        Firewall {
            id: "fw2".to_string(),
            status: "succeeded".to_string(),
            created_at: "2024-02-01T00:00:00Z".to_string(),
            pending_changes: vec![],
            name: "FW 2".to_string(),
            droplet_ids: Some(vec![42]),
            tags: Some(vec!["foo".to_string()]),
            inbound_rules: Some(vec![FirewallInboundRule {
                protocol: "tcp".to_string(),
                ports: "80".to_string(),
                sources: FirewallRuleTarget {
                    addresses: Some(vec!["8.8.8.8".to_string()]),
                    droplet_ids: None,
                    load_balancer_uids: None,
                    kubernetes_ids: None,
                    tags: None,
                },
            }]),
            outbound_rules: None,
        }
    }

    fn get_firewall_2_json() -> serde_json::Value {
        json!({
            "id": "fw2",
            "status": "succeeded",
            "created_at": "2024-02-01T00:00:00Z",
            "pending_changes": [],
            "name": "FW 2",
            "droplet_ids": [42],
            "tags": ["foo"],
            "inbound_rules": [{
                "protocol": "tcp",
                "ports": "80",
                "sources": {
                    "addresses": ["8.8.8.8"],
                    "droplet_ids": null,
                    "load_balancer_uuids": null,
                    "kubernetes_ids": null,
                    "tags": null,
                },
            }],
            "outbound_rules": null,
        })
    }

    #[test]
    fn test_get_firewall() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/firewalls")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "firewalls": [
                        get_firewall_1_json(),
                        get_firewall_2_json(),
                    ],
                    "meta": {
                        "total": 2
                    },
                    "links": {}
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .firewall
            .get_firewall("FW 2".to_string());
        assert_eq!(Ok(Some(get_firewall_2_obj())), resp);
        _m.assert();
    }

    #[test]
    fn test_get_firewall_paginated() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/firewalls")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "firewalls": [
                        get_firewall_1_json(),
                    ],
                    "meta": {
                        "total": 2
                    },
                    "links": {
                        "pages": {
                            "next": format!("{}/v2/firewalls?page=2", server.url())
                        }
                    }
                }))
                .unwrap(),
            )
            .create();
        let _m_page2 = server
            .mock("GET", "/v2/firewalls?page=2")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "firewalls": [
                        get_firewall_2_json(),
                    ],
                    "meta": {
                        "total": 2
                    },
                    "links": {}
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .firewall
            .get_firewall("FW 2".to_string());
        assert_eq!(Ok(Some(get_firewall_2_obj())), resp);
        _m.assert();
        _m_page2.assert();
    }

    #[test]
    fn test_get_firewall_missing() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/firewalls")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "firewalls": [],
                    "meta": {
                        "total": 0
                    },
                    "links": {}
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .firewall
            .get_firewall("FW 2".to_string());
        assert_eq!(Ok(None), resp);
        _m.assert();
    }

    #[test]
    fn test_delete_firewall() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("DELETE", "/v2/firewalls/fw2/rules")
            .match_header("Authorization", "Bearer foo")
            .match_header("Content-Type", "application/json")
            .match_body(mockito::Matcher::Json(json!({
                "inbound_rules": [{
                    "protocol": "tcp",
                    "ports": "443",
                    "sources": {
                        "addresses": ["1.1.1.1"],
                        "droplet_ids": [12345],
                        "load_balancer_uids": ["123-456-789"],
                        "kubernetes_ids": ["98765"],
                        "tags": ["foo"],
                    },
                }],
            })))
            .with_status(StatusCode::NO_CONTENT.as_u16() as usize)
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .firewall
            .delete_firewall_rule(
                &"fw2",
                Some(vec![FirewallInboundRule {
                    protocol: "tcp".to_string(),
                    ports: "443".to_string(),
                    sources: FirewallRuleTarget {
                        addresses: Some(vec!["1.1.1.1".to_string()]),
                        droplet_ids: Some(vec![12345]),
                        load_balancer_uids: Some(vec!["123-456-789".to_string()]),
                        kubernetes_ids: Some(vec!["98765".to_string()]),
                        tags: Some(vec!["foo".to_string()]),
                    },
                }]),
                None,
                &false,
            );
        assert_eq!(Ok(()), resp);
        _m.assert();
    }

    #[test]
    fn test_create_firewall() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("POST", "/v2/firewalls/fw2/rules")
            .match_header("Authorization", "Bearer foo")
            .match_header("Content-Type", "application/json")
            .match_body(mockito::Matcher::Json(json!({
                "inbound_rules": [{
                    "protocol": "tcp",
                    "ports": "443",
                    "sources": {
                        "addresses": ["1.1.1.1"],
                        "droplet_ids": [12345],
                        "load_balancer_uids": ["123-456-789"],
                        "kubernetes_ids": ["98765"],
                        "tags": ["foo"],
                    },
                }],
            })))
            .with_status(StatusCode::NO_CONTENT.as_u16() as usize)
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .firewall
            .add_firewall_rule(
                &"fw2",
                Some(vec![FirewallInboundRule {
                    protocol: "tcp".to_string(),
                    ports: "443".to_string(),
                    sources: FirewallRuleTarget {
                        addresses: Some(vec!["1.1.1.1".to_string()]),
                        droplet_ids: Some(vec![12345]),
                        load_balancer_uids: Some(vec!["123-456-789".to_string()]),
                        kubernetes_ids: Some(vec!["98765".to_string()]),
                        tags: Some(vec!["foo".to_string()]),
                    },
                }]),
                None,
                &false,
            );
        assert_eq!(Ok(()), resp);
        _m.assert();
    }
}

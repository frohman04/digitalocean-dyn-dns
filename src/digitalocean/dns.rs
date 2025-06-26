use std::net::IpAddr;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::digitalocean::api::{DigitalOceanApiClient, Links, Meta};
use crate::digitalocean::error::Error;

pub trait DigitalOceanDnsClient {
    fn get_domain(&self, domain: &str) -> Result<Option<Domain>, Error>;

    fn get_record(
        &self,
        domain: &str,
        record: &str,
        rtype: &str,
    ) -> Result<Option<DomainRecord>, Error>;

    fn update_record(
        &self,
        domain: &str,
        record: &DomainRecord,
        value: &IpAddr,
        ttl: &u16,
        dry_run: &bool,
    ) -> Result<DomainRecord, Error>;

    fn create_record(
        &self,
        domain: &str,
        record: &str,
        rtype: &str,
        value: &IpAddr,
        ttl: &u16,
        dry_run: &bool,
    ) -> Result<DomainRecord, Error>;
}

pub struct DigitalOceanDnsClientImpl {
    api: DigitalOceanApiClient,
}

impl DigitalOceanDnsClientImpl {
    pub fn new(api: DigitalOceanApiClient) -> DigitalOceanDnsClientImpl {
        DigitalOceanDnsClientImpl { api }
    }
}

impl DigitalOceanDnsClient for DigitalOceanDnsClientImpl {
    /// Check to see if a domain is controlled by this DigitalOcean account
    fn get_domain(&self, domain: &str) -> Result<Option<Domain>, Error> {
        let mut url = self.api.get_url("/v2/domains");
        let mut exit = false;
        let mut obj: Option<Domain> = None;

        while !exit {
            let resp = self
                .api
                .get_request_builder(Method::GET, url.clone())
                .send()?
                .json::<DomainsResp>()?;

            obj = resp.domains.into_iter().find(|d| d.name == *domain);
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

    /// Check to see if a domain is controlled by this DigitalOcean account
    fn get_record(
        &self,
        domain: &str,
        record: &str,
        rtype: &str,
    ) -> Result<Option<DomainRecord>, Error> {
        self.api.get_object_by_name(
            record,
            self.api
                .get_url(format!("/v2/domains/{domain}/records?type={rtype}").as_str()),
            |r: DomainRecordsResp| r.domain_records,
            |r: &DomainRecordsResp| r.links.clone(),
            |t: &DomainRecord, name: &str| t.name == *name,
        )
    }

    /// Update an existing DNS A/AAAA record to point to a new IP address
    fn update_record(
        &self,
        domain: &str,
        record: &DomainRecord,
        value: &IpAddr,
        ttl: &u16,
        dry_run: &bool,
    ) -> Result<DomainRecord, Error> {
        if *dry_run {
            info!(
                "DRY RUN: Updating record for {}.{} to {}",
                record.name, domain, value
            );
            Ok(DomainRecord {
                id: 0,
                typ: "".to_string(),
                name: "".to_string(),
                data: "".to_string(),
                priority: None,
                port: None,
                ttl: *ttl,
                weight: None,
                flags: None,
                tag: None,
            })
        } else {
            let url = self
                .api
                .get_url(format!("/v2/domains/{}/records/{}", domain, record.id).as_str());
            let resp = self
                .api
                .get_request_builder(Method::PUT, url)
                .json(&DomainRecordPutBody {
                    data: value.to_string(),
                })
                .send()?
                .json::<DomainRecordsModifyResp>()?;
            if resp.domain_record.data.parse::<IpAddr>()? == *value {
                Ok(resp.domain_record)
            } else {
                Err(Error::UpdateDns(
                    "New IP address not reflected in updated DNS record".to_string(),
                ))
            }
        }
    }

    /// Create a new DNS A/AAAA record to point to an IP address
    fn create_record(
        &self,
        domain: &str,
        record: &str,
        rtype: &str,
        value: &IpAddr,
        ttl: &u16,
        dry_run: &bool,
    ) -> Result<DomainRecord, Error> {
        if *dry_run {
            info!(
                "DRY RUN: Create {} record for {}.{} to {}",
                rtype, record, domain, value
            );
            Ok(DomainRecord {
                id: 0,
                typ: "".to_string(),
                name: "".to_string(),
                data: "".to_string(),
                priority: None,
                port: None,
                ttl: *ttl,
                weight: None,
                flags: None,
                tag: None,
            })
        } else {
            let url = self
                .api
                .get_url(format!("/v2/domains/{domain}/records").as_str());
            let resp = self
                .api
                .get_request_builder(Method::POST, url)
                .json(&DomainRecordPostBody {
                    typ: rtype.to_string(),
                    name: record.to_string(),
                    data: value.to_string(),
                    priority: None,
                    port: None,
                    ttl: 60,
                    weight: None,
                    flags: None,
                    tag: None,
                })
                .send()?
                .json::<DomainRecordsModifyResp>()?;
            if resp.domain_record.data.parse::<IpAddr>()? == *value {
                Ok(resp.domain_record)
            } else {
                Err(Error::CreateDns(
                    "New IP address not reflected in new DNS record".to_string(),
                ))
            }
        }
    }
}

// /v2/domains

#[derive(Deserialize, Debug)]
struct DomainsResp {
    domains: Vec<Domain>,
    #[allow(dead_code)]
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct Domain {
    /// The name of the domain itself.  This should follow the standard domain format of domain.TLD.
    /// For instance, example.com is a valid domain name.
    pub name: String,
    /// This value is the time to live for the records on this domain, in seconds.  This defines the
    /// time frame that clients can cache queried information before a refresh should be requested.
    pub ttl: u16,
    /// This attribute contains the complete contents of the zone file for the selected domain.
    /// Individual domain record resources should be used to get more granular control over records.
    /// However, this attribute can also be used to get information about the SOA record, which is
    /// created automatically and is not accessible as an individual record resource.
    pub zone_file: String,
}

// /v2/domains/[domain]/records

#[derive(Deserialize, Debug)]
struct DomainRecordsResp {
    domain_records: Vec<DomainRecord>,
    #[allow(dead_code)]
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug)]
struct DomainRecordsModifyResp {
    domain_record: DomainRecord,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct DomainRecord {
    /// A unique identifier for each domain record.
    pub id: u32,
    /// The type of the DNS record. For example: A, CNAME, TXT, ...
    #[serde(alias = "type")]
    pub typ: String,
    /// The host name, alias, or service being defined by the record.
    pub name: String,
    /// Variable data depending on record type. For example, the "data" value for an A record would
    /// be the IPv4 address to which the domain will be mapped. For a CAA record, it would contain
    /// the domain name of the CA being granted permission to issue certificates.
    pub data: String,
    /// The priority for SRV and MX records.
    pub priority: Option<u16>,
    /// The port for SRV records.
    pub port: Option<u16>,
    /// This value is the time to live for the record, in seconds. This defines the time frame that
    /// clients can cache queried information before a refresh should be requested
    pub ttl: u16,
    /// The weight for SRV records.
    pub weight: Option<u16>,
    /// An unsigned integer between 0-255 used for CAA records.
    pub flags: Option<u8>,
    /// The parameter tag for CAA records. Valid values are "issue", "issuewild", or "iodef"
    pub tag: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DomainRecordPostBody {
    /// The type of the DNS record. For example: A, CNAME, TXT, ...
    #[serde(rename(serialize = "type"))]
    pub typ: String,
    /// The host name, alias, or service being defined by the record.
    pub name: String,
    /// Variable data depending on record type. For example, the "data" value for an A record would
    /// be the IPv4 address to which the domain will be mapped. For a CAA record, it would contain
    /// the domain name of the CA being granted permission to issue certificates.
    pub data: String,
    /// The priority for SRV and MX records.
    pub priority: Option<u16>,
    /// The port for SRV records.
    pub port: Option<u16>,
    /// This value is the time to live for the record, in seconds. This defines the time frame that
    /// clients can cache queried information before a refresh should be requested
    pub ttl: u16,
    /// The weight for SRV records.
    pub weight: Option<u16>,
    /// An unsigned integer between 0-255 used for CAA records.
    pub flags: Option<u8>,
    /// The parameter tag for CAA records. Valid values are "issue", "issuewild", or "iodef"
    pub tag: Option<String>,
}

#[derive(Serialize, Debug)]
struct DomainRecordPutBody {
    pub data: String,
}

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;

    use mockito;

    use crate::digitalocean::DigitalOceanClient;
    use crate::digitalocean::dns::{Domain, DomainRecord};

    #[test]
    fn test_get_domain_simple_found() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/domains")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domains": [
                        {
                            "name": "google.com",
                            "ttl": 40,
                            "zone_file": "blargh!"
                        },
                        {
                            "name": "yahoo.com",
                            "ttl": 100,
                            "zone_file": "oof"
                        }
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
            .dns
            .get_domain(&"yahoo.com".to_string());
        assert_eq!(
            Ok(Some(Domain {
                name: "yahoo.com".to_string(),
                ttl: 100,
                zone_file: "oof".to_string()
            })),
            resp
        );
        _m.assert();
    }

    #[test]
    fn test_get_domain_paginated_found() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/domains")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domains": [
                        {
                            "name": "google.com",
                            "ttl": 40,
                            "zone_file": "blargh!"
                        }
                    ],
                    "meta": {
                        "total": 1
                    },
                    "links": {
                        "pages": {
                            "next": format!("{}/v2/domains?page=2", server.url())
                        }
                    }
                }))
                .unwrap(),
            )
            .create();
        let _m_page2 = server
            .mock("GET", "/v2/domains?page=2")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domains": [
                        {
                            "name": "yahoo.com",
                            "ttl": 100,
                            "zone_file": "oof"
                        }
                    ],
                    "meta": {
                        "total": 1
                    },
                    "links": {}
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .dns
            .get_domain(&"yahoo.com".to_string());
        assert_eq!(
            Ok(Some(Domain {
                name: "yahoo.com".to_string(),
                ttl: 100,
                zone_file: "oof".to_string()
            })),
            resp
        );
        _m.assert();
        _m_page2.assert();
    }

    #[test]
    fn test_get_domain_missing() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/domains")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domains": [],
                    "meta": {
                        "total": 0
                    },
                    "links": {}
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .dns
            .get_domain(&"yahoo.com".to_string());
        assert_eq!(Ok(None), resp);
        _m.assert();
    }

    #[test]
    fn test_get_record_simple_found() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/domains/google.com/records?type=A")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domain_records": [
                        {
                            "id": 123,
                            "type": "A",
                            "name": "@",
                            "data": "1.2.3.4",
                            "priority": null,
                            "port": null,
                            "ttl": 40,
                            "weight": null,
                            "flags": null,
                            "tag": null
                        },
                        {
                            "id": 234,
                            "type": "A",
                            "name": "foo",
                            "data": "2.3.4.5",
                            "priority": null,
                            "port": null,
                            "ttl": 100,
                            "weight": null,
                            "flags": null,
                            "tag": null
                        }
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
            .dns
            .get_record(
                &"google.com".to_string(),
                &"foo".to_string(),
                &"A".to_string(),
            );
        assert_eq!(
            Ok(Some(DomainRecord {
                id: 234,
                typ: "A".to_string(),
                name: "foo".to_string(),
                data: "2.3.4.5".to_string(),
                priority: None,
                port: None,
                ttl: 100,
                weight: None,
                flags: None,
                tag: None
            })),
            resp
        );
        _m.assert();
    }

    #[test]
    fn test_get_record_paginated_found() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/domains/google.com/records?type=A")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domain_records": [
                        {
                            "id": 123,
                            "type": "A",
                            "name": "@",
                            "data": "1.2.3.4",
                            "priority": null,
                            "port": null,
                            "ttl": 40,
                            "weight": null,
                            "flags": null,
                            "tag": null
                        }
                    ],
                    "meta": {
                        "total": 1
                    },
                    "links": {
                        "pages": {
                            "next": format!("{}/v2/domains/google.com/records?type=A&page=2", server.url())
                        }
                    }
                }))
                    .unwrap(),
            )
            .create();
        let _m_page2 = server
            .mock("GET", "/v2/domains/google.com/records?type=A&page=2")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domain_records": [
                        {
                            "id": 234,
                            "type": "A",
                            "name": "foo",
                            "data": "2.3.4.5",
                            "priority": null,
                            "port": null,
                            "ttl": 100,
                            "weight": null,
                            "flags": null,
                            "tag": null
                        }
                    ],
                    "meta": {
                        "total": 1
                    },
                    "links": {}
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .dns
            .get_record(
                &"google.com".to_string(),
                &"foo".to_string(),
                &"A".to_string(),
            );
        assert_eq!(
            Ok(Some(DomainRecord {
                id: 234,
                typ: "A".to_string(),
                name: "foo".to_string(),
                data: "2.3.4.5".to_string(),
                priority: None,
                port: None,
                ttl: 100,
                weight: None,
                flags: None,
                tag: None
            })),
            resp
        );
        _m.assert();
        _m_page2.assert();
    }

    #[test]
    fn test_get_record_missing() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/domains/google.com/records?type=A")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domain_records": [],
                    "meta": {
                        "total": 0
                    },
                    "links": {}
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .dns
            .get_record(
                &"google.com".to_string(),
                &"foo".to_string(),
                &"A".to_string(),
            );
        assert_eq!(Ok(None), resp);
        _m.assert();
    }

    #[test]
    fn test_update_record() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("PUT", "/v2/domains/google.com/records/234")
            .match_header("Authorization", "Bearer foo")
            .match_header("Content-Type", "application/json")
            .match_body(mockito::Matcher::Json(json!({
                "data": "2.3.4.5"
            })))
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domain_record": {
                        "id": 234,
                        "type": "A",
                        "name": "foo",
                        "data": "2.3.4.5",
                        "priority": null,
                        "port": null,
                        "ttl": 60,
                        "weight": null,
                        "flags": null,
                        "tag": null
                    }
                }))
                .unwrap(),
            )
            .create();

        let orig_record = DomainRecord {
            id: 234,
            typ: "A".to_string(),
            name: "foo".to_string(),
            data: "1.2.3.4".to_string(),
            priority: None,
            port: None,
            ttl: 100,
            weight: None,
            flags: None,
            tag: None,
        };
        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .dns
            .update_record(
                &"google.com".to_string(),
                &orig_record,
                &Ipv4Addr::new(2, 3, 4, 5).into(),
                &60,
                &false,
            );
        assert_eq!(
            Ok(DomainRecord {
                id: 234,
                typ: "A".to_string(),
                name: "foo".to_string(),
                data: "2.3.4.5".to_string(),
                priority: None,
                port: None,
                ttl: 60,
                weight: None,
                flags: None,
                tag: None
            }),
            resp
        );
        _m.assert();
    }

    #[test]
    fn test_create_record() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("POST", "/v2/domains/google.com/records")
            .match_header("Authorization", "Bearer foo")
            .match_header("Content-Type", "application/json")
            .match_body(mockito::Matcher::Json(json!({
                "type": "A",
                "name": "foo",
                "data": "1.2.3.4",
                "priority": null,
                "port": null,
                "ttl": 60,
                "weight": null,
                "flags": null,
                "tag": null
            })))
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "domain_record": {
                        "id": 234,
                        "type": "A",
                        "name": "foo",
                        "data": "1.2.3.4",
                        "priority": null,
                        "port": null,
                        "ttl": 100,
                        "weight": null,
                        "flags": null,
                        "tag": null
                    }
                }))
                .unwrap(),
            )
            .create();

        let resp = DigitalOceanClient::new_for_test("foo".to_string(), server.url())
            .dns
            .create_record(
                &"google.com".to_string(),
                &"foo".to_string(),
                &"A".to_string(),
                &Ipv4Addr::new(1, 2, 3, 4).into(),
                &100,
                &false,
            );
        assert_eq!(
            Ok(DomainRecord {
                id: 234,
                typ: "A".to_string(),
                name: "foo".to_string(),
                data: "1.2.3.4".to_string(),
                priority: None,
                port: None,
                ttl: 100,
                weight: None,
                flags: None,
                tag: None
            }),
            resp
        );
        _m.assert();
    }
}

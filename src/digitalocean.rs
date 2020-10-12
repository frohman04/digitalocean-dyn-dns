use reqwest::blocking::ClientBuilder;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use mockito;

use std::net::IpAddr;

pub struct DigitalOceanClient {
    base_url: String,
    token: String,
}

impl DigitalOceanClient {
    pub fn new(token: String) -> DigitalOceanClient {
        #[cfg(not(test))]
        let base_url = "https://api.digitalocean.com".to_string();
        #[cfg(test)]
        let base_url = mockito::server_url();

        DigitalOceanClient { base_url, token }
    }

    /// Check to see if a domain is controlled by this DigitalOcean account
    pub fn get_domain(&self, domain: &String) -> Result<Option<Domain>, Error> {
        let mut url = format!("{}/v2/domains", self.base_url);
        let mut exit = false;
        let mut obj: Option<Domain> = None;

        while !exit {
            let resp = ClientBuilder::new()
                .build()
                .unwrap()
                .get(&url.clone())
                .header("Authorization", format!("Bearer {}", self.token))
                .send()?
                .json::<DomainsResp>()?;

            obj = resp
                .domains
                .into_iter()
                .filter(|d| d.name == *domain)
                .next();
            if obj.is_some() {
                exit = true;
            } else if resp.links.pages.is_some() && resp.links.pages.clone().unwrap().next.is_some()
            {
                url = resp.links.pages.unwrap().next.unwrap();
                url = url.replace("http://", "https://");
            } else {
                exit = true;
            }
        }

        Ok(obj)
    }

    /// Check to see if a domain is controlled by this DigitalOcean account
    pub fn get_record(
        &self,
        domain: &String,
        record: &String,
        rtype: &String,
    ) -> Result<Option<DomainRecord>, Error> {
        let mut url = format!(
            "{}/v2/domains/{}/records?type={}",
            self.base_url, domain, rtype
        );
        let mut exit = false;
        let mut obj: Option<DomainRecord> = None;

        while !exit {
            let resp = ClientBuilder::new()
                .build()
                .unwrap()
                .get(&url.clone())
                .header("Authorization", format!("Bearer {}", self.token))
                .send()?
                .json::<DomainRecordsResp>()?;

            obj = resp
                .domain_records
                .into_iter()
                .filter(|r| r.name == *record)
                .next();
            if obj.is_some() {
                exit = true;
            } else if resp.links.pages.is_some() && resp.links.pages.clone().unwrap().next.is_some()
            {
                url = resp.links.pages.unwrap().next.unwrap();
                url = url.replace("http://", "https://");
            } else {
                exit = true;
            }
        }

        Ok(obj)
    }

    /// Update an existing DNS A/AAAA record to point to a new IP address
    pub fn update_record(
        &self,
        domain: &String,
        record: &DomainRecord,
        value: &IpAddr,
    ) -> Result<DomainRecord, Error> {
        let url = format!(
            "{}/v2/domains/{}/records/{}",
            self.base_url, domain, record.id
        );
        let resp = ClientBuilder::new()
            .build()
            .unwrap()
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&DomainRecordPutBody {
                data: value.to_string(),
            })
            .send()?
            .json::<DomainRecordsModifyResp>()?;
        if resp.domain_record.data.parse::<IpAddr>()? == *value {
            Ok(resp.domain_record)
        } else {
            Err(Error::Update(
                "New IP address not reflected in updated DNS record".to_string(),
            ))
        }
    }

    /// Create a new DNS A/AAAA record to point to an IP address
    pub fn create_record(
        &self,
        domain: &String,
        record: &String,
        rtype: &String,
        value: &IpAddr,
    ) -> Result<DomainRecord, Error> {
        let url = format!("{}/v2/domains/{}/records", self.base_url, domain);
        let resp = ClientBuilder::new()
            .build()
            .unwrap()
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&DomainRecordPostBody {
                typ: rtype.clone(),
                name: record.clone(),
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
            Err(Error::Create(
                "New IP address not reflected in new DNS record".to_string(),
            ))
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Request(reqwest::Error),
    IpParse(std::net::AddrParseError),
    Update(String),
    Create(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Request(e)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Error::IpParse(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// common parts of responses for collections

#[derive(Deserialize, Debug)]
struct Meta {
    total: u32,
}

#[derive(Deserialize, Debug)]
struct Links {
    pages: Option<Pages>,
}

#[derive(Deserialize, Debug, Clone)]
struct Pages {
    first: Option<String>,
    prev: Option<String>,
    next: Option<String>,
    last: Option<String>,
}

// /v2/domains

#[derive(Deserialize, Debug)]
struct DomainsResp {
    domains: Vec<Domain>,
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug)]
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
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug)]
struct DomainRecordsModifyResp {
    domain_record: DomainRecord,
}

#[derive(Deserialize, Debug)]
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

use reqwest::blocking::ClientBuilder;
use serde::Deserialize;

pub struct DigitalOceanClient {
    token: String,
}

impl DigitalOceanClient {
    pub fn new(token: String) -> DigitalOceanClient {
        DigitalOceanClient { token }
    }

    /// Check to see if a domain is controlled by this DigitalOcean account
    pub fn get_domain(&self, domain: String) -> Result<Option<Domain>, reqwest::Error> {
        let mut url = "https://api.digitalocean.com/v2/domains".to_string();
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

            obj = resp.domains.into_iter().filter(|d| d.name == domain).next();
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
        domain: String,
        record: String,
        rtype: String,
    ) -> Result<Option<DomainRecord>, reqwest::Error> {
        let mut url = format!(
            "https://api.digitalocean.com/v2/domains/{}/records?type={}",
            domain, rtype
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
                .filter(|r| r.name == record)
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

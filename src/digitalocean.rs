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
    pub fn domain_exists(&self, domain: String) -> Result<bool, reqwest::Error> {
        let mut url = "https://api.digitalocean.com/v2/domains".to_string();
        let mut exit = false;
        let mut found = false;

        while !exit {
            let resp = ClientBuilder::new()
                .build()
                .unwrap()
                .get(&url.clone())
                .header("Authorization", format!("Bearer {}", self.token))
                .send()?
                .json::<DomainsResp>()?;

            if resp.domains.iter().any(|d| d.name == domain) {
                exit = true;
                found = true;
            } else if resp.links.pages.is_some() && resp.links.pages.clone().unwrap().next.is_some()
            {
                url = resp.links.pages.unwrap().next.unwrap();
                url = url.replace("http://", "https://");
            } else {
                exit = true;
                found = false;
            }
        }

        Ok(found)
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
struct Domain {
    /// The name of the domain itself.  This should follow the standard domain format of domain.TLD.
    /// For instance, example.com is a valid domain name.
    name: String,
    /// This value is the time to live for the records on this domain, in seconds.  This defines the
    /// time frame that clients can cache queried information before a refresh should be requested.
    ttl: u16,
    /// This attribute contains the complete contents of the zone file for the selected domain.
    /// Individual domain record resources should be used to get more granular control over records.
    /// However, this attribute can also be used to get information about the SOA record, which is
    /// created automatically and is not accessible as an individual record resource.
    zone_file: String,
}

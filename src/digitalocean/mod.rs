use crate::digitalocean::api::DigitalOceanApiClient;
use crate::digitalocean::dns::{DigitalOceanDnsClient, DigitalOceanDnsClientImpl};
use crate::digitalocean::firewall::{DigitalOceanFirewallClient, DigitalOceanFirewallClientImpl};

pub mod api;
pub mod dns;
pub mod error;
pub mod firewall;

#[allow(dead_code)]
pub struct DigitalOceanClient {
    api: DigitalOceanApiClient,
    pub dns: Box<dyn DigitalOceanDnsClient>,
    pub firewall: Box<dyn DigitalOceanFirewallClient>,
}

impl DigitalOceanClient {
    pub fn new(token: String) -> DigitalOceanClient {
        let api = DigitalOceanApiClient::new(token);
        DigitalOceanClient {
            api: api.clone(),
            dns: Box::new(DigitalOceanDnsClientImpl::new(api.clone())),
            firewall: Box::new(DigitalOceanFirewallClientImpl::new(api)),
        }
    }

    #[cfg(test)]
    pub fn new_for_test(token: String, base_url: String) -> DigitalOceanClient {
        let api = DigitalOceanApiClient::new_for_test(token, base_url);
        DigitalOceanClient {
            api: api.clone(),
            dns: Box::new(DigitalOceanDnsClientImpl::new(api.clone())),
            firewall: Box::new(DigitalOceanFirewallClientImpl::new(api)),
        }
    }
}

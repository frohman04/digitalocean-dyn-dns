use crate::digitalocean::api::DigitalOceanApiClient;
use crate::digitalocean::dns::{DigitalOceanDnsClient, DigitalOceanDnsClientImpl};
use crate::digitalocean::droplet::{DigitalOceanDropletClient, DigitalOceanDropletClientImpl};
use crate::digitalocean::firewall::{DigitalOceanFirewallClient, DigitalOceanFirewallClientImpl};

pub mod api;
pub mod dns;
pub mod droplet;
pub mod error;
pub mod firewall;

#[allow(dead_code)]
pub struct DigitalOceanClient {
    api: DigitalOceanApiClient,
    pub dns: Box<dyn DigitalOceanDnsClient>,
    pub firewall: Box<dyn DigitalOceanFirewallClient>,
    pub droplet: Box<dyn DigitalOceanDropletClient>,
}

impl DigitalOceanClient {
    pub fn new(token: String) -> DigitalOceanClient {
        DigitalOceanClient::new_for_client(DigitalOceanApiClient::new(token))
    }

    fn new_for_client(api: DigitalOceanApiClient) -> DigitalOceanClient {
        DigitalOceanClient {
            api: api.clone(),
            dns: Box::new(DigitalOceanDnsClientImpl::new(api.clone())),
            firewall: Box::new(DigitalOceanFirewallClientImpl::new(api.clone())),
            droplet: Box::new(DigitalOceanDropletClientImpl::new(api)),
        }
    }

    #[cfg(test)]
    pub fn new_for_test(token: String, base_url: String) -> DigitalOceanClient {
        DigitalOceanClient::new_for_client(DigitalOceanApiClient::new_for_test(token, base_url))
    }
}

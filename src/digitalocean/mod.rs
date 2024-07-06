use crate::digitalocean::api::DigitalOceanApiClient;
use crate::digitalocean::dns::{DigitalOceanDnsClient, DigitalOceanDnsClientImpl};
use crate::digitalocean::droplet::{DigitalOceanDropletClient, DigitalOceanDropletClientImpl};
use crate::digitalocean::firewall::{DigitalOceanFirewallClient, DigitalOceanFirewallClientImpl};
use crate::digitalocean::kubernetes::{
    DigitalOceanKubernetesClient, DigitalOceanKubernetesClientImpl,
};
use crate::digitalocean::loadbalancer::{
    DigitalOceanLoadbalancerClient, DigitalOceanLoadbalancerClientImpl,
};
use std::rc::Rc;

pub mod api;
pub mod dns;
pub mod droplet;
pub mod error;
pub mod firewall;
pub mod kubernetes;
pub mod loadbalancer;

#[allow(dead_code)]
pub struct DigitalOceanClient {
    api: DigitalOceanApiClient,
    pub dns: Rc<dyn DigitalOceanDnsClient>,
    pub droplet: Rc<dyn DigitalOceanDropletClient>,
    pub firewall: Rc<dyn DigitalOceanFirewallClient>,
    pub kubernetes: Rc<dyn DigitalOceanKubernetesClient>,
    pub load_balancer: Rc<dyn DigitalOceanLoadbalancerClient>,
}

impl DigitalOceanClient {
    pub fn new(token: String) -> DigitalOceanClient {
        DigitalOceanClient::new_for_client(DigitalOceanApiClient::new(token))
    }

    fn new_for_client(api: DigitalOceanApiClient) -> DigitalOceanClient {
        DigitalOceanClient {
            api: api.clone(),
            dns: Rc::new(DigitalOceanDnsClientImpl::new(api.clone())),
            droplet: Rc::new(DigitalOceanDropletClientImpl::new(api.clone())),
            firewall: Rc::new(DigitalOceanFirewallClientImpl::new(api.clone())),
            kubernetes: Rc::new(DigitalOceanKubernetesClientImpl::new(api.clone())),
            load_balancer: Rc::new(DigitalOceanLoadbalancerClientImpl::new(api)),
        }
    }

    #[cfg(test)]
    pub fn new_for_test(token: String, base_url: String) -> DigitalOceanClient {
        DigitalOceanClient::new_for_client(DigitalOceanApiClient::new_for_test(token, base_url))
    }
}

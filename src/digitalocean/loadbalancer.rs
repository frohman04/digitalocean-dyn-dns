use crate::digitalocean::api::{DigitalOceanApiClient, Links, Meta};
use crate::digitalocean::error::Error;
use reqwest::Method;
use serde::Deserialize;

pub trait DigitalOceanLoadbalancerClient {
    fn get_load_balancers(&self) -> Result<Vec<Loadbalancer>, Error>;
}

pub struct DigitalOceanLoadbalancerClientImpl {
    api: DigitalOceanApiClient,
}

impl DigitalOceanLoadbalancerClientImpl {
    pub fn new(api: DigitalOceanApiClient) -> DigitalOceanLoadbalancerClientImpl {
        DigitalOceanLoadbalancerClientImpl { api }
    }
}

impl DigitalOceanLoadbalancerClient for DigitalOceanLoadbalancerClientImpl {
    /// Get info on all load balancers.
    fn get_load_balancers(&self) -> Result<Vec<Loadbalancer>, Error> {
        let mut url = self.api.get_url("/v2/load_balancers");
        let mut exit = false;
        let mut droplets: Vec<Loadbalancer> = Vec::new();

        while !exit {
            let resp = self
                .api
                .get_request_builder(Method::GET, url.clone())
                .send()?
                .json::<LoadbalancersResp>()?;

            droplets.extend(resp.load_balancers.into_iter());
            if resp.links.pages.is_some() && resp.links.pages.clone().unwrap().next.is_some() {
                url = resp.links.pages.unwrap().next.unwrap();
            } else {
                exit = true;
            }
        }

        Ok(droplets)
    }
}

// /v2/load_balancers

#[derive(Deserialize, Debug)]
struct LoadbalancersResp {
    load_balancers: Vec<Loadbalancer>,
    #[allow(dead_code)]
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Loadbalancer {
    /// A unique ID that can be used to identify and reference a load balancer.
    pub id: String,
    /// A human-readable name for a load balancer instance.
    pub name: String,
    /// The ID of the project that the load balancer is associated with. If no ID is provided at
    /// creation, the load balancer associates with the user's default project. If an invalid
    /// project ID is provided, the load balancer will not be created
    pub project_id: String,
    /// An attribute containing the public-facing IP address of the load balancer.
    pub ip: String,
    /// How many nodes the load balancer contains. Each additional node increases the load
    /// balancer's ability to manage more connections. Load balancers can be scaled up or down, and
    /// you can change the number of nodes after creation up to once per hour. This field is
    /// currently not available in the AMS2, NYC2, or SFO1 regions. Use the size field to scale load
    /// balancers that reside in these regions.
    /// range [ 1 .. 100 ]
    pub size_unit: u8,
    /// This field has been replaced by the size_unit field for all regions except in AMS2, NYC2,
    /// and SFO1. Each available load balancer size now equates to the load balancer having a set
    /// number of nodes.
    ///
    ///     lb-small = 1 node
    ///     lb-medium = 3 nodes
    ///     lb-large = 6 nodes
    ///
    /// You can resize load balancers after creation up to once per hour. You cannot resize a load
    /// balancer within the first hour of its creation.
    #[deprecated]
    pub size: String,
    /// This field has been deprecated. You can no longer specify an algorithm for load balancers.
    /// values: "round_robin" "least_connections"
    #[deprecated]
    pub algorithm: String,
    /// A status string indicating the current state of the load balancer. This can be new, active,
    /// or errored.
    pub status: String,
    /// A time value given in ISO8601 combined date and time format that represents when the load
    /// balancer was created.
    pub created_at: String,
    /// An array of objects specifying the forwarding rules for a load balancer.
    pub forwarding_rules: Vec<LoadbalancerForwardingRule>,
    /// An object specifying health check settings for the load balancer.
    pub health_check: LoadbalancerHealthCheck,
    /// An object specifying sticky sessions settings for the load balancer.
    pub sticky_sessions: LoadbalancerStickySessions,
    /// A boolean value indicating whether HTTP requests to the load balancer on port 80 will be
    /// redirected to HTTPS on port 443.
    pub redirect_http_to_https: bool,
    /// A boolean value indicating whether PROXY Protocol is in use.
    pub enable_proxy_protocol: bool,
    /// A boolean value indicating whether HTTP keepalive connections are maintained to target
    /// Droplets.
    pub enable_backend_keepalive: bool,
    /// An integer value which configures the idle timeout for HTTP requests to the target droplets
    /// range [ 30 .. 60 ]
    pub http_idle_timeout_seconds: u16,
    /// A string specifying the UUID of the VPC to which the load balancer is assigned.
    pub vpc_uuid: String,
    /// A boolean value indicating whether to disable automatic DNS record creation for Let's
    /// Encrypt certificates that are added to the load balancer.
    pub disable_lets_encrypt_dns_records: bool,
    /// An object specifying allow and deny rules to control traffic to the load balancer.
    pub firewall: LoadbalancerFirewall,
    /// The region where the load balancer instance is located. When setting a region, the value
    /// should be the slug identifier for the region. When you query a load balancer, an entire
    /// region object will be returned.
    pub region: LoadbalancerRegion,
    /// An array containing the IDs of the Droplets assigned to the load balancer.
    pub droplet_ids: Vec<u32>,
    /// The name of a Droplet tag corresponding to Droplets assigned to the load balancer.
    pub tag: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct LoadbalancerForwardingRule {
    /// The protocol used for traffic to the load balancer. The possible values are: http, https,
    /// http2, http3, tcp, or udp. If you set the entry_protocol to udp, the target_protocol must be
    /// set to udp. When using UDP, the load balancer requires that you set up a health check with a
    /// port that uses TCP, HTTP, or HTTPS to work properly.
    pub entry_protocol: String,
    /// An integer representing the port on which the load balancer instance will listen.
    pub entry_port: u16,
    /// The protocol used for traffic from the load balancer to the backend Droplets. The possible
    /// values are: http, https, http2, tcp, or udp. If you set the target_protocol to udp, the
    /// entry_protocol must be set to udp. When using UDP, the load balancer requires that you set
    /// up a health check with a port that uses TCP, HTTP, or HTTPS to work properly.
    pub target_protocol: String,
    /// An integer representing the port on the backend Droplets to which the load balancer will
    /// send traffic.
    pub target_port: u16,
    /// The ID of the TLS certificate used for SSL termination if enabled.
    pub certificate_id: Option<String>,
    /// A boolean value indicating whether SSL encrypted traffic will be passed through to the
    /// backend Droplets.
    pub tls_passthrough: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct LoadbalancerHealthCheck {
    /// The protocol used for health checks sent to the backend Droplets. The possible values are
    /// http, https, or tcp
    pub protocol: String,
    /// An integer representing the port on the backend Droplets on which the health check will
    /// attempt a connection.
    pub port: u16,
    /// The path on the backend Droplets to which the load balancer instance will send a request.
    pub path: String,
    /// The number of seconds between two consecutive health checks.
    pub check_interval_seconds: u16,
    /// The number of seconds the load balancer instance will wait for a response until marking a
    /// health check as failed
    pub response_timeout_seconds: u16,
    /// The number of times a health check must fail for a backend Droplet to be marked "unhealthy"
    /// and be removed from the pool.
    pub unhealthy_threshold: u8,
    /// The number of times a health check must pass for a backend Droplet to be marked "healthy"
    /// and be re-added to the pool.
    pub healthy_threshold: u8,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct LoadbalancerStickySessions {
    /// An attribute indicating how and if requests from a client will be persistently served by the
    /// same backend Droplet. The possible values are cookies or none.
    #[serde(alias = "type")]
    pub typ: String,
    /// The name of the cookie sent to the client. This attribute is only returned when using
    /// cookies for the sticky sessions type.
    pub cookie_name: String,
    /// The number of seconds until the cookie set by the load balancer expires. This attribute is
    /// only returned when using cookies for the sticky sessions type.
    pub cookie_ttl_seconds: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct LoadbalancerFirewall {
    /// the rules for denying traffic to the load balancer (in the form 'ip:1.2.3.4' or
    /// 'cidr:1.2.0.0/16')
    pub deny: Vec<String>,
    /// the rules for allowing traffic to the load balancer (in the form 'ip:1.2.3.4' or
    /// 'cidr:1.2.0.0/16')
    pub allow: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct LoadbalancerRegion {
    /// The display name of the region. This will be a full name that is used in the control panel
    /// and other interfaces.
    pub name: String,
    /// A human-readable string that is used as a unique identifier for each region.
    pub slug: String,
    /// This attribute is set to an array which contains features available in this region.
    pub features: Vec<String>,
    /// This is a boolean value that represents whether new Droplets can be created in this region.
    pub available: bool,
    /// This attribute is set to an array which contains the identifying slugs for the sizes
    /// available in this region.
    pub sizes: Vec<String>,
}

use crate::digitalocean::api::{DigitalOceanApiClient, Links, Meta};
use crate::digitalocean::error::Error;
use serde::Deserialize;

pub trait DigitalOceanDropletClient {
    fn get_droplets(&self) -> Result<Vec<Droplet>, Error>;
}

pub struct DigitalOceanDropletClientImpl {
    api: DigitalOceanApiClient,
}

impl DigitalOceanDropletClientImpl {
    pub fn new(api: DigitalOceanApiClient) -> DigitalOceanDropletClientImpl {
        DigitalOceanDropletClientImpl { api }
    }
}

impl DigitalOceanDropletClient for DigitalOceanDropletClientImpl {
    /// Get info on all droplets.
    fn get_droplets(&self) -> Result<Vec<Droplet>, Error> {
        self.api.get_all_objects(
            self.api.get_url("/v2/droplets"),
            |r: DropletsResp| r.droplets,
            |r: &DropletsResp| r.links.clone(),
        )
    }
}

// /v2/droplets

#[derive(Deserialize, Debug)]
struct DropletsResp {
    droplets: Vec<Droplet>,
    #[allow(dead_code)]
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[allow(dead_code)]
pub struct Droplet {
    /// A unique identifier for each Droplet instance. This is automatically generated upon Droplet
    /// creation.
    pub id: u32,
    /// The human-readable name set for the Droplet instance.
    pub name: String,
    /// Memory of the Droplet in megabytes. (multiple of 8)
    pub memory: u32,
    /// The number of virtual CPUs.
    pub vcpus: u8,
    /// The size of the Droplet's disk in gigabytes.
    pub disk: u16,
    /// A boolean value indicating whether the Droplet has been locked, preventing actions by users.
    pub locked: bool,
    /// A status string indicating the state of the Droplet instance. This may be "new", "active",
    /// "off", or "archive".
    pub status: String,
    /// These Droplets will have this attribute set to null. The current kernel for Droplets with
    /// externally managed kernels. This will initially be set to the kernel of the base image when
    /// the Droplet is created.
    #[deprecated(note = "All Droplets created after March 2017 use internal kernels by default.")]
    pub kernel: Option<DropletKernel>,
    /// A time value given in ISO8601 combined date and time format that represents when the Droplet
    /// was created.
    pub created_at: String,
    /// An array of features enabled on this Droplet.
    pub features: Vec<String>,
    /// An array of backup IDs of any backups that have been taken of the Droplet instance. Droplet
    /// backups are enabled at the time of the instance creation.
    pub backup_ids: Vec<u32>,
    /// The details of the Droplet's backups feature, if backups are configured for the Droplet.
    /// This object contains keys for the start and end times of the window during which the backup
    /// will start.
    pub next_backup_window: Option<DropletNextBackupWindow>,
    /// An array of snapshot IDs of any snapshots created from the Droplet instance.
    pub snapshot_ids: Vec<u32>,
    pub image: DropletImage,
    /// A flat array including the unique identifier for each Block Storage volume attached to the
    /// Droplet.
    pub volume_ids: Vec<String>,
    pub size: DropletSize,
    /// The unique slug identifier for the size of this Droplet.
    pub size_slug: String,
    /// The details of the network that are configured for the Droplet instance. This is an object
    /// that contains keys for IPv4 and IPv6. The value of each of these is an array that contains
    /// objects describing an individual IP resource allocated to the Droplet. These will define
    /// attributes like the IP address, netmask, and gateway of the specific network depending on
    /// the type of network it is.
    pub networks: DropletNetworks,
    pub region: DropletRegion,
    /// An array of Tags the Droplet has been tagged with.
    pub tags: Vec<String>,
    /// A string specifying the UUID of the VPC to which the Droplet is assigned.
    pub vpc_uuid: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletKernel {
    /// A unique number used to identify and reference a specific kernel.
    pub id: u32,
    /// The display name of the kernel. This is shown in the web UI and is generally a descriptive
    /// title for the kernel in question.
    pub name: String,
    /// A standard kernel version string representing the version, patch, and release information.
    pub version: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletNextBackupWindow {
    /// A time value given in ISO8601 combined date and time format specifying the start of the
    /// Droplet's backup window.
    pub start: String,
    /// A time value given in ISO8601 combined date and time format specifying the end of the
    /// Droplet's backup window.
    pub end: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletImage {
    /// A unique number that can be used to identify and reference a specific image.
    pub id: u32,
    /// The display name that has been given to an image. This is what is shown in the control panel
    /// and is generally a descriptive title for the image in question.
    pub name: String,
    /// Describes the kind of image. It may be one of base, snapshot, backup, custom, or admin.
    /// Respectively, this specifies whether an image is a DigitalOcean base OS image,
    /// user-generated Droplet snapshot, automatically created Droplet backup, user-provided virtual
    /// machine image, or an image used for DigitalOcean managed resources (e.g. DOKS worker nodes).
    #[serde(alias = "type")]
    pub typ: String,
    /// The name of a custom image's distribution. Currently, the valid values are Arch Linux,
    /// CentOS, CoreOS, Debian, Fedora, Fedora Atomic, FreeBSD, Gentoo, openSUSE, RancherOS,
    /// Rocky Linux, Ubuntu, and Unknown. Any other value will be accepted but ignored, and Unknown
    /// will be used in its place.
    pub distribution: String,
    /// A uniquely identifying string that is associated with each of the DigitalOcean-provided
    /// public images. These can be used to reference a public image as an alternative to the
    /// numeric id.
    pub slug: Option<String>,
    /// This is a boolean value that indicates whether the image in question is public or not. An
    /// image that is public is available to all accounts. A non-public image is only accessible
    /// from your account.
    pub public: bool,
    /// This attribute is an array of the regions that the image is available in. The regions are
    /// represented by their identifying slug values.
    /// Items Enum: "ams1" "ams2" "ams3" "blr1" "fra1" "lon1" "nyc1" "nyc2" "nyc3" "sfo1" "sfo2"
    /// "sfo3" "sgp1" "tor1"
    pub regions: Vec<String>,
    /// A time value given in ISO8601 combined date and time format that represents when the image
    /// was created.
    pub created_at: String,
    /// The minimum disk size in GB required for a Droplet to use this image.
    pub min_disk_size: Option<u16>,
    /// The size of the image in gigabytes.
    pub size_gigabytes: Option<f32>,
    /// An optional free-form text field to describe an image.
    pub description: Option<String>,
    /// A flat array of tag names as strings to be applied to the resource. Tag names may be for
    /// either existing or new tags.
    pub tags: Vec<String>,
    /// A status string indicating the state of a custom image. This may be NEW, available, pending,
    /// deleted, or retired.
    pub status: String,
    /// A string containing information about errors that may occur when importing a custom image.
    pub error_message: Option<String>,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletSize {
    /// A human-readable string that is used to uniquely identify each size.
    pub slug: String,
    /// The amount of RAM allocated to Droplets created of this size. The value is represented in
    /// megabytes. (multiple of 8)
    pub memory: u32,
    /// The integer of number CPUs allocated to Droplets of this size.
    pub vcpus: u16,
    /// The amount of disk space set aside for Droplets of this size. The value is represented in
    /// gigabytes.
    pub disk: u16,
    /// The amount of transfer bandwidth that is available for Droplets created in this size. This
    /// only counts traffic on the public interface. The value is given in terabytes.
    pub transfer: f32,
    /// This attribute describes the monthly cost of this Droplet size if the Droplet is kept for an
    /// entire month. The value is measured in US dollars.
    pub price_monthly: f32,
    /// This describes the price of the Droplet size as measured hourly. The value is measured in US
    /// dollars.
    pub price_hourly: f32,
    /// An array containing the region slugs where this size is available for Droplet creates.
    pub regions: Vec<String>,
    /// This is a boolean value that represents whether new Droplets can be created with this size.
    pub available: bool,
    /// A string describing the class of Droplets created from this size. For example: Basic,
    /// General Purpose, CPU-Optimized, Memory-Optimized, or Storage-Optimized.
    pub description: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletNetworks {
    pub v4: Vec<DropletNetworkV4>,
    pub v6: Vec<DropletNetworkV6>,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletNetworkV4 {
    /// The IP address of the IPv4 network interface.
    pub ip_address: String,
    /// The netmask of the IPv4 network interface.
    pub netmask: String,
    /// The gateway of the specified IPv4 network interface.
    ///
    /// For private interfaces, a gateway is not provided. This is denoted by returning nil as its
    /// value.
    pub gateway: Option<String>,
    /// The type of the IPv4 network interface. (Enum: "public" "private")
    #[serde(alias = "type")]
    pub typ: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletNetworkV6 {
    /// The IP address of the IPv6 network interface.
    pub ip_address: String,
    /// The netmask of the IPv6 network interface.
    pub netmask: u8,
    /// The gateway of the specified IPv6 network interface.
    pub gateway: Option<String>,
    /// The type of the IPv6 network interface. (Enum: "public")
    ///
    /// Note: IPv6 private networking is not currently supported.
    #[serde(alias = "type")]
    pub typ: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct DropletRegion {
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

#[cfg(test)]
mod test {
    use crate::digitalocean::DigitalOceanClient;
    use crate::digitalocean::droplet::{
        Droplet, DropletImage, DropletNetworkV4, DropletNetworks, DropletNextBackupWindow,
        DropletRegion, DropletSize,
    };

    fn get_droplet_1_json() -> serde_json::Value {
        json!({
            "id": 1,
            "name": "bar",
            "memory": 64,
            "vcpus": 2,
            "disk": 100,
            "locked": false,
            "status": "active",
            "kernel": null,
            "created_at": "2024-01-01T04:23:45Z",
            "features": ["backup"],
            "backup_ids": [12345],
            "next_backup_window": {
                "start": "2024-01-02T06:00:00Z",
                "end": "2024-01-02T07:00:00Z"
            },
            "snapshot_ids": [67890],
            "image": {
                "id": 42,
                "name": "image1",
                "type": "backup",
                "distribution": "Ubuntu",
                "slug": "ubuntu",
                "public": true,
                "regions": ["nyc1"],
                "created_at": "2024-01-01T00:00:00Z",
                "min_disk_size": 100,
                "size_gigabytes": 64.1,
                "description": "initial backup after creation",
                "tags": ["amazing"],
                "status": "available",
                "error_message": null,
            },
            "volume_ids": [],
            "size": {
                "slug": "small",
                "memory": 64,
                "vcpus": 2,
                "disk": 100,
                "transfer": 1.0,
                "price_monthly": 360.0,
                "price_hourly": 0.5,
                "regions": ["nyc1", "nyc2"],
                "available": true,
                "description": "a small instance for dev",
            },
            "size_slug": "small",
            "networks": {
                "v4": [{
                    "ip_address": "1.2.3.4",
                    "netmask": "1.2.3.0/28",
                    "gateway": "1.2.3.0",
                    "type": "public",
                }],
                "v6": [],
            },
            "region": {
                "name": "NYC 1",
                "slug": "nyc1",
                "features": ["backup"],
                "available": true,
                "sizes": ["small"],
            },
            "tags": ["awesome"],
            "vpc_uuid": "123-456-789",
        })
    }

    fn get_droplet_1_obj() -> Droplet {
        #[allow(deprecated)]
        Droplet {
            id: 1,
            name: "bar".to_string(),
            memory: 64,
            vcpus: 2,
            disk: 100,
            locked: false,
            status: "active".to_string(),
            kernel: None,
            created_at: "2024-01-01T04:23:45Z".to_string(),
            features: vec!["backup".to_string()],
            backup_ids: vec![12345],
            next_backup_window: Some(DropletNextBackupWindow {
                start: "2024-01-02T06:00:00Z".to_string(),
                end: "2024-01-02T07:00:00Z".to_string(),
            }),
            snapshot_ids: vec![67890],
            image: DropletImage {
                id: 42,
                name: "image1".to_string(),
                typ: "backup".to_string(),
                distribution: "Ubuntu".to_string(),
                slug: Some("ubuntu".to_string()),
                public: true,
                regions: vec!["nyc1".to_string()],
                created_at: "2024-01-01T00:00:00Z".to_string(),
                min_disk_size: Some(100),
                size_gigabytes: Some(64.1),
                description: Some("initial backup after creation".to_string()),
                tags: vec!["amazing".to_string()],
                status: "available".to_string(),
                error_message: None,
            },
            volume_ids: vec![],
            size: DropletSize {
                slug: "small".to_string(),
                memory: 64,
                vcpus: 2,
                disk: 100,
                transfer: 1.0,
                price_monthly: 360.0,
                price_hourly: 0.5,
                regions: vec!["nyc1".to_string(), "nyc2".to_string()],
                available: true,
                description: "a small instance for dev".to_string(),
            },
            size_slug: "small".to_string(),
            networks: DropletNetworks {
                v4: vec![DropletNetworkV4 {
                    ip_address: "1.2.3.4".to_string(),
                    netmask: "1.2.3.0/28".to_string(),
                    gateway: Some("1.2.3.0".to_string()),
                    typ: "public".to_string(),
                }],
                v6: vec![],
            },
            region: DropletRegion {
                name: "NYC 1".to_string(),
                slug: "nyc1".to_string(),
                features: vec!["backup".to_string()],
                available: true,
                sizes: vec!["small".to_string()],
            },
            tags: vec!["awesome".to_string()],
            vpc_uuid: "123-456-789".to_string(),
        }
    }

    fn get_droplet_2_json() -> serde_json::Value {
        json!({
            "id": 2,
            "name": "snake",
            "memory": 32,
            "vcpus": 1,
            "disk": 200,
            "locked": false,
            "status": "active",
            "kernel": null,
            "created_at": "2023-01-01T04:23:45Z",
            "features": ["backup"],
            "backup_ids": [1234],
            "next_backup_window": {
                "start": "2024-02-02T06:00:00Z",
                "end": "2024-02-02T07:00:00Z"
            },
            "snapshot_ids": [6789],
            "image": {
                "id": 21,
                "name": "image2",
                "type": "backup",
                "distribution": "Debian",
                "slug": "debian",
                "public": true,
                "regions": ["nyc1"],
                "created_at": "2020-01-01T00:00:00Z",
                "min_disk_size": 50,
                "size_gigabytes": 34.1,
                "description": "initial backup after creation",
                "tags": ["amazing"],
                "status": "available",
                "error_message": null,
            },
            "volume_ids": [],
            "size": {
                "slug": "small",
                "memory": 64,
                "vcpus": 2,
                "disk": 100,
                "transfer": 1.0,
                "price_monthly": 360.0,
                "price_hourly": 0.5,
                "regions": ["nyc1", "nyc2"],
                "available": true,
                "description": "a small instance for dev",
            },
            "size_slug": "small",
            "networks": {
                "v4": [{
                    "ip_address": "10.20.30.41",
                    "netmask": "10.20.30.40/28",
                    "gateway": "1.2.3.0",
                    "type": "public",
                }],
                "v6": [],
            },
            "region": {
                "name": "NYC 1",
                "slug": "nyc1",
                "features": ["backup"],
                "available": true,
                "sizes": ["small"],
            },
            "tags": ["awesome"],
            "vpc_uuid": "123-456-789",
        })
    }

    fn get_droplet_2_obj() -> Droplet {
        #[allow(deprecated)]
        Droplet {
            id: 2,
            name: "snake".to_string(),
            memory: 32,
            vcpus: 1,
            disk: 200,
            locked: false,
            status: "active".to_string(),
            kernel: None,
            created_at: "2023-01-01T04:23:45Z".to_string(),
            features: vec!["backup".to_string()],
            backup_ids: vec![1234],
            next_backup_window: Some(DropletNextBackupWindow {
                start: "2024-02-02T06:00:00Z".to_string(),
                end: "2024-02-02T07:00:00Z".to_string(),
            }),
            snapshot_ids: vec![6789],
            image: DropletImage {
                id: 21,
                name: "image2".to_string(),
                typ: "backup".to_string(),
                distribution: "Debian".to_string(),
                slug: Some("debian".to_string()),
                public: true,
                regions: vec!["nyc1".to_string()],
                created_at: "2020-01-01T00:00:00Z".to_string(),
                min_disk_size: Some(50),
                size_gigabytes: Some(34.1),
                description: Some("initial backup after creation".to_string()),
                tags: vec!["amazing".to_string()],
                status: "available".to_string(),
                error_message: None,
            },
            volume_ids: vec![],
            size: DropletSize {
                slug: "small".to_string(),
                memory: 64,
                vcpus: 2,
                disk: 100,
                transfer: 1.0,
                price_monthly: 360.0,
                price_hourly: 0.5,
                regions: vec!["nyc1".to_string(), "nyc2".to_string()],
                available: true,
                description: "a small instance for dev".to_string(),
            },
            size_slug: "small".to_string(),
            networks: DropletNetworks {
                v4: vec![DropletNetworkV4 {
                    ip_address: "10.20.30.41".to_string(),
                    netmask: "10.20.30.40/28".to_string(),
                    gateway: Some("1.2.3.0".to_string()),
                    typ: "public".to_string(),
                }],
                v6: vec![],
            },
            region: DropletRegion {
                name: "NYC 1".to_string(),
                slug: "nyc1".to_string(),
                features: vec!["backup".to_string()],
                available: true,
                sizes: vec!["small".to_string()],
            },
            tags: vec!["awesome".to_string()],
            vpc_uuid: "123-456-789".to_string(),
        }
    }

    #[test]
    fn test_get_droplets() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/droplets")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "droplets": [
                        get_droplet_1_json(),
                        get_droplet_2_json(),
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
            .droplet
            .get_droplets();
        assert_eq!(Ok(vec![get_droplet_1_obj(), get_droplet_2_obj()]), resp);
        _m.assert();
    }

    #[test]
    fn test_get_droplets_paginated() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/droplets")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "droplets": [
                        get_droplet_1_json(),
                    ],
                    "meta": {
                        "total": 2
                    },
                    "links": {
                        "pages": {
                            "next": format!("{}/v2/droplets?page=2", server.url())
                        }
                    }
                }))
                .unwrap(),
            )
            .create();
        let _m_page2 = server
            .mock("GET", "/v2/droplets?page=2")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "droplets": [
                        get_droplet_2_json(),
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
            .droplet
            .get_droplets();
        assert_eq!(Ok(vec![get_droplet_1_obj(), get_droplet_2_obj()]), resp);
        _m.assert();
        _m_page2.assert();
    }
}

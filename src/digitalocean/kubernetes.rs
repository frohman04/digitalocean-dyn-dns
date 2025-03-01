use crate::digitalocean::api::{DigitalOceanApiClient, Links, Meta};
use crate::digitalocean::error::Error;
use serde::Deserialize;
use std::collections::HashMap;

pub trait DigitalOceanKubernetesClient {
    fn get_kubernetes_clusters(&self) -> Result<Vec<KubernetesCluster>, Error>;
}

pub struct DigitalOceanKubernetesClientImpl {
    api: DigitalOceanApiClient,
}

impl DigitalOceanKubernetesClientImpl {
    pub fn new(api: DigitalOceanApiClient) -> DigitalOceanKubernetesClientImpl {
        DigitalOceanKubernetesClientImpl { api }
    }
}

impl DigitalOceanKubernetesClient for DigitalOceanKubernetesClientImpl {
    /// Get info on all kubernetes clusters.
    fn get_kubernetes_clusters(&self) -> Result<Vec<KubernetesCluster>, Error> {
        self.api.get_all_objects(
            self.api.get_url("/v2/kubernetes/clusters"),
            |r: KubernetesClusterResp| r.kubernetes_clusters,
            |r: &KubernetesClusterResp| r.links.clone(),
        )
    }
}

// /v2/kubernetes/clusters

#[derive(Deserialize, Debug)]
struct KubernetesClusterResp {
    kubernetes_clusters: Vec<KubernetesCluster>,
    #[allow(dead_code)]
    meta: Meta,
    links: Links,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct KubernetesCluster {
    /// A unique ID that can be used to identify and reference a Kubernetes cluster.
    pub id: String,
    /// A human-readable name for a Kubernetes cluster.
    pub name: String,
    /// The slug identifier for the region where the Kubernetes cluster is located.
    pub region: String,
    /// The slug identifier for the version of Kubernetes used for the cluster. If set to a minor
    /// version (e.g. "1.14"), the latest version within it will be used (e.g. "1.14.6-do.1"); if
    /// set to "latest", the latest published version will be used. See the /v2/kubernetes/options
    /// endpoint to find all currently available versions.
    pub version: String,
    /// The range of IP addresses in the overlay network of the Kubernetes cluster in CIDR notation.
    pub cluster_subnet: String,
    /// The range of assignable IP addresses for services running in the Kubernetes cluster in CIDR
    /// notation.
    pub service_subnet: String,
    /// A string specifying the UUID of the VPC to which the Kubernetes cluster is assigned.
    pub vpc_uuid: String,
    /// The public IPv4 address of the Kubernetes master node. This will not be set if high
    /// availability is configured on the cluster (v1.21+)
    pub ipv4: Option<String>,
    /// The base URL of the API server on the Kubernetes master node.
    pub endpoint: String,
    /// An array of tags applied to the Kubernetes cluster. All clusters are automatically tagged
    /// k8s and k8s:$K8S_CLUSTER_ID.
    pub tags: Vec<String>,
    /// An object specifying the details of the worker nodes available to the Kubernetes cluster.
    pub node_pools: Vec<KubernetesClusterNodePool>,
    /// An object specifying the maintenance window policy for the Kubernetes cluster.
    pub maintenance_policy: Option<KubernetesClusterMaintenancePolicy>,
    /// A boolean value indicating whether the cluster will be automatically upgraded to new patch
    /// releases during its maintenance window.
    pub auto_upgrade: bool,
    /// An object containing a state attribute whose value is set to a string indicating the current
    /// status of the cluster.
    pub status: KubernetesClusterStatus,
    /// A time value given in ISO8601 combined date and time format that represents when the
    /// Kubernetes cluster was created.
    pub created_at: String,
    /// A time value given in ISO8601 combined date and time format that represents when the
    /// Kubernetes cluster was last updated.
    pub updated_at: String,
    /// A boolean value indicating whether surge upgrade is enabled/disabled for the cluster. Surge
    /// upgrade makes cluster upgrades fast and reliable by bringing up new nodes before destroying
    /// the outdated nodes.
    pub surge_upgrade: bool,
    /// A boolean value indicating whether the control plane is run in a highly available
    /// configuration in the cluster. Highly available control planes incur less downtime. The
    /// property cannot be disabled.
    pub ha: bool,
    /// A read-only boolean value indicating if a container registry is integrated with the cluster.
    pub registry_enabled: bool,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct KubernetesClusterNodePool {
    /// The slug identifier for the type of Droplet used as workers in the node pool.
    pub size: String,
    /// A unique ID that can be used to identify and reference a specific node pool.
    pub id: String,
    /// A human-readable name for the node pool.
    pub name: String,
    /// The number of Droplet instances in the node pool.
    pub count: u16,
    /// An array containing the tags applied to the node pool. All node pools are automatically
    /// tagged k8s, k8s-worker, and k8s:$K8S_CLUSTER_ID.
    pub tags: Vec<String>,
    /// An object of key/value mappings specifying labels to apply to all nodes in a pool. Labels
    /// will automatically be applied to all existing nodes and any subsequent nodes added to the
    /// pool. Note that when a label is removed, it is not deleted from the nodes in the pool.
    pub labels: Option<HashMap<String, String>>,
    /// An array of taints to apply to all nodes in a pool. Taints will automatically be applied to
    /// all existing nodes and any subsequent nodes added to the pool. When a taint is removed, it
    /// is deleted from all nodes in the pool.
    pub taints: Vec<KubernetesClusterNodePoolTaint>,
    /// A boolean value indicating whether auto-scaling is enabled for this node pool.
    pub auto_scale: bool,
    /// The minimum number of nodes that this node pool can be auto-scaled to. The value will be 0
    /// if auto_scale is set to false.
    pub min_nodes: u16,
    /// The maximum number of nodes that this node pool can be auto-scaled to. The value will be 0
    /// if auto_scale is set to false.
    pub max_nodes: u16,
    /// An object specifying the details of a specific worker node in a node pool.
    pub nodes: Vec<KubernetesClusterNodePoolNode>,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct KubernetesClusterNodePoolTaint {
    /// An arbitrary string. The key and value fields of the taint object form a key-value pair. For
    /// example, if the value of the key field is "special" and the value of the value field is
    /// "gpu", the key value pair would be special=gpu.
    pub key: String,
    /// An arbitrary string. The key and value fields of the taint object form a key-value pair. For
    /// example, if the value of the key field is "special" and the value of the value field is
    /// "gpu", the key value pair would be special=gpu.
    pub value: String,
    /// How the node reacts to pods that it won't tolerate. Available effect values are NoSchedule,
    /// PreferNoSchedule, and NoExecute.
    pub effect: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct KubernetesClusterNodePoolNode {
    /// A unique ID that can be used to identify and reference the node.
    pub id: String,
    /// An automatically generated, human-readable name for the node.
    pub name: String,
    /// An object containing a state attribute whose value is set to a string indicating the current
    /// status of the node.
    pub status: KubernetesClusterNodePoolNodeState,
    /// The ID of the Droplet used for the worker node.
    pub droplet_id: String,
    /// A time value given in ISO8601 combined date and time format that represents when the node
    /// was created.
    pub created_at: String,
    /// A time value given in ISO8601 combined date and time format that represents when the node
    /// was last updated.
    pub updated_at: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct KubernetesClusterNodePoolNodeState {
    /// A string indicating the current status of the node.
    /// values: "provisioning" "running" "draining" "deleting"
    pub state: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct KubernetesClusterMaintenancePolicy {
    /// The start time in UTC of the maintenance window policy in 24-hour clock format / HH:MM
    /// notation (e.g., 15:00).
    pub start_time: String,
    /// The duration of the maintenance window policy in human-readable format.
    pub duration: String,
    /// The day of the maintenance window policy. May be one of monday through sunday, or any to
    /// indicate an arbitrary week day.
    pub day: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub struct KubernetesClusterStatus {
    /// A string indicating the current status of the cluster.
    /// values: "running" "provisioning" "degraded" "error" "deleted" "upgrading" "deleting"
    pub state: String,
    /// An optional message providing additional information about the current cluster state.
    pub message: Option<String>,
}

#[cfg(test)]
mod test {
    use crate::digitalocean::DigitalOceanClient;
    use crate::digitalocean::kubernetes::{
        KubernetesCluster, KubernetesClusterMaintenancePolicy, KubernetesClusterNodePool,
        KubernetesClusterNodePoolNode, KubernetesClusterNodePoolNodeState,
        KubernetesClusterNodePoolTaint, KubernetesClusterStatus,
    };
    use std::collections::HashMap;

    fn get_cluster_1_json() -> serde_json::Value {
        json!({
            "id": "1",
            "name": "cluster1",
            "region": "nyc1",
            "version": "1.26.5",
            "cluster_subnet": "10.0.0.0/24",
            "service_subnet": "10.0.1.0/24",
            "vpc_uuid": "123-456-789",
            "ipv4": "10.0.0.1",
            "endpoint": "http://cluster1.kube.digitalocean.com",
            "tags": ["awesome"],
            "node_pools": [{
                "size": "small",
                "id": "42",
                "name": "nodes1",
                "count": 10,
                "tags": [],
                "labels": {
                    "foo": "bar",
                },
                "taints": [{
                    "key": "key",
                    "value": "value",
                    "effect": "NoSchedule",
                }],
                "auto_scale": false,
                "min_nodes": 0,
                "max_nodes": 0,
                "nodes": [{
                    "id": "100",
                    "name": "node1",
                    "status": {
                        "state": "running",
                    },
                    "droplet_id": "987-654-321",
                    "created_at": "2024-01-01T04:00:00Z",
                    "updated_at": "2024-01-01T04:00:00Z",
                }]
            }],
            "maintenance_policy": {
                "start_time": "15:00",
                "duration": "1 hour",
                "day": "monday",
            },
            "auto_upgrade": false,
            "status": {
                "state": "running",
                "message": null,
            },
            "created_at": "2024-01-01T04:00:00Z",
            "updated_at": "2024-01-01T04:00:00Z",
            "surge_upgrade": false,
            "ha": false,
            "registry_enabled": false,
        })
    }

    fn get_cluster_1_obj() -> KubernetesCluster {
        KubernetesCluster {
            id: "1".to_string(),
            name: "cluster1".to_string(),
            region: "nyc1".to_string(),
            version: "1.26.5".to_string(),
            cluster_subnet: "10.0.0.0/24".to_string(),
            service_subnet: "10.0.1.0/24".to_string(),
            vpc_uuid: "123-456-789".to_string(),
            ipv4: Some("10.0.0.1".to_string()),
            endpoint: "http://cluster1.kube.digitalocean.com".to_string(),
            tags: vec!["awesome".to_string()],
            node_pools: vec![KubernetesClusterNodePool {
                size: "small".to_string(),
                id: "42".to_string(),
                name: "nodes1".to_string(),
                count: 10,
                tags: vec![],
                labels: Some(HashMap::from([("foo".to_string(), "bar".to_string())])),
                taints: vec![KubernetesClusterNodePoolTaint {
                    key: "key".to_string(),
                    value: "value".to_string(),
                    effect: "NoSchedule".to_string(),
                }],
                auto_scale: false,
                min_nodes: 0,
                max_nodes: 0,
                nodes: vec![KubernetesClusterNodePoolNode {
                    id: "100".to_string(),
                    name: "node1".to_string(),
                    status: KubernetesClusterNodePoolNodeState {
                        state: "running".to_string(),
                    },
                    droplet_id: "987-654-321".to_string(),
                    created_at: "2024-01-01T04:00:00Z".to_string(),
                    updated_at: "2024-01-01T04:00:00Z".to_string(),
                }],
            }],
            maintenance_policy: Some(KubernetesClusterMaintenancePolicy {
                start_time: "15:00".to_string(),
                duration: "1 hour".to_string(),
                day: "monday".to_string(),
            }),
            auto_upgrade: false,
            status: KubernetesClusterStatus {
                state: "running".to_string(),
                message: None,
            },
            created_at: "2024-01-01T04:00:00Z".to_string(),
            updated_at: "2024-01-01T04:00:00Z".to_string(),
            surge_upgrade: false,
            ha: false,
            registry_enabled: false,
        }
    }

    fn get_cluster_2_json() -> serde_json::Value {
        json!({
            "id": "2",
            "name": "cluster2",
            "region": "nyc2",
            "version": "1.28.2",
            "cluster_subnet": "10.1.0.0/24",
            "service_subnet": "10.1.1.0/24",
            "vpc_uuid": "123-456-789",
            "ipv4": null,
            "endpoint": "http://cluster2.kube.digitalocean.com",
            "tags": ["amazing"],
            "node_pools": [{
                "size": "small",
                "id": "48",
                "name": "nodes2",
                "count": 5,
                "tags": ["what"],
                "labels": {
                    "baz": "snake",
                },
                "taints": [{
                    "key": "key",
                    "value": "value",
                    "effect": "NoSchedule",
                }],
                "auto_scale": true,
                "min_nodes": 5,
                "max_nodes": 10,
                "nodes": [{
                    "id": "200",
                    "name": "node2",
                    "status": {
                        "state": "running",
                    },
                    "droplet_id": "987-654-321",
                    "created_at": "2024-02-01T04:00:00Z",
                    "updated_at": "2024-02-01T04:00:00Z",
                }]
            }],
            "maintenance_policy": {
                "start_time": "10:00",
                "duration": "2 hours",
                "day": "tuesday",
            },
            "auto_upgrade": false,
            "status": {
                "state": "running",
                "message": null,
            },
            "created_at": "2024-02-01T04:00:00Z",
            "updated_at": "2024-02-01T04:00:00Z",
            "surge_upgrade": false,
            "ha": true,
            "registry_enabled": false,
        })
    }

    fn get_cluster_2_obj() -> KubernetesCluster {
        KubernetesCluster {
            id: "2".to_string(),
            name: "cluster2".to_string(),
            region: "nyc2".to_string(),
            version: "1.28.2".to_string(),
            cluster_subnet: "10.1.0.0/24".to_string(),
            service_subnet: "10.1.1.0/24".to_string(),
            vpc_uuid: "123-456-789".to_string(),
            ipv4: None,
            endpoint: "http://cluster2.kube.digitalocean.com".to_string(),
            tags: vec!["amazing".to_string()],
            node_pools: vec![KubernetesClusterNodePool {
                size: "small".to_string(),
                id: "48".to_string(),
                name: "nodes2".to_string(),
                count: 5,
                tags: vec!["what".to_string()],
                labels: Some(HashMap::from([("baz".to_string(), "snake".to_string())])),
                taints: vec![KubernetesClusterNodePoolTaint {
                    key: "key".to_string(),
                    value: "value".to_string(),
                    effect: "NoSchedule".to_string(),
                }],
                auto_scale: true,
                min_nodes: 5,
                max_nodes: 10,
                nodes: vec![KubernetesClusterNodePoolNode {
                    id: "200".to_string(),
                    name: "node2".to_string(),
                    status: KubernetesClusterNodePoolNodeState {
                        state: "running".to_string(),
                    },
                    droplet_id: "987-654-321".to_string(),
                    created_at: "2024-02-01T04:00:00Z".to_string(),
                    updated_at: "2024-02-01T04:00:00Z".to_string(),
                }],
            }],
            maintenance_policy: Some(KubernetesClusterMaintenancePolicy {
                start_time: "10:00".to_string(),
                duration: "2 hours".to_string(),
                day: "tuesday".to_string(),
            }),
            auto_upgrade: false,
            status: KubernetesClusterStatus {
                state: "running".to_string(),
                message: None,
            },
            created_at: "2024-02-01T04:00:00Z".to_string(),
            updated_at: "2024-02-01T04:00:00Z".to_string(),
            surge_upgrade: false,
            ha: true,
            registry_enabled: false,
        }
    }

    #[test]
    fn test_get_clusters() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/kubernetes/clusters")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "kubernetes_clusters": [
                        get_cluster_1_json(),
                        get_cluster_2_json(),
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
            .kubernetes
            .get_kubernetes_clusters();
        assert_eq!(Ok(vec![get_cluster_1_obj(), get_cluster_2_obj()]), resp);
        _m.assert();
    }

    #[test]
    fn test_get_clusters_paginated() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/v2/kubernetes/clusters")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "kubernetes_clusters": [
                        get_cluster_1_json(),
                    ],
                    "meta": {
                        "total": 2
                    },
                    "links": {
                        "pages": {
                            "next": format!("{}/v2/kubernetes/clusters?page=2", server.url())
                        }
                    }
                }))
                .unwrap(),
            )
            .create();
        let _m_page2 = server
            .mock("GET", "/v2/kubernetes/clusters?page=2")
            .match_header("Authorization", "Bearer foo")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(
                serde_json::to_string(&json!({
                    "kubernetes_clusters": [
                        get_cluster_2_json(),
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
            .kubernetes
            .get_kubernetes_clusters();
        assert_eq!(Ok(vec![get_cluster_1_obj(), get_cluster_2_obj()]), resp);
        _m.assert();
        _m_page2.assert();
    }
}

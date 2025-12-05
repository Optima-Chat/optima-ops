//! Infrastructure status module - AWS resource monitoring
//!
//! This module provides infrastructure monitoring for AWS resources.
//! By default, it uses mock data for fast compilation.
//! Enable the "aws" feature to use real AWS SDK calls.

use serde::{Deserialize, Serialize};

#[cfg(feature = "aws")]
use tracing::info;

/// EC2 instance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ec2Status {
    pub instance_id: String,
    pub name: String,
    pub state: String,
    pub instance_type: String,
    pub public_ip: Option<String>,
    pub private_ip: Option<String>,
}

/// ECS service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsServiceStatus {
    pub service_name: String,
    pub cluster: String,
    pub desired_count: i32,
    pub running_count: i32,
    pub pending_count: i32,
    pub status: String,
}

/// ECS cluster status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsClusterStatus {
    pub cluster_name: String,
    pub status: String,
    pub running_tasks: i32,
    pub pending_tasks: i32,
    pub registered_container_instances: i32,
}

/// RDS instance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdsStatus {
    pub identifier: String,
    pub engine: String,
    pub status: String,
    pub endpoint: Option<String>,
    pub instance_class: String,
}

/// ALB status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbStatus {
    pub name: String,
    pub dns_name: String,
    pub state: String,
    pub target_groups: Vec<TargetGroupStatus>,
}

/// Target group status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetGroupStatus {
    pub name: String,
    pub healthy_count: i32,
    pub unhealthy_count: i32,
}

/// Combined infrastructure status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InfrastructureStatus {
    pub ec2_instances: Vec<Ec2Status>,
    pub ecs_clusters: Vec<EcsClusterStatus>,
    pub ecs_services: Vec<EcsServiceStatus>,
    pub rds_instances: Vec<RdsStatus>,
    pub albs: Vec<AlbStatus>,
    pub last_updated: Option<String>,
    pub error: Option<String>,
}

impl InfrastructureStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_error(error: String) -> Self {
        Self {
            error: Some(error),
            ..Default::default()
        }
    }
}

/// Infrastructure client for AWS queries
pub struct InfraClient {
    region: String,
}

impl InfraClient {
    pub fn new(region: &str) -> Self {
        Self {
            region: region.to_string(),
        }
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    /// Get full infrastructure status
    /// Uses real AWS SDK when "aws" feature is enabled, otherwise returns mock data
    #[cfg(feature = "aws")]
    pub async fn get_status(&self) -> InfrastructureStatus {
        info!("Fetching real AWS infrastructure status for region: {}", self.region);

        match self.fetch_real_status().await {
            Ok(status) => status,
            Err(e) => InfrastructureStatus::with_error(format!("AWS error: {}", e)),
        }
    }

    #[cfg(not(feature = "aws"))]
    pub async fn get_status(&self) -> InfrastructureStatus {
        // Return mock data for fast development
        self.get_mock_status()
    }

    /// Mock data for development (no AWS SDK needed)
    #[allow(dead_code)]
    fn get_mock_status(&self) -> InfrastructureStatus {
        InfrastructureStatus {
            ec2_instances: vec![
                Ec2Status {
                    instance_id: "i-0abc123def456".to_string(),
                    name: "optima-prod".to_string(),
                    state: "running".to_string(),
                    instance_type: "t3.medium".to_string(),
                    public_ip: Some("54.123.45.67".to_string()),
                    private_ip: Some("10.0.1.100".to_string()),
                },
            ],
            ecs_clusters: vec![
                EcsClusterStatus {
                    cluster_name: "optima-cluster".to_string(),
                    status: "ACTIVE".to_string(),
                    running_tasks: 5,
                    pending_tasks: 0,
                    registered_container_instances: 2,
                },
            ],
            ecs_services: vec![
                EcsServiceStatus {
                    service_name: "user-auth-stage".to_string(),
                    cluster: "optima-cluster".to_string(),
                    desired_count: 1,
                    running_count: 1,
                    pending_count: 0,
                    status: "ACTIVE".to_string(),
                },
                EcsServiceStatus {
                    service_name: "commerce-backend-stage".to_string(),
                    cluster: "optima-cluster".to_string(),
                    desired_count: 1,
                    running_count: 1,
                    pending_count: 0,
                    status: "ACTIVE".to_string(),
                },
            ],
            rds_instances: vec![
                RdsStatus {
                    identifier: "optima-prod-postgres".to_string(),
                    engine: "postgres".to_string(),
                    status: "available".to_string(),
                    endpoint: Some("optima-prod-postgres.xxx.rds.amazonaws.com".to_string()),
                    instance_class: "db.t3.medium".to_string(),
                },
            ],
            albs: vec![
                AlbStatus {
                    name: "optima-prod-alb".to_string(),
                    dns_name: "optima-prod-alb-xxx.ap-southeast-1.elb.amazonaws.com".to_string(),
                    state: "active".to_string(),
                    target_groups: vec![
                        TargetGroupStatus {
                            name: "user-auth-tg".to_string(),
                            healthy_count: 1,
                            unhealthy_count: 0,
                        },
                    ],
                },
            ],
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
            error: None,
        }
    }

    /// Real AWS SDK implementation
    #[cfg(feature = "aws")]
    async fn fetch_real_status(&self) -> Result<InfrastructureStatus, Box<dyn std::error::Error + Send + Sync>> {
        use aws_config::BehaviorVersion;

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;

        let ec2_client = aws_sdk_ec2::Client::new(&config);
        let ecs_client = aws_sdk_ecs::Client::new(&config);
        let rds_client = aws_sdk_rds::Client::new(&config);
        let elb_client = aws_sdk_elasticloadbalancingv2::Client::new(&config);

        // Fetch EC2 instances
        let ec2_instances = self.fetch_ec2_instances(&ec2_client).await?;

        // Fetch ECS clusters and services
        let (ecs_clusters, ecs_services) = self.fetch_ecs_status(&ecs_client).await?;

        // Fetch RDS instances
        let rds_instances = self.fetch_rds_instances(&rds_client).await?;

        // Fetch ALBs
        let albs = self.fetch_albs(&elb_client).await?;

        Ok(InfrastructureStatus {
            ec2_instances,
            ecs_clusters,
            ecs_services,
            rds_instances,
            albs,
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
            error: None,
        })
    }

    #[cfg(feature = "aws")]
    async fn fetch_ec2_instances(&self, client: &aws_sdk_ec2::Client) -> Result<Vec<Ec2Status>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = client.describe_instances().send().await?;
        let mut instances = Vec::new();

        for reservation in resp.reservations() {
            for instance in reservation.instances() {
                let name = instance.tags()
                    .iter()
                    .find(|t| t.key() == Some("Name"))
                    .and_then(|t| t.value())
                    .unwrap_or("unnamed")
                    .to_string();

                instances.push(Ec2Status {
                    instance_id: instance.instance_id().unwrap_or("").to_string(),
                    name,
                    state: instance.state()
                        .map(|s| s.name().map(|n| n.as_str()).unwrap_or("unknown"))
                        .unwrap_or("unknown")
                        .to_string(),
                    instance_type: instance.instance_type()
                        .map(|t| t.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    public_ip: instance.public_ip_address().map(|s| s.to_string()),
                    private_ip: instance.private_ip_address().map(|s| s.to_string()),
                });
            }
        }

        Ok(instances)
    }

    #[cfg(feature = "aws")]
    async fn fetch_ecs_status(&self, client: &aws_sdk_ecs::Client) -> Result<(Vec<EcsClusterStatus>, Vec<EcsServiceStatus>), Box<dyn std::error::Error + Send + Sync>> {
        let clusters_resp = client.list_clusters().send().await?;
        let cluster_arns: Vec<String> = clusters_resp.cluster_arns().iter().map(|s| s.to_string()).collect();

        let mut clusters = Vec::new();
        let mut services = Vec::new();

        if !cluster_arns.is_empty() {
            let desc_resp = client.describe_clusters()
                .set_clusters(Some(cluster_arns.clone()))
                .send()
                .await?;

            for cluster in desc_resp.clusters() {
                clusters.push(EcsClusterStatus {
                    cluster_name: cluster.cluster_name().unwrap_or("").to_string(),
                    status: cluster.status().unwrap_or("").to_string(),
                    running_tasks: cluster.running_tasks_count(),
                    pending_tasks: cluster.pending_tasks_count(),
                    registered_container_instances: cluster.registered_container_instances_count(),
                });

                // Fetch services for this cluster
                let svc_resp = client.list_services()
                    .cluster(cluster.cluster_arn().unwrap_or(""))
                    .send()
                    .await?;

                let svc_arns: Vec<String> = svc_resp.service_arns().iter().map(|s| s.to_string()).collect();

                if !svc_arns.is_empty() {
                    let desc_svc = client.describe_services()
                        .cluster(cluster.cluster_arn().unwrap_or(""))
                        .set_services(Some(svc_arns))
                        .send()
                        .await?;

                    for svc in desc_svc.services() {
                        services.push(EcsServiceStatus {
                            service_name: svc.service_name().unwrap_or("").to_string(),
                            cluster: cluster.cluster_name().unwrap_or("").to_string(),
                            desired_count: svc.desired_count(),
                            running_count: svc.running_count(),
                            pending_count: svc.pending_count(),
                            status: svc.status().unwrap_or("").to_string(),
                        });
                    }
                }
            }
        }

        Ok((clusters, services))
    }

    #[cfg(feature = "aws")]
    async fn fetch_rds_instances(&self, client: &aws_sdk_rds::Client) -> Result<Vec<RdsStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = client.describe_db_instances().send().await?;
        let mut instances = Vec::new();

        for db in resp.db_instances() {
            instances.push(RdsStatus {
                identifier: db.db_instance_identifier().unwrap_or("").to_string(),
                engine: db.engine().unwrap_or("").to_string(),
                status: db.db_instance_status().unwrap_or("").to_string(),
                endpoint: db.endpoint().and_then(|e| e.address()).map(|s| s.to_string()),
                instance_class: db.db_instance_class().unwrap_or("").to_string(),
            });
        }

        Ok(instances)
    }

    #[cfg(feature = "aws")]
    async fn fetch_albs(&self, client: &aws_sdk_elasticloadbalancingv2::Client) -> Result<Vec<AlbStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = client.describe_load_balancers().send().await?;
        let mut albs = Vec::new();

        for lb in resp.load_balancers() {
            // Get target groups for this ALB
            let tg_resp = client.describe_target_groups()
                .load_balancer_arn(lb.load_balancer_arn().unwrap_or(""))
                .send()
                .await?;

            let mut target_groups = Vec::new();
            for tg in tg_resp.target_groups() {
                // Get target health
                let health_resp = client.describe_target_health()
                    .target_group_arn(tg.target_group_arn().unwrap_or(""))
                    .send()
                    .await?;

                let healthy = health_resp.target_health_descriptions()
                    .iter()
                    .filter(|h| h.target_health().map(|th| th.state().map(|s| s.as_str()) == Some("healthy")).unwrap_or(false))
                    .count() as i32;
                let unhealthy = health_resp.target_health_descriptions()
                    .iter()
                    .filter(|h| h.target_health().map(|th| th.state().map(|s| s.as_str()) != Some("healthy")).unwrap_or(true))
                    .count() as i32;

                target_groups.push(TargetGroupStatus {
                    name: tg.target_group_name().unwrap_or("").to_string(),
                    healthy_count: healthy,
                    unhealthy_count: unhealthy,
                });
            }

            albs.push(AlbStatus {
                name: lb.load_balancer_name().unwrap_or("").to_string(),
                dns_name: lb.dns_name().unwrap_or("").to_string(),
                state: lb.state().map(|s| s.code().map(|c| c.as_str()).unwrap_or("unknown")).unwrap_or("unknown").to_string(),
                target_groups,
            });
        }

        Ok(albs)
    }
}

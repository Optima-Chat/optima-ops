//! CloudWatch monitoring client for EC2 and ECS metrics
//!
//! Provides functionality to:
//! - Get current CPU/memory utilization for EC2 instances
//! - Get historical metrics for sparkline charts
//! - Get ECS service metrics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// EC2 instance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ec2Metrics {
    pub instance_id: String,
    pub instance_name: String,
    pub environment: String,
    pub state: String,
    pub cpu_current: Option<f64>,
    pub memory_current: Option<f64>,
    pub cpu_avg_1h: Option<f64>,
    pub cpu_history: Vec<f64>, // 24 data points for sparkline
}

impl Ec2Metrics {
    pub fn cpu_sparkline(&self) -> String {
        render_sparkline(&self.cpu_history)
    }
}

/// ECS service metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsServiceMetrics {
    pub service_name: String,
    pub cluster_name: String,
    pub running_count: i32,
    pub desired_count: i32,
    pub cpu_utilization: Option<f64>,
    pub memory_utilization: Option<f64>,
}

/// ECS cluster summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsClusterSummary {
    pub cluster_name: String,
    pub running_tasks: i32,
    pub pending_tasks: i32,
    pub container_instances: i32,
    pub active_services: i32,
    pub avg_cpu: Option<f64>,
    pub avg_memory: Option<f64>,
}

/// Render a sparkline from values using Unicode block characters
pub fn render_sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = values.iter().cloned().fold(0.0_f64, f64::max);
    let min = values.iter().cloned().fold(f64::MAX, f64::min);
    let range = max - min;

    values
        .iter()
        .map(|v| {
            let normalized = if range > 0.0 {
                (v - min) / range
            } else {
                0.5
            };
            let idx = (normalized * 7.0).round() as usize;
            chars[idx.min(7)]
        })
        .collect()
}

/// Monitoring client for CloudWatch metrics
#[derive(Clone)]
pub struct MonitoringClient {
    region: String,
    #[cfg(feature = "aws")]
    cloudwatch_client: Option<aws_sdk_cloudwatch::Client>,
    #[cfg(feature = "aws")]
    ec2_client: Option<aws_sdk_ec2::Client>,
}

impl MonitoringClient {
    /// Create a new monitoring client
    pub async fn new(region: &str) -> Self {
        #[cfg(feature = "aws")]
        {
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(region.to_string()))
                .load()
                .await;

            Self {
                region: region.to_string(),
                cloudwatch_client: Some(aws_sdk_cloudwatch::Client::new(&config)),
                ec2_client: Some(aws_sdk_ec2::Client::new(&config)),
            }
        }

        #[cfg(not(feature = "aws"))]
        {
            Self {
                region: region.to_string(),
            }
        }
    }

    /// Get metrics for all EC2 instances
    pub async fn get_all_ec2_metrics(&self) -> Vec<Ec2Metrics> {
        #[cfg(feature = "aws")]
        {
            self.fetch_ec2_metrics().await
        }

        #[cfg(not(feature = "aws"))]
        {
            // Return mock data when AWS feature is disabled
            self.mock_ec2_metrics()
        }
    }

    /// Get CPU history for a specific instance (24 data points, 1 per hour)
    pub async fn get_cpu_history(
        &self,
        instance_id: &str,
    ) -> Vec<(DateTime<Utc>, f64)> {
        #[cfg(feature = "aws")]
        {
            self.fetch_cpu_history(instance_id).await
        }

        #[cfg(not(feature = "aws"))]
        {
            let _ = instance_id;
            // Return mock data
            let now = Utc::now();
            (0..24)
                .map(|i| {
                    let time = now - chrono::Duration::hours(23 - i);
                    let value = 30.0 + (i as f64 * 2.0).sin() * 20.0;
                    (time, value)
                })
                .collect()
        }
    }

    /// Get ECS service metrics for a cluster
    pub async fn get_ecs_cluster_summary(&self, cluster_name: &str) -> Option<EcsClusterSummary> {
        #[cfg(feature = "aws")]
        {
            self.fetch_ecs_cluster_summary(cluster_name).await
        }

        #[cfg(not(feature = "aws"))]
        {
            // Return mock data
            Some(EcsClusterSummary {
                cluster_name: cluster_name.to_string(),
                running_tasks: 12,
                pending_tasks: 0,
                container_instances: 2,
                active_services: 15,
                avg_cpu: Some(35.0),
                avg_memory: Some(58.0),
            })
        }
    }

    #[cfg(feature = "aws")]
    async fn fetch_ec2_metrics(&self) -> Vec<Ec2Metrics> {
        use aws_sdk_ec2::types::Filter;

        let ec2_client = match &self.ec2_client {
            Some(c) => c,
            None => return vec![],
        };

        // Get all running EC2 instances
        let result = ec2_client
            .describe_instances()
            .filters(
                Filter::builder()
                    .name("instance-state-name")
                    .values("running")
                    .build(),
            )
            .send()
            .await;

        let mut metrics = Vec::new();

        if let Ok(response) = result {
            for reservation in response.reservations() {
                for instance in reservation.instances() {
                    let instance_id = instance.instance_id().unwrap_or_default().to_string();
                    let instance_name = instance
                        .tags()
                        .iter()
                        .find(|t| t.key() == Some("Name"))
                        .and_then(|t| t.value())
                        .unwrap_or(&instance_id)
                        .to_string();

                    let environment = instance
                        .tags()
                        .iter()
                        .find(|t| t.key() == Some("Environment"))
                        .and_then(|t| t.value())
                        .unwrap_or("unknown")
                        .to_string();

                    let state = instance
                        .state()
                        .and_then(|s| s.name())
                        .map(|n| n.as_str().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    // Get CPU metrics from CloudWatch
                    let (cpu_current, cpu_avg_1h, cpu_history) =
                        self.fetch_instance_cpu_metrics(&instance_id).await;

                    metrics.push(Ec2Metrics {
                        instance_id,
                        instance_name,
                        environment,
                        state,
                        cpu_current,
                        memory_current: None, // Memory requires CloudWatch agent
                        cpu_avg_1h,
                        cpu_history,
                    });
                }
            }
        }

        metrics
    }

    #[cfg(feature = "aws")]
    async fn fetch_instance_cpu_metrics(
        &self,
        instance_id: &str,
    ) -> (Option<f64>, Option<f64>, Vec<f64>) {
        use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};

        let cw_client = match &self.cloudwatch_client {
            Some(c) => c,
            None => return (None, None, vec![]),
        };

        let now = Utc::now();
        let start_time = now - chrono::Duration::hours(24);

        let dimension = Dimension::builder()
            .name("InstanceId")
            .value(instance_id)
            .build();

        let metric = Metric::builder()
            .namespace("AWS/EC2")
            .metric_name("CPUUtilization")
            .dimensions(dimension)
            .build();

        let metric_stat = MetricStat::builder()
            .metric(metric)
            .period(3600) // 1 hour
            .stat("Average")
            .build();

        let query = MetricDataQuery::builder()
            .id("cpu")
            .metric_stat(metric_stat)
            .return_data(true)
            .build();

        let result = cw_client
            .get_metric_data()
            .metric_data_queries(query)
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_secs(
                start_time.timestamp(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_secs(
                now.timestamp(),
            ))
            .send()
            .await;

        match result {
            Ok(response) => {
                let values: Vec<f64> = response
                    .metric_data_results()
                    .first()
                    .map(|r| r.values().to_vec())
                    .unwrap_or_default();

                let cpu_current = values.last().copied();
                let cpu_avg_1h = if values.len() >= 1 {
                    Some(values.iter().sum::<f64>() / values.len() as f64)
                } else {
                    None
                };

                (cpu_current, cpu_avg_1h, values)
            }
            Err(_) => (None, None, vec![]),
        }
    }

    #[cfg(feature = "aws")]
    async fn fetch_cpu_history(&self, instance_id: &str) -> Vec<(DateTime<Utc>, f64)> {
        let (_, _, values) = self.fetch_instance_cpu_metrics(instance_id).await;
        let now = Utc::now();

        values
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                let time = now - chrono::Duration::hours((values.len() - 1 - i) as i64);
                (time, v)
            })
            .collect()
    }

    #[cfg(feature = "aws")]
    async fn fetch_ecs_cluster_summary(&self, cluster_name: &str) -> Option<EcsClusterSummary> {
        // This would require aws-sdk-ecs which is already available
        // For now, return mock data
        Some(EcsClusterSummary {
            cluster_name: cluster_name.to_string(),
            running_tasks: 12,
            pending_tasks: 0,
            container_instances: 2,
            active_services: 15,
            avg_cpu: Some(35.0),
            avg_memory: Some(58.0),
        })
    }

    #[cfg(not(feature = "aws"))]
    fn mock_ec2_metrics(&self) -> Vec<Ec2Metrics> {
        vec![
            Ec2Metrics {
                instance_id: "i-ec2prod001".to_string(),
                instance_name: "ec2-prod".to_string(),
                environment: "EC2 Prod".to_string(),
                state: "running".to_string(),
                cpu_current: Some(45.0),
                memory_current: Some(62.0),
                cpu_avg_1h: Some(42.0),
                cpu_history: vec![
                    30.0, 35.0, 40.0, 45.0, 50.0, 55.0, 60.0, 55.0, 50.0, 45.0, 40.0, 35.0,
                    30.0, 35.0, 40.0, 45.0, 50.0, 55.0, 50.0, 45.0, 40.0, 42.0, 44.0, 45.0,
                ],
            },
            Ec2Metrics {
                instance_id: "i-ecsstage001".to_string(),
                instance_name: "optima-stage-asg".to_string(),
                environment: "ECS Stage".to_string(),
                state: "running".to_string(),
                cpu_current: Some(32.0),
                memory_current: Some(58.0),
                cpu_avg_1h: Some(30.0),
                cpu_history: vec![
                    25.0, 28.0, 30.0, 32.0, 35.0, 38.0, 40.0, 38.0, 35.0, 32.0, 30.0, 28.0,
                    25.0, 28.0, 30.0, 32.0, 35.0, 38.0, 35.0, 32.0, 30.0, 31.0, 32.0, 32.0,
                ],
            },
            Ec2Metrics {
                instance_id: "i-ecsprod001".to_string(),
                instance_name: "optima-prod-asg".to_string(),
                environment: "ECS Prod".to_string(),
                state: "running".to_string(),
                cpu_current: Some(28.0),
                memory_current: Some(45.0),
                cpu_avg_1h: Some(26.0),
                cpu_history: vec![
                    20.0, 22.0, 25.0, 28.0, 30.0, 32.0, 35.0, 32.0, 30.0, 28.0, 25.0, 22.0,
                    20.0, 22.0, 25.0, 28.0, 30.0, 32.0, 30.0, 28.0, 26.0, 27.0, 28.0, 28.0,
                ],
            },
            Ec2Metrics {
                instance_id: "i-shared001".to_string(),
                instance_name: "shared-services".to_string(),
                environment: "Shared".to_string(),
                state: "running".to_string(),
                cpu_current: Some(15.0),
                memory_current: Some(30.0),
                cpu_avg_1h: Some(12.0),
                cpu_history: vec![
                    10.0, 12.0, 14.0, 15.0, 16.0, 18.0, 20.0, 18.0, 16.0, 15.0, 14.0, 12.0,
                    10.0, 12.0, 14.0, 15.0, 16.0, 18.0, 16.0, 15.0, 14.0, 14.0, 15.0, 15.0,
                ],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_sparkline() {
        let values = vec![0.0, 25.0, 50.0, 75.0, 100.0];
        let sparkline = render_sparkline(&values);
        assert_eq!(sparkline.chars().count(), 5);
        assert!(sparkline.starts_with('▁'));
        assert!(sparkline.ends_with('█'));
    }

    #[test]
    fn test_render_sparkline_empty() {
        let values: Vec<f64> = vec![];
        let sparkline = render_sparkline(&values);
        assert!(sparkline.is_empty());
    }

    #[test]
    fn test_render_sparkline_constant() {
        let values = vec![50.0, 50.0, 50.0, 50.0];
        let sparkline = render_sparkline(&values);
        // All values same, should be middle bars
        assert_eq!(sparkline.chars().count(), 4);
    }
}

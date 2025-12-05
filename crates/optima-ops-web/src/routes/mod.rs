//! Route definitions for the web dashboard - Multi-page architecture

use crate::filters;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Form, Json, Router,
};
use askama::Template;
use serde::Deserialize;
use serde_json::json;
use optima_ops_core::{
    default_deployment_services, get_environment, get_status_class, get_status_text,
    DeploymentService, Environment, EnvironmentType, GitHubClient, InfraClient,
    MonitoringClient, ServiceCategory, ServiceDef, WorkflowRun,
};

use crate::state::AppState;

mod health;

pub use health::*;

/// Create the main router with all routes
pub fn create_router() -> Router<AppState> {
    Router::new()
        // Page routes
        .route("/", get(page_overview))
        .route("/env/ec2-prod", get(page_ec2_prod))
        .route("/env/ecs-stage", get(page_ecs_stage))
        .route("/env/ecs-prod", get(page_ecs_prod))
        .route("/env/shared", get(page_shared))
        .route("/github", get(page_github))
        // API routes
        .route("/health", get(health_check))
        .route("/api/health", get(api_health))
        .route("/api/containers/{name}/restart", post(api_container_restart))
        .route("/api/deployments/{service}/trigger", post(api_trigger_deployment))
        .route("/api/migrations/{task}/run", post(api_run_migration))
        // HTMX partial routes
        .route("/partials/overview/instances", get(partial_overview_instances))
        .route("/partials/ec2-prod/containers", get(partial_ec2_containers))
        .route("/partials/container-logs", get(partial_container_logs))
        .route("/partials/github/recent", get(partial_github_recent))
        // Legacy routes for backward compatibility
        .route("/partials/services", get(partial_services))
        .route("/partials/infrastructure", get(partial_infrastructure))
        .route("/partials/containers", get(partial_containers))
        .route("/partials/deployments", get(partial_deployments))
}

// ============== Page Templates ==============

/// Overview page template
#[derive(Template)]
#[template(path = "overview.html")]
struct OverviewTemplate {
    current_page: &'static str,
    ec2_prod_service_count: usize,
    ecs_stage_service_count: usize,
    ecs_prod_service_count: usize,
    shared_service_count: usize,
}

/// Overview page handler
async fn page_overview() -> impl IntoResponse {
    let ec2_prod = get_environment(EnvironmentType::Ec2Prod);
    let ecs_stage = get_environment(EnvironmentType::EcsStage);
    let ecs_prod = get_environment(EnvironmentType::EcsProd);
    let shared = get_environment(EnvironmentType::Shared);

    let template = OverviewTemplate {
        current_page: "overview",
        ec2_prod_service_count: ec2_prod.services.len(),
        ecs_stage_service_count: ecs_stage.services.len(),
        ecs_prod_service_count: ecs_prod.services.len(),
        shared_service_count: shared.services.len(),
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

/// EC2 Prod page template
#[derive(Template)]
#[template(path = "env_ec2_prod.html")]
struct Ec2ProdTemplate {
    current_page: &'static str,
    core_services: Vec<ServiceDef>,
    mcp_services: Vec<ServiceDef>,
}

/// EC2 Prod page handler
async fn page_ec2_prod() -> impl IntoResponse {
    let config = get_environment(EnvironmentType::Ec2Prod);

    let core_services: Vec<ServiceDef> = config
        .services
        .iter()
        .filter(|s| s.category == ServiceCategory::Core)
        .cloned()
        .collect();

    let mcp_services: Vec<ServiceDef> = config
        .services
        .iter()
        .filter(|s| s.category == ServiceCategory::McpTool)
        .cloned()
        .collect();

    let template = Ec2ProdTemplate {
        current_page: "ec2-prod",
        core_services,
        mcp_services,
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

/// ECS environment page template
#[derive(Template)]
#[template(path = "env_ecs.html")]
struct EcsEnvTemplate {
    current_page: String,
    env_type: String,
    env_display_name: String,
    cluster_name: Option<&'static str>,
    cluster_summary: Option<ClusterSummary>,
    core_services: Vec<ServiceDef>,
    mcp_services: Vec<ServiceDef>,
    bi_services: Vec<ServiceDef>,
    migration_tasks: Vec<ServiceDef>,
    scheduled_tasks: Vec<ServiceDef>,
}

struct ClusterSummary {
    running_tasks: i32,
    pending_tasks: i32,
    container_instances: i32,
    active_services: i32,
    avg_cpu: Option<f64>,
    avg_memory: Option<f64>,
}

/// ECS Stage page handler
async fn page_ecs_stage() -> impl IntoResponse {
    render_ecs_page(EnvironmentType::EcsStage).await
}

/// ECS Prod page handler
async fn page_ecs_prod() -> impl IntoResponse {
    render_ecs_page(EnvironmentType::EcsProd).await
}

async fn render_ecs_page(env_type: EnvironmentType) -> impl IntoResponse {
    let config = get_environment(env_type);

    let core_services: Vec<ServiceDef> = config
        .services
        .iter()
        .filter(|s| s.category == ServiceCategory::Core)
        .cloned()
        .collect();

    let mcp_services: Vec<ServiceDef> = config
        .services
        .iter()
        .filter(|s| s.category == ServiceCategory::McpTool)
        .cloned()
        .collect();

    let bi_services: Vec<ServiceDef> = config
        .services
        .iter()
        .filter(|s| s.category == ServiceCategory::BiService)
        .cloned()
        .collect();

    let migration_tasks: Vec<ServiceDef> = config
        .services
        .iter()
        .filter(|s| s.category == ServiceCategory::Migration)
        .cloned()
        .collect();

    let scheduled_tasks: Vec<ServiceDef> = config
        .services
        .iter()
        .filter(|s| s.category == ServiceCategory::Scheduled)
        .cloned()
        .collect();

    // Mock cluster summary for now
    let cluster_summary = Some(ClusterSummary {
        running_tasks: core_services.len() as i32 + mcp_services.len() as i32,
        pending_tasks: 0,
        container_instances: 2,
        active_services: core_services.len() as i32 + mcp_services.len() as i32 + bi_services.len() as i32,
        avg_cpu: Some(35.0),
        avg_memory: Some(58.0),
    });

    let template = EcsEnvTemplate {
        current_page: env_type.as_str().to_string(),
        env_type: env_type.as_str().to_string(),
        env_display_name: env_type.display_name().to_string(),
        cluster_name: config.cluster_name,
        cluster_summary,
        core_services,
        mcp_services,
        bi_services,
        migration_tasks,
        scheduled_tasks,
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

/// Shared services page template
#[derive(Template)]
#[template(path = "env_shared.html")]
struct SharedTemplate {
    current_page: &'static str,
    services: Vec<ServiceDef>,
}

/// Shared services page handler
async fn page_shared() -> impl IntoResponse {
    let config = get_environment(EnvironmentType::Shared);

    let template = SharedTemplate {
        current_page: "shared",
        services: config.services.clone(),
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

/// GitHub Actions page template
#[derive(Template)]
#[template(path = "github.html")]
struct GithubTemplate {
    current_page: &'static str,
    authenticated: bool,
    deployment_services: Vec<DeploymentTemplateInfo>,
}

struct DeploymentTemplateInfo {
    name: String,
    display_name: String,
    repo: String,
    workflow_url: String,
    latest_run: Option<RunInfo>,
}

/// GitHub Actions page handler
async fn page_github() -> impl IntoResponse {
    let client = get_github_client();
    let authenticated = client.is_authenticated();
    let services = default_deployment_services();

    let mut deployment_services = Vec::new();
    for service in &services {
        let latest_run = if authenticated {
            client
                .get_deployment_status(service)
                .await
                .ok()
                .and_then(|s| s.latest_run)
                .map(RunInfo::from)
        } else {
            None
        };

        deployment_services.push(DeploymentTemplateInfo {
            name: service.name.clone(),
            display_name: service.display_name.clone(),
            repo: service.repo.clone(),
            workflow_url: format!(
                "https://github.com/{}/actions/workflows/{}",
                service.repo, service.workflow_file
            ),
            latest_run,
        });
    }

    let template = GithubTemplate {
        current_page: "github",
        authenticated,
        deployment_services,
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

// ============== Partial Templates ==============

/// Overview instances partial template
#[derive(Template)]
#[template(path = "partials/overview_instances.html")]
struct OverviewInstancesTemplate {
    instances: Vec<optima_ops_core::Ec2Metrics>,
    last_updated: String,
}

/// HTMX partial: overview instances
async fn partial_overview_instances() -> impl IntoResponse {
    let monitoring = MonitoringClient::new("ap-southeast-1").await;
    let instances = monitoring.get_all_ec2_metrics().await;
    let last_updated = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let template = OverviewInstancesTemplate {
        instances,
        last_updated,
    };

    Html(template.render().unwrap_or_default())
}

// ============== Container Management ==============

/// Container info for templates
struct ContainerInfo {
    id: String,
    name: String,
    status: String,
    ports: String,
    status_class: String,
}

fn get_container_status_class(status: &str) -> String {
    let status_lower = status.to_lowercase();
    if status_lower.starts_with("up") {
        "bg-green-100 text-green-800".to_string()
    } else if status_lower.contains("exited") || status_lower.contains("dead") {
        "bg-red-100 text-red-800".to_string()
    } else if status_lower.contains("restarting") || status_lower.contains("paused") {
        "bg-yellow-100 text-yellow-800".to_string()
    } else {
        "bg-gray-100 text-gray-800".to_string()
    }
}

async fn fetch_containers_via_ssh(state: &AppState) -> Result<Vec<ContainerInfo>, String> {
    let mut guard = state.get_ssh_client().await?;
    let client = guard.as_mut().ok_or("SSH client not initialized")?;

    client.connect().await.map_err(|e| e.to_string())?;

    let result = client
        .get_container_status(None)
        .await
        .map_err(|e| e.to_string())?;

    if result.exit_code != 0 {
        return Err(format!("docker ps failed: {}", result.stderr));
    }

    let containers = optima_ops_core::parse_container_status(&result.stdout);

    Ok(containers
        .into_iter()
        .map(|c| ContainerInfo {
            status_class: get_container_status_class(&c.status),
            id: c.id,
            name: c.name,
            status: c.status,
            ports: c.ports,
        })
        .collect())
}

/// HTMX partial: EC2 Prod containers
async fn partial_ec2_containers(State(state): State<AppState>) -> impl IntoResponse {
    let last_updated = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    match fetch_containers_via_ssh(&state).await {
        Ok(containers) => {
            let template = ContainersTemplate {
                containers,
                error: None,
                last_updated,
                environment: "EC2 Prod".to_string(),
            };
            Html(template.render().unwrap_or_default())
        }
        Err(e) => {
            let template = ContainersTemplate {
                containers: Vec::new(),
                error: Some(e),
                last_updated,
                environment: "EC2 Prod".to_string(),
            };
            Html(template.render().unwrap_or_default())
        }
    }
}

/// Containers partial template
#[derive(Template)]
#[template(path = "partials/containers.html")]
struct ContainersTemplate {
    containers: Vec<ContainerInfo>,
    error: Option<String>,
    last_updated: String,
    environment: String,
}

/// HTMX partial: containers list (legacy)
async fn partial_containers(State(state): State<AppState>) -> impl IntoResponse {
    partial_ec2_containers(State(state)).await
}

/// Container logs query params
#[derive(Debug, Deserialize)]
struct ContainerLogsQuery {
    name: String,
    tail: Option<u32>,
    #[serde(default)]
    env: Option<String>,
}

/// Container logs partial template
#[derive(Template)]
#[template(path = "partials/container_logs.html")]
struct ContainerLogsTemplate {
    container_name: String,
    logs: String,
    tail: u32,
    error: Option<String>,
}

async fn fetch_container_logs_via_ssh(
    state: &AppState,
    container_name: &str,
    tail: u32,
) -> Result<String, String> {
    let mut guard = state.get_ssh_client().await?;
    let client = guard.as_mut().ok_or("SSH client not initialized")?;

    client.connect().await.map_err(|e| e.to_string())?;

    let result = client
        .get_container_logs(container_name, Some(tail), false)
        .await
        .map_err(|e| e.to_string())?;

    if result.exit_code != 0 {
        return Err(format!("docker logs failed: {}", result.stderr));
    }

    let logs = if result.stdout.is_empty() {
        result.stderr
    } else {
        result.stdout
    };

    Ok(logs)
}

async fn partial_container_logs(
    State(state): State<AppState>,
    Query(params): Query<ContainerLogsQuery>,
) -> impl IntoResponse {
    let tail = params.tail.unwrap_or(50);

    match fetch_container_logs_via_ssh(&state, &params.name, tail).await {
        Ok(logs) => {
            let template = ContainerLogsTemplate {
                container_name: params.name,
                logs,
                tail,
                error: None,
            };
            Html(template.render().unwrap_or_default())
        }
        Err(e) => {
            let template = ContainerLogsTemplate {
                container_name: params.name,
                logs: String::new(),
                tail,
                error: Some(e),
            };
            Html(template.render().unwrap_or_default())
        }
    }
}

/// API endpoint: restart container
async fn api_container_restart(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut guard = match state.get_ssh_client().await {
        Ok(g) => g,
        Err(e) => return Json(json!({ "success": false, "error": e })),
    };

    let client = match guard.as_mut() {
        Some(c) => c,
        None => return Json(json!({ "success": false, "error": "SSH client not initialized" })),
    };

    if let Err(e) = client.connect().await {
        return Json(json!({ "success": false, "error": e.to_string() }));
    }

    match client.docker_command(&format!("restart {}", name)).await {
        Ok(result) => {
            if result.exit_code == 0 {
                Json(json!({
                    "success": true,
                    "message": format!("Container {} restarted successfully", name),
                }))
            } else {
                Json(json!({
                    "success": false,
                    "error": format!("docker restart failed: {}", result.stderr)
                }))
            }
        }
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

// ============== Deployment Management ==============

struct RunInfo {
    id: i64,
    status: String,
    status_text: String,
    status_class: String,
    conclusion: Option<String>,
    html_url: String,
    created_at: String,
    created_date: String,
    actor: String,
    event: String,
    display_title: String,
}

impl From<WorkflowRun> for RunInfo {
    fn from(run: WorkflowRun) -> Self {
        let status_class = get_status_class(&run.status, run.conclusion.as_deref());
        let status_text = get_status_text(&run.status, run.conclusion.as_deref());
        let created_date = run
            .created_at
            .split('T')
            .next()
            .unwrap_or(&run.created_at)
            .to_string();

        RunInfo {
            id: run.id,
            status: run.status,
            status_text: status_text.to_string(),
            status_class: status_class.to_string(),
            conclusion: run.conclusion,
            html_url: run.html_url,
            created_at: run.created_at,
            created_date,
            actor: run.actor.login,
            event: run.event,
            display_title: run.display_title.unwrap_or_else(|| run.name),
        }
    }
}

fn get_github_client() -> GitHubClient {
    GitHubClient::new(None)
}

/// Recent deployments partial for GitHub page
#[derive(Template)]
#[template(path = "partials/github_recent.html")]
struct GithubRecentTemplate {
    deployments: Vec<RecentDeployment>,
    last_updated: String,
}

struct RecentDeployment {
    service_name: String,
    display_name: String,
    workflow: String,
    status_text: String,
    status_class: String,
    time_ago: String,
    html_url: String,
}

async fn partial_github_recent() -> impl IntoResponse {
    let client = get_github_client();
    let services = default_deployment_services();
    let last_updated = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let mut deployments = Vec::new();

    for service in &services {
        if let Ok(status) = client.get_deployment_status(service).await {
            if let Some(run) = status.latest_run {
                let run_info = RunInfo::from(run);
                deployments.push(RecentDeployment {
                    service_name: service.name.clone(),
                    display_name: service.display_name.clone(),
                    workflow: service.workflow_file.replace(".yml", ""),
                    status_text: run_info.status_text,
                    status_class: run_info.status_class,
                    time_ago: run_info.created_date,
                    html_url: run_info.html_url,
                });
            }
        }
    }

    let template = GithubRecentTemplate {
        deployments,
        last_updated,
    };

    Html(template.render().unwrap_or_default())
}

/// Trigger deployment request
#[derive(Debug, Deserialize)]
struct TriggerDeploymentForm {
    environment: Option<String>,
}

async fn api_trigger_deployment(
    Path(service_name): Path<String>,
    Form(form): Form<TriggerDeploymentForm>,
) -> impl IntoResponse {
    let client = get_github_client();

    if !client.is_authenticated() {
        return Json(json!({
            "success": false,
            "error": "GitHub token not configured"
        }));
    }

    let services = default_deployment_services();
    let service = services.iter().find(|s| s.name == service_name);

    let service = match service {
        Some(s) => s,
        None => {
            return Json(json!({
                "success": false,
                "error": format!("Service '{}' not found", service_name)
            }))
        }
    };

    let parts: Vec<&str> = service.repo.split('/').collect();
    if parts.len() != 2 {
        return Json(json!({
            "success": false,
            "error": format!("Invalid repo format: {}", service.repo)
        }));
    }
    let (owner, repo) = (parts[0], parts[1]);

    let environment = form.environment.unwrap_or_else(|| "stage".to_string());
    let inputs = json!({ "environment": environment });

    match client
        .trigger_workflow(owner, repo, &service.workflow_file, "main", Some(inputs))
        .await
    {
        Ok(_) => Json(json!({
            "success": true,
            "message": format!("Deployment triggered for {} ({})", service.display_name, environment)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

/// Migration run request
#[derive(Debug, Deserialize)]
struct MigrationRunQuery {
    env: Option<String>,
}

async fn api_run_migration(
    Path(task): Path<String>,
    Query(query): Query<MigrationRunQuery>,
) -> impl IntoResponse {
    // TODO: Implement ECS RunTask for migrations
    let env = query.env.unwrap_or_else(|| "stage".to_string());
    Json(json!({
        "success": false,
        "error": format!("Migration {} for {} not yet implemented", task, env)
    }))
}

// ============== Legacy Routes ==============

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn api_health(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config();
    Json(json!({
        "status": "healthy",
        "environment": config.get_environment().as_str(),
        "service_count": config.get_all_services().len()
    }))
}

/// Service card partial template for HTMX
#[derive(Template)]
#[template(path = "partials/service_card.html")]
struct ServiceCardTemplate {
    name: String,
    service_type: String,
    health_endpoint: String,
    status: String,
    status_class: String,
    response_time: Option<u64>,
}

async fn partial_services(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config();
    let checker = HealthChecker::new();
    let services = config.get_all_services();

    let mut cards = Vec::new();
    for service in &services {
        let result = checker.check(&service.name, &service.health_endpoint).await;
        let (status, status_class) = match result.status {
            HealthStatus::Healthy => (
                "Healthy".to_string(),
                "bg-green-100 text-green-800".to_string(),
            ),
            HealthStatus::Unhealthy => (
                "Unhealthy".to_string(),
                "bg-red-100 text-red-800".to_string(),
            ),
            HealthStatus::Unknown => (
                "Unknown".to_string(),
                "bg-gray-100 text-gray-800".to_string(),
            ),
        };

        cards.push(ServiceCardTemplate {
            name: service.name.clone(),
            service_type: format!("{:?}", service.service_type),
            health_endpoint: service.health_endpoint.clone(),
            status,
            status_class,
            response_time: result.response_time_ms,
        });
    }

    let mut html = String::new();
    for card in cards {
        html.push_str(&card.render().unwrap_or_default());
    }

    Html(html)
}

/// Infrastructure partial template
#[derive(Template)]
#[template(path = "partials/infrastructure.html")]
struct InfrastructureTemplate {
    ec2_instances: Vec<Ec2Info>,
    ecs_services: Vec<EcsInfo>,
    rds_instances: Vec<RdsInfo>,
    last_updated: String,
}

struct Ec2Info {
    instance_id: String,
    name: String,
    state: String,
    instance_type: String,
    private_ip: String,
    state_class: String,
}

struct EcsInfo {
    service_name: String,
    cluster: String,
    running_count: i32,
    desired_count: i32,
    status: String,
    status_class: String,
}

struct RdsInfo {
    identifier: String,
    engine: String,
    status: String,
    instance_class: String,
    status_class: String,
}

async fn partial_infrastructure(State(_state): State<AppState>) -> impl IntoResponse {
    let client = InfraClient::new("ap-southeast-1");
    let status = client.get_status().await;

    let ec2_instances: Vec<Ec2Info> = status
        .ec2_instances
        .into_iter()
        .map(|ec2| {
            let state_class = match ec2.state.as_str() {
                "running" => "bg-green-100 text-green-800".to_string(),
                "stopped" => "bg-red-100 text-red-800".to_string(),
                _ => "bg-gray-100 text-gray-800".to_string(),
            };
            Ec2Info {
                instance_id: ec2.instance_id,
                name: ec2.name,
                state: ec2.state,
                instance_type: ec2.instance_type,
                private_ip: ec2.private_ip.unwrap_or_else(|| "-".to_string()),
                state_class,
            }
        })
        .collect();

    let ecs_services: Vec<EcsInfo> = status
        .ecs_services
        .into_iter()
        .map(|svc| {
            let status_class = if svc.running_count == svc.desired_count && svc.running_count > 0 {
                "bg-green-100 text-green-800".to_string()
            } else if svc.running_count == 0 {
                "bg-red-100 text-red-800".to_string()
            } else {
                "bg-yellow-100 text-yellow-800".to_string()
            };
            EcsInfo {
                service_name: svc.service_name,
                cluster: svc.cluster,
                running_count: svc.running_count,
                desired_count: svc.desired_count,
                status: svc.status,
                status_class,
            }
        })
        .collect();

    let rds_instances: Vec<RdsInfo> = status
        .rds_instances
        .into_iter()
        .map(|rds| {
            let status_class = match rds.status.as_str() {
                "available" => "bg-green-100 text-green-800".to_string(),
                "stopped" => "bg-red-100 text-red-800".to_string(),
                _ => "bg-yellow-100 text-yellow-800".to_string(),
            };
            RdsInfo {
                identifier: rds.identifier,
                engine: rds.engine,
                status: rds.status,
                instance_class: rds.instance_class,
                status_class,
            }
        })
        .collect();

    let template = InfrastructureTemplate {
        ec2_instances,
        ecs_services,
        rds_instances,
        last_updated: status.last_updated.unwrap_or_else(|| "-".to_string()),
    };

    Html(template.render().unwrap_or_default())
}

/// Deployment info for templates
struct DeploymentInfo {
    name: String,
    display_name: String,
    repo: String,
    workflow_file: String,
    repo_url: String,
    workflow_url: String,
    latest_run: Option<RunInfo>,
    recent_runs: Vec<RunInfo>,
}

/// Deployments partial template
#[derive(Template)]
#[template(path = "partials/deployments.html")]
struct DeploymentsTemplate {
    deployments: Vec<DeploymentInfo>,
    authenticated: bool,
    error: Option<String>,
    last_updated: String,
}

async fn partial_deployments() -> impl IntoResponse {
    let client = get_github_client();
    let services = default_deployment_services();
    let authenticated = client.is_authenticated();
    let last_updated = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let mut deployments = Vec::new();
    let mut global_error: Option<String> = None;

    for service in &services {
        match client.get_deployment_status(service).await {
            Ok(status) => {
                let latest_run = status.latest_run.map(RunInfo::from);
                let recent_runs: Vec<RunInfo> = status
                    .recent_runs
                    .into_iter()
                    .skip(1)
                    .take(4)
                    .map(RunInfo::from)
                    .collect();

                deployments.push(DeploymentInfo {
                    name: service.name.clone(),
                    display_name: service.display_name.clone(),
                    repo: service.repo.clone(),
                    workflow_file: service.workflow_file.clone(),
                    repo_url: status.repo_url,
                    workflow_url: status.workflow_url,
                    latest_run,
                    recent_runs,
                });
            }
            Err(e) => {
                deployments.push(DeploymentInfo {
                    name: service.name.clone(),
                    display_name: service.display_name.clone(),
                    repo: service.repo.clone(),
                    workflow_file: service.workflow_file.clone(),
                    repo_url: format!("https://github.com/{}", service.repo),
                    workflow_url: format!(
                        "https://github.com/{}/actions/workflows/{}",
                        service.repo, service.workflow_file
                    ),
                    latest_run: None,
                    recent_runs: Vec::new(),
                });

                if global_error.is_none() {
                    global_error = Some(e.to_string());
                }
            }
        }
    }

    let template = DeploymentsTemplate {
        deployments,
        authenticated,
        error: global_error,
        last_updated,
    };

    Html(template.render().unwrap_or_default())
}

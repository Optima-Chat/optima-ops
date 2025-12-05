//! Route definitions for the web dashboard

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
    Json,
    Form,
};
use askama::Template;
use serde::Deserialize;
use serde_json::json;
use optima_ops_core::{
    default_deployment_services, get_status_class, get_status_text,
    DeploymentService, Environment, GitHubClient, InfraClient, WorkflowRun,
};

use crate::state::AppState;

mod health;

pub use health::*;

/// Create the main router with all routes
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/health", get(health_check))
        .route("/api/health", get(api_health))
        .route("/api/services/status", get(api_services_status))
        .route("/api/environment", post(set_environment))
        .route("/api/environment", get(get_environment))
        .route("/api/infrastructure", get(api_infrastructure))
        .route("/api/containers", get(api_containers))
        .route("/api/containers/{name}/restart", post(api_container_restart))
        // Deployment routes
        .route("/api/deployments", get(api_deployments))
        .route("/api/deployments/{service}/trigger", post(api_trigger_deployment))
        .route("/partials/services", get(partial_services))
        .route("/partials/header", get(partial_header))
        .route("/partials/infrastructure", get(partial_infrastructure))
        .route("/partials/containers", get(partial_containers))
        .route("/partials/container-logs", get(partial_container_logs))
        .route("/partials/deployments", get(partial_deployments))
}

/// Dashboard template
#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    title: String,
    environment: String,
    services: Vec<ServiceInfo>,
}

struct ServiceInfo {
    name: String,
    health_endpoint: String,
    service_type: String,
}

/// Main dashboard page
async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config();
    let services: Vec<ServiceInfo> = config
        .get_all_services()
        .into_iter()
        .map(|s| ServiceInfo {
            name: s.name.clone(),
            health_endpoint: s.health_endpoint.clone(),
            service_type: format!("{:?}", s.service_type),
        })
        .collect();

    let template = DashboardTemplate {
        title: "Optima Ops Dashboard".to_string(),
        environment: config.get_environment().to_string(),
        services,
    };

    Html(template.render().unwrap_or_else(|e| {
        format!("Template error: {}", e)
    }))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// API health endpoint (returns JSON)
async fn api_health(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config();

    Json(json!({
        "status": "healthy",
        "environment": config.get_environment().as_str(),
        "service_count": config.get_all_services().len()
    }))
}

/// API endpoint to get all services status (JSON)
async fn api_services_status(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config();
    let checker = HealthChecker::new();
    let services = config.get_all_services();

    let mut results = Vec::new();
    for service in &services {
        let result = checker.check(&service.name, &service.health_endpoint).await;
        results.push(result);
    }

    Json(results)
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

/// HTMX partial: services grid with actual health status
async fn partial_services(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config();
    let checker = HealthChecker::new();
    let services = config.get_all_services();

    let mut cards = Vec::new();
    for service in &services {
        let result = checker.check(&service.name, &service.health_endpoint).await;
        let (status, status_class) = match result.status {
            HealthStatus::Healthy => ("Healthy".to_string(), "bg-green-100 text-green-800".to_string()),
            HealthStatus::Unhealthy => ("Unhealthy".to_string(), "bg-red-100 text-red-800".to_string()),
            HealthStatus::Unknown => ("Unknown".to_string(), "bg-gray-100 text-gray-800".to_string()),
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

    // Render all cards as HTML
    let mut html = String::new();
    for card in cards {
        html.push_str(&card.render().unwrap_or_default());
    }

    Html(html)
}

/// Environment change request
#[derive(Debug, Deserialize)]
struct EnvironmentForm {
    environment: String,
}

/// Set environment endpoint (POST)
async fn set_environment(
    State(state): State<AppState>,
    Form(form): Form<EnvironmentForm>,
) -> impl IntoResponse {
    let env = match form.environment.as_str() {
        "production" => Environment::Production,
        "shared" => Environment::Shared,
        _ => Environment::Production,
    };

    state.set_environment(env);

    // Return HX-Redirect header to reload the page
    (
        [("HX-Redirect", "/")],
        Html(format!("Environment set to {}", form.environment))
    )
}

/// Get current environment (JSON)
async fn get_environment(State(state): State<AppState>) -> impl IntoResponse {
    let env = state.current_environment();
    Json(json!({
        "environment": env.as_str(),
        "display_name": env.to_string()
    }))
}

/// Header partial template for HTMX updates
#[derive(Template)]
#[template(path = "partials/header.html")]
struct HeaderTemplate {
    environment: String,
    environments: Vec<(String, String, bool)>,
}

/// HTMX partial: header with environment selector
async fn partial_header(State(state): State<AppState>) -> impl IntoResponse {
    let current_env = state.current_environment();
    let environments: Vec<(String, String, bool)> = AppState::available_environments()
        .into_iter()
        .map(|(value, label)| {
            let is_selected = current_env.as_str() == value;
            (value.to_string(), label.to_string(), is_selected)
        })
        .collect();

    let template = HeaderTemplate {
        environment: current_env.to_string(),
        environments,
    };

    Html(template.render().unwrap_or_default())
}

/// API endpoint to get infrastructure status (JSON)
async fn api_infrastructure(State(_state): State<AppState>) -> impl IntoResponse {
    let client = InfraClient::new("ap-southeast-1");
    let status = client.get_status().await;
    Json(status)
}

/// Infrastructure partial template for HTMX
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

/// HTMX partial: infrastructure status grid
async fn partial_infrastructure(State(_state): State<AppState>) -> impl IntoResponse {
    let client = InfraClient::new("ap-southeast-1");
    let status = client.get_status().await;

    let ec2_instances: Vec<Ec2Info> = status.ec2_instances.into_iter().map(|ec2| {
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
    }).collect();

    let ecs_services: Vec<EcsInfo> = status.ecs_services.into_iter().map(|svc| {
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
    }).collect();

    let rds_instances: Vec<RdsInfo> = status.rds_instances.into_iter().map(|rds| {
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
    }).collect();

    let template = InfrastructureTemplate {
        ec2_instances,
        ecs_services,
        rds_instances,
        last_updated: status.last_updated.unwrap_or_else(|| "-".to_string()),
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

/// Get status class based on container status string
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

/// Fetch real container data via SSH
async fn fetch_containers_via_ssh(state: &AppState) -> Result<Vec<ContainerInfo>, String> {
    let mut guard = state.get_ssh_client().await?;
    let client = guard.as_mut().ok_or("SSH client not initialized")?;

    // Connect if not already connected
    client.connect().await.map_err(|e| e.to_string())?;

    // Get container status
    let result = client.get_container_status(None).await.map_err(|e| e.to_string())?;

    if result.exit_code != 0 {
        return Err(format!("docker ps failed: {}", result.stderr));
    }

    // Parse the output
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

/// API endpoint: list containers (JSON)
async fn api_containers(State(state): State<AppState>) -> impl IntoResponse {
    match fetch_containers_via_ssh(&state).await {
        Ok(containers) => {
            let json_containers: Vec<serde_json::Value> = containers
                .into_iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "name": c.name,
                        "status": c.status,
                        "ports": c.ports
                    })
                })
                .collect();
            Json(json!({ "success": true, "containers": json_containers }))
        }
        Err(e) => {
            Json(json!({ "success": false, "error": e }))
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

    // Connect if not already connected
    if let Err(e) = client.connect().await {
        return Json(json!({ "success": false, "error": e.to_string() }));
    }

    // Execute docker restart command
    match client.docker_command(&format!("restart {}", name)).await {
        Ok(result) => {
            if result.exit_code == 0 {
                Json(json!({
                    "success": true,
                    "message": format!("Container {} restarted successfully", name),
                    "execution_time_ms": result.execution_time.as_millis()
                }))
            } else {
                Json(json!({
                    "success": false,
                    "error": format!("docker restart failed: {}", result.stderr)
                }))
            }
        }
        Err(e) => {
            Json(json!({ "success": false, "error": e.to_string() }))
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

/// HTMX partial: containers list
async fn partial_containers(State(state): State<AppState>) -> impl IntoResponse {
    let env = state.current_environment();
    let last_updated = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    match fetch_containers_via_ssh(&state).await {
        Ok(containers) => {
            let template = ContainersTemplate {
                containers,
                error: None,
                last_updated,
                environment: env.to_string(),
            };
            Html(template.render().unwrap_or_default())
        }
        Err(e) => {
            let template = ContainersTemplate {
                containers: Vec::new(),
                error: Some(e),
                last_updated,
                environment: env.to_string(),
            };
            Html(template.render().unwrap_or_default())
        }
    }
}

/// Container logs query params
#[derive(Debug, Deserialize)]
struct ContainerLogsQuery {
    name: String,
    tail: Option<u32>,
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

/// Fetch container logs via SSH
async fn fetch_container_logs_via_ssh(
    state: &AppState,
    container_name: &str,
    tail: u32,
) -> Result<String, String> {
    let mut guard = state.get_ssh_client().await?;
    let client = guard.as_mut().ok_or("SSH client not initialized")?;

    // Connect if not already connected
    client.connect().await.map_err(|e| e.to_string())?;

    // Get container logs
    let result = client
        .get_container_logs(container_name, Some(tail), false)
        .await
        .map_err(|e| e.to_string())?;

    if result.exit_code != 0 {
        return Err(format!("docker logs failed: {}", result.stderr));
    }

    // Combine stdout and stderr (logs can be in either)
    let logs = if result.stdout.is_empty() {
        result.stderr
    } else {
        result.stdout
    };

    Ok(logs)
}

/// HTMX partial: container logs
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

// ============== Deployment Management ==============

/// Deployment service info for templates
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

/// Workflow run info for templates
struct RunInfo {
    id: i64,
    status: String,
    status_text: String,
    status_class: String,
    conclusion: Option<String>,
    html_url: String,
    created_at: String,
    created_date: String,  // Just the date part for display
    actor: String,
    event: String,
    display_title: String,
}

impl From<WorkflowRun> for RunInfo {
    fn from(run: WorkflowRun) -> Self {
        let status_class = get_status_class(&run.status, run.conclusion.as_deref());
        let status_text = get_status_text(&run.status, run.conclusion.as_deref());

        // Extract just the date part (YYYY-MM-DD) from ISO timestamp
        let created_date = run.created_at.split('T').next()
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

/// Get GitHub client from environment
fn get_github_client() -> GitHubClient {
    GitHubClient::new(None) // Will use GITHUB_TOKEN env var
}

/// API endpoint: get all deployment statuses
async fn api_deployments() -> impl IntoResponse {
    let client = get_github_client();
    let services = default_deployment_services();
    let mut results = Vec::new();

    for service in &services {
        let status = client.get_deployment_status(service).await;
        match status {
            Ok(s) => {
                results.push(json!({
                    "service": s.service.name,
                    "display_name": s.service.display_name,
                    "repo": s.service.repo,
                    "workflow_url": s.workflow_url,
                    "repo_url": s.repo_url,
                    "latest_run": s.latest_run,
                    "recent_runs": s.recent_runs,
                }));
            }
            Err(e) => {
                results.push(json!({
                    "service": service.name,
                    "display_name": service.display_name,
                    "error": e.to_string(),
                }));
            }
        }
    }

    Json(json!({
        "success": true,
        "authenticated": client.is_authenticated(),
        "deployments": results
    }))
}

/// Trigger deployment request
#[derive(Debug, Deserialize)]
struct TriggerDeploymentForm {
    environment: Option<String>,
}

/// API endpoint: trigger deployment for a service
async fn api_trigger_deployment(
    Path(service_name): Path<String>,
    Form(form): Form<TriggerDeploymentForm>,
) -> impl IntoResponse {
    let client = get_github_client();

    if !client.is_authenticated() {
        return Json(json!({
            "success": false,
            "error": "GitHub token not configured. Set GITHUB_TOKEN environment variable."
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
            }));
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

    // Build inputs
    let environment = form.environment.unwrap_or_else(|| "stage".to_string());
    let inputs = json!({
        "environment": environment
    });

    match client
        .trigger_workflow(owner, repo, &service.workflow_file, "main", Some(inputs))
        .await
    {
        Ok(_) => Json(json!({
            "success": true,
            "message": format!("Deployment triggered for {} ({})", service.display_name, environment),
            "workflow_url": format!("https://github.com/{}/actions/workflows/{}", service.repo, service.workflow_file)
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        })),
    }
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

/// HTMX partial: deployments grid
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
                    .skip(1) // Skip first since it's already in latest_run
                    .take(4) // Show up to 4 more runs
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
                // Still add the service but with no run info
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

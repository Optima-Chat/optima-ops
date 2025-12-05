//! Route definitions for the web dashboard

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
    Json,
    Form,
};
use askama::Template;
use serde::Deserialize;
use serde_json::json;
use optima_ops_core::{Environment, InfraClient};

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
        .route("/partials/services", get(partial_services))
        .route("/partials/header", get(partial_header))
        .route("/partials/infrastructure", get(partial_infrastructure))
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
        "stage" => Environment::Stage,
        "shared" => Environment::Shared,
        "development" => Environment::Development,
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

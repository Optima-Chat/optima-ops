//! GitHub Actions API client for deployment management
//!
//! 功能:
//! - 获取 workflow 运行状态
//! - 触发 workflow_dispatch 部署
//! - 解析 workflow inputs

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

const GITHUB_API_BASE: &str = "https://api.github.com";

/// GitHub Actions client
pub struct GitHubClient {
    client: reqwest::Client,
    token: Option<String>,
}

/// Workflow run status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: i64,
    pub name: String,
    pub head_branch: String,
    pub head_sha: String,
    pub status: String,        // queued, in_progress, completed
    pub conclusion: Option<String>, // success, failure, cancelled, skipped
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub run_started_at: Option<String>,
    pub actor: Actor,
    pub triggering_actor: Option<Actor>,
    pub event: String,         // push, workflow_dispatch, schedule
    pub display_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub login: String,
    pub avatar_url: String,
}

/// Workflow runs response
#[derive(Debug, Deserialize)]
struct WorkflowRunsResponse {
    total_count: i32,
    workflow_runs: Vec<WorkflowRun>,
}

/// Workflow info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub state: String,
    pub html_url: String,
}

/// Workflows list response
#[derive(Debug, Deserialize)]
struct WorkflowsResponse {
    total_count: i32,
    workflows: Vec<Workflow>,
}

/// Workflow dispatch input definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<String>,
    #[serde(rename = "type")]
    pub input_type: Option<String>,  // string, boolean, choice
    pub options: Option<Vec<String>>, // for choice type
}

/// Service deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentService {
    pub name: String,
    pub display_name: String,
    pub repo: String,            // e.g., "Optima-Chat/user-auth"
    pub workflow_file: String,   // e.g., "deploy-ecs.yml"
    pub default_inputs: Option<serde_json::Value>,
}

/// Deployment status summary for a service
#[derive(Debug, Clone, Serialize)]
pub struct DeploymentStatus {
    pub service: DeploymentService,
    pub latest_run: Option<WorkflowRun>,
    pub recent_runs: Vec<WorkflowRun>,
    pub workflow_url: String,
    pub repo_url: String,
}

impl GitHubClient {
    /// Create a new GitHub client
    ///
    /// Token can be:
    /// - Personal Access Token (PAT) with `repo` and `workflow` scopes
    /// - Environment variable `GITHUB_TOKEN`
    pub fn new(token: Option<String>) -> Self {
        let token = token.or_else(|| std::env::var("GITHUB_TOKEN").ok());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, token }
    }

    /// Build request headers
    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("optima-ops-dashboard"));
        headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));

        if let Some(ref token) = self.token {
            if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                headers.insert(AUTHORIZATION, value);
            }
        }

        headers
    }

    /// Check if client has authentication
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Get recent workflow runs for a repository
    ///
    /// # Arguments
    /// * `owner` - Repository owner (e.g., "Optima-Chat")
    /// * `repo` - Repository name (e.g., "user-auth")
    /// * `workflow_id` - Workflow file name (e.g., "deploy-ecs.yml") or workflow ID
    /// * `per_page` - Number of results (max 100)
    pub async fn get_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        workflow_id: &str,
        per_page: u8,
    ) -> Result<Vec<WorkflowRun>> {
        let url = format!(
            "{}/repos/{}/{}/actions/workflows/{}/runs?per_page={}",
            GITHUB_API_BASE, owner, repo, workflow_id, per_page
        );

        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to fetch workflow runs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error ({}): {}", status, body);
        }

        let data: WorkflowRunsResponse = response
            .json()
            .await
            .context("Failed to parse workflow runs response")?;

        Ok(data.workflow_runs)
    }

    /// Get all workflow runs for a repository (not filtered by workflow)
    pub async fn get_all_runs(
        &self,
        owner: &str,
        repo: &str,
        per_page: u8,
    ) -> Result<Vec<WorkflowRun>> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs?per_page={}",
            GITHUB_API_BASE, owner, repo, per_page
        );

        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to fetch workflow runs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error ({}): {}", status, body);
        }

        let data: WorkflowRunsResponse = response
            .json()
            .await
            .context("Failed to parse workflow runs response")?;

        Ok(data.workflow_runs)
    }

    /// List workflows in a repository
    pub async fn list_workflows(&self, owner: &str, repo: &str) -> Result<Vec<Workflow>> {
        let url = format!(
            "{}/repos/{}/{}/actions/workflows",
            GITHUB_API_BASE, owner, repo
        );

        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to fetch workflows")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error ({}): {}", status, body);
        }

        let data: WorkflowsResponse = response
            .json()
            .await
            .context("Failed to parse workflows response")?;

        Ok(data.workflows)
    }

    /// Trigger a workflow dispatch event
    ///
    /// # Arguments
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `workflow_id` - Workflow file name or ID
    /// * `git_ref` - Branch or tag to run on (e.g., "main")
    /// * `inputs` - Optional workflow inputs
    pub async fn trigger_workflow(
        &self,
        owner: &str,
        repo: &str,
        workflow_id: &str,
        git_ref: &str,
        inputs: Option<serde_json::Value>,
    ) -> Result<()> {
        if !self.is_authenticated() {
            anyhow::bail!("GitHub token required to trigger workflows");
        }

        let url = format!(
            "{}/repos/{}/{}/actions/workflows/{}/dispatches",
            GITHUB_API_BASE, owner, repo, workflow_id
        );

        let mut body = serde_json::json!({
            "ref": git_ref
        });

        if let Some(inputs) = inputs {
            body["inputs"] = inputs;
        }

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to trigger workflow")?;

        // 204 No Content = success
        if response.status().as_u16() == 204 {
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to trigger workflow ({}): {}", status, body);
        }

        Ok(())
    }

    /// Get deployment status for a service
    pub async fn get_deployment_status(
        &self,
        service: &DeploymentService,
    ) -> Result<DeploymentStatus> {
        let parts: Vec<&str> = service.repo.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid repo format: {}", service.repo);
        }
        let (owner, repo) = (parts[0], parts[1]);

        let runs = self
            .get_workflow_runs(owner, repo, &service.workflow_file, 5)
            .await
            .unwrap_or_default();

        let latest_run = runs.first().cloned();

        Ok(DeploymentStatus {
            service: service.clone(),
            latest_run,
            recent_runs: runs,
            workflow_url: format!(
                "https://github.com/{}/actions/workflows/{}",
                service.repo, service.workflow_file
            ),
            repo_url: format!("https://github.com/{}", service.repo),
        })
    }
}

/// Get status badge class for workflow conclusion
pub fn get_status_class(status: &str, conclusion: Option<&str>) -> &'static str {
    match (status, conclusion) {
        ("completed", Some("success")) => "bg-green-100 text-green-800",
        ("completed", Some("failure")) => "bg-red-100 text-red-800",
        ("completed", Some("cancelled")) => "bg-gray-100 text-gray-800",
        ("in_progress", _) => "bg-yellow-100 text-yellow-800",
        ("queued", _) => "bg-blue-100 text-blue-800",
        _ => "bg-gray-100 text-gray-800",
    }
}

/// Get status display text
pub fn get_status_text(status: &str, conclusion: Option<&str>) -> &'static str {
    match (status, conclusion) {
        ("completed", Some("success")) => "成功",
        ("completed", Some("failure")) => "失败",
        ("completed", Some("cancelled")) => "已取消",
        ("completed", Some("skipped")) => "跳过",
        ("in_progress", _) => "运行中",
        ("queued", _) => "排队中",
        ("waiting", _) => "等待中",
        _ => "未知",
    }
}

/// Default deployment services configuration
pub fn default_deployment_services() -> Vec<DeploymentService> {
    vec![
        // Core Services
        DeploymentService {
            name: "user-auth".to_string(),
            display_name: "User Auth".to_string(),
            repo: "Optima-Chat/user-auth".to_string(),
            workflow_file: "deploy-ecs.yml".to_string(),
            default_inputs: Some(serde_json::json!({
                "environment": "stage"
            })),
        },
        DeploymentService {
            name: "mcp-host".to_string(),
            display_name: "MCP Host".to_string(),
            repo: "Optima-Chat/mcp-host".to_string(),
            workflow_file: "deploy-ecs.yml".to_string(),
            default_inputs: Some(serde_json::json!({
                "environment": "stage"
            })),
        },
        DeploymentService {
            name: "commerce-backend".to_string(),
            display_name: "Commerce Backend".to_string(),
            repo: "Optima-Chat/commerce-backend".to_string(),
            workflow_file: "deploy-ecs.yml".to_string(),
            default_inputs: Some(serde_json::json!({
                "environment": "stage"
            })),
        },
        DeploymentService {
            name: "agentic-chat".to_string(),
            display_name: "Agentic Chat".to_string(),
            repo: "Optima-Chat/agentic-chat".to_string(),
            workflow_file: "deploy-ecs.yml".to_string(),
            default_inputs: Some(serde_json::json!({
                "environment": "stage"
            })),
        },
        // AI Tools
        DeploymentService {
            name: "optima-bi".to_string(),
            display_name: "Optima BI".to_string(),
            repo: "Optima-Chat/optima-bi".to_string(),
            workflow_file: "deploy-ecs.yml".to_string(),
            default_inputs: Some(serde_json::json!({
                "environment": "stage"
            })),
        },
        DeploymentService {
            name: "optima-ai-shell".to_string(),
            display_name: "AI Shell".to_string(),
            repo: "Optima-Chat/optima-ai-shell".to_string(),
            workflow_file: "deploy-ecs.yml".to_string(),
            default_inputs: Some(serde_json::json!({
                "environment": "stage"
            })),
        },
    ]
}

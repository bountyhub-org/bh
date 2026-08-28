use std::sync::Arc;
use std::time::Duration;

use connectrpc::ConnectError;
use connectrpc::client::{ClientConfig, HttpClient};

use crate::connect::bountyhub::project::v1::{
    ActivateProjectRequest, CreateProjectRequest, DeactivateProjectRequest, DeleteProjectRequest,
    GetProjectByIdRequest, ListProjectsRequest, ProjectServiceClient,
};
use crate::connect::bountyhub::runner::v1::{
    CreateRunnerRegistrationRequest, DeleteRunnerRequest, ListRunnersRequest, RunnerServiceClient,
};

#[derive(Debug, Clone)]
pub struct Registration {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Runner {
    pub id: String,
    pub name: String,
    pub status: String,
}

/// RPC surface backed by the ConnectRPC clients generated from the
/// `bountyhub.project.v1` and `bountyhub.runner.v1` protos.
///
/// The indirection exists so command handlers can be tested with a mock
/// instead of a live HTTP transport.
#[async_trait::async_trait]
pub trait ConnectApi: Send + Sync {
    async fn create_runner_registration(&self) -> Result<Registration, String>;

    async fn list_runners(
        &self,
        page_size: Option<u32>,
        page_token: Option<String>,
    ) -> Result<Vec<Runner>, String>;

    async fn delete_runner(&self, id: String) -> Result<(), String>;

    async fn create_project(&self, name: String, description: String) -> Result<String, String>;

    async fn list_projects(
        &self,
        page_size: Option<u32>,
        page_token: Option<String>,
    ) -> Result<Vec<Project>, String>;

    async fn get_project(&self, id: String) -> Result<Project, String>;

    async fn delete_project(&self, id: String) -> Result<(), String>;

    async fn activate_project(&self, id: String) -> Result<(), String>;

    async fn deactivate_project(&self, id: String) -> Result<(), String>;
}

pub struct ConnectClients {
    runner: RunnerServiceClient<HttpClient>,
    project: ProjectServiceClient<HttpClient>,
}

impl ConnectClients {
    /// Build ConnectRPC clients for the BountyHub services rooted at `url`.
    ///
    /// The bearer token is sent as a default `authorization` header on every
    /// call; requests use the Connect protocol with JSON encoding.
    pub fn new(url: &str, token: &str) -> Result<Self, String> {
        let base: http::Uri = url
            .parse()
            .map_err(|e| format!("invalid BOUNTYHUB_URL '{url}': {e}"))?;

        let tls = {
            // Both `ring` (via connectrpc) and `aws-lc-rs` are in the
            // dependency graph, so rustls cannot pick a process-level
            // provider on its own — select one explicitly.
            let _ = rustls::crypto::ring::default_provider().install_default();
            let mut roots = rustls::RootCertStore::empty();
            let native = rustls_native_certs::load_native_certs();
            if let Some(err) = native.errors.first() {
                return Err(format!("failed to load platform certificates: {err}"));
            }
            for cert in native.certs {
                roots
                    .add(cert)
                    .map_err(|e| format!("failed to load platform certificate: {e}"))?;
            }
            Arc::new(
                rustls::ClientConfig::builder()
                    .with_root_certificates(roots)
                    .with_no_client_auth(),
            )
        };
        let http = HttpClient::with_tls(tls);

        let config = ClientConfig::new(base)
            .json()
            .with_default_timeout(Duration::from_secs(30))
            .with_default_header(http::header::AUTHORIZATION, format!("Bearer {}", token));

        Ok(Self {
            runner: RunnerServiceClient::new(http.clone(), config.clone()),
            project: ProjectServiceClient::new(http, config),
        })
    }
}

fn describe_error(context: &str, err: ConnectError) -> String {
    let code = format!("{:?}", err.code);
    match err.message {
        Some(message) => format!("{context}: {code}: {message}"),
        None => format!("{context}: {code}"),
    }
}

#[async_trait::async_trait]
impl ConnectApi for ConnectClients {
    async fn create_runner_registration(&self) -> Result<Registration, String> {
        let resp = self
            .runner
            .create_runner_registration(CreateRunnerRegistrationRequest::default())
            .await
            .map_err(|e| describe_error("failed to create runner registration", e))?;

        let view = resp.into_view();
        Ok(Registration {
            url: view.url.to_string(),
            token: view.token.to_string(),
        })
    }

    async fn list_runners(
        &self,
        page_size: Option<u32>,
        page_token: Option<String>,
    ) -> Result<Vec<Runner>, String> {
        let request = ListRunnersRequest {
            page_size: page_size.unwrap_or(0),
            page_token: page_token.unwrap_or_default(),
            __buffa_unknown_fields: Default::default(),
        };

        let resp = self
            .runner
            .list_runners(request)
            .await
            .map_err(|e| describe_error("failed to list runners", e))?;

        let view = resp.into_view();
        Ok(view
            .runners
            .iter()
            .map(|runner| Runner {
                id: runner.id.to_string(),
                name: runner.name.to_string(),
                status: runner.status.to_string(),
            })
            .collect())
    }

    async fn delete_runner(&self, id: String) -> Result<(), String> {
        let request = DeleteRunnerRequest {
            id,
            __buffa_unknown_fields: Default::default(),
        };

        self.runner
            .delete_runner(request)
            .await
            .map_err(|e| describe_error("failed to delete runner", e))?;

        Ok(())
    }

    async fn create_project(&self, name: String, description: String) -> Result<String, String> {
        let request = CreateProjectRequest {
            name,
            description,
            __buffa_unknown_fields: Default::default(),
        };

        let resp = self
            .project
            .create_project(request)
            .await
            .map_err(|e| describe_error("failed to create project", e))?;

        Ok(resp.into_view().id.to_string())
    }

    async fn list_projects(
        &self,
        page_size: Option<u32>,
        page_token: Option<String>,
    ) -> Result<Vec<Project>, String> {
        let request = ListProjectsRequest {
            page_size: page_size.unwrap_or(0),
            page_token: page_token.unwrap_or_default(),
            __buffa_unknown_fields: Default::default(),
        };

        let resp = self
            .project
            .list_projects(request)
            .await
            .map_err(|e| describe_error("failed to list projects", e))?;

        let view = resp.into_view();
        Ok(view
            .items
            .iter()
            .map(|item| Project {
                id: item.id.to_string(),
                name: item.name.to_string(),
                description: item.description.to_string(),
                status: item.status.to_string(),
                updated_at: item
                    .updated_at
                    .as_option()
                    .map(|timestamp| timestamp.seconds),
            })
            .collect())
    }

    async fn get_project(&self, id: String) -> Result<Project, String> {
        let request = GetProjectByIdRequest {
            id,
            __buffa_unknown_fields: Default::default(),
        };

        let resp = self
            .project
            .get_project_by_id(request)
            .await
            .map_err(|e| describe_error("failed to get project", e))?;

        let view = resp.into_view();
        Ok(Project {
            id: view.id.to_string(),
            name: view.name.to_string(),
            description: view.description.to_string(),
            status: view.status.to_string(),
            updated_at: view
                .updated_at
                .as_option()
                .map(|timestamp| timestamp.seconds),
        })
    }

    async fn delete_project(&self, id: String) -> Result<(), String> {
        let request = DeleteProjectRequest {
            id,
            __buffa_unknown_fields: Default::default(),
        };

        self.project
            .delete_project(request)
            .await
            .map_err(|e| describe_error("failed to delete project", e))?;

        Ok(())
    }

    async fn activate_project(&self, id: String) -> Result<(), String> {
        let request = ActivateProjectRequest {
            id,
            __buffa_unknown_fields: Default::default(),
        };

        self.project
            .activate_project(request)
            .await
            .map_err(|e| describe_error("failed to activate project", e))?;

        Ok(())
    }

    async fn deactivate_project(&self, id: String) -> Result<(), String> {
        let request = DeactivateProjectRequest {
            id,
            __buffa_unknown_fields: Default::default(),
        };

        self.project
            .deactivate_project(request)
            .await
            .map_err(|e| describe_error("failed to deactivate project", e))?;

        Ok(())
    }
}

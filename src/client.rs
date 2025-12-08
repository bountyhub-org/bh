#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig};
use uuid::Uuid;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
    #[error("Not Found")]
    NotFound,
    #[error("Conflict")]
    Conflict,
    #[error("Error: $0")]
    Generic(String),
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::StatusCode(401) => Error::Unauthorized,
            ureq::Error::StatusCode(403) => Error::Forbidden,
            ureq::Error::StatusCode(404) => Error::NotFound,
            ureq::Error::StatusCode(409) => Error::Conflict,
            err => Error::Generic(format!("{err:?}")),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DispatchScanRequest {
    pub scan_name: String,
    pub inputs: Option<BTreeMap<String, Value>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadBlobFileRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RunnerRegistrationResponse {
    pub url: String,
    pub token: String,
}

#[cfg_attr(test, automock)]
pub trait Client {
    fn download_job_artifact(
        &self,
        job_id: Uuid,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>>;

    fn delete_job_artifact(&self, job_id: Uuid, name: &str) -> Result<()>;

    fn delete_job(&self, job_id: Uuid) -> Result<()>;

    fn dispatch_scan(
        &self,
        workflow_id: Uuid,
        scan_name: String,
        inputs: Option<BTreeMap<String, Value>>,
    ) -> Result<()>;

    fn download_blob_file(&self, path: &str) -> Result<Box<dyn Read + Send + Sync + 'static>>;

    fn upload_blob_file(&self, file: File, dst: &str) -> Result<()>;

    fn create_runner_registration(&self) -> Result<RunnerRegistrationResponse>;

    fn create_bhlast_domain(&self) -> Result<String>;
}

pub struct HTTPClient {
    authorization: String,
    bountyhub_domain: String,
    bountyhub_agent: Agent,
    file_agent: Agent,
}

impl HTTPClient {
    pub fn new(bountyhub_domain: &str, pat: &str, version: &str) -> Self {
        let tls = TlsConfig::builder()
            .root_certs(RootCerts::PlatformVerifier)
            .build();

        let ua = format!("bh/{}", version);
        let bountyhub_agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_send_body(Some(Duration::from_secs(10)))
                .user_agent(ua.as_str())
                .timeout_connect(Some(Duration::from_secs(10)))
                .timeout_recv_response(Some(Duration::from_secs(10)))
                .timeout_send_request(Some(Duration::from_secs(10)))
                .tls_config(tls.clone())
                .build(),
        );
        let file_agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_recv_response(Some(Duration::from_secs(240)))
                .user_agent(ua.as_str())
                .timeout_connect(Some(Duration::from_secs(10)))
                .timeout_send_body(Some(Duration::from_secs(240)))
                .timeout_send_request(Some(Duration::from_secs(10)))
                .tls_config(tls.clone())
                .build(),
        );

        Self {
            authorization: format!("Bearer {}", pat),
            bountyhub_domain: bountyhub_domain.to_string(),
            bountyhub_agent,
            file_agent,
        }
    }

    #[cfg(test)]
    pub fn bountyhub_domain(&self) -> String {
        self.bountyhub_domain.clone()
    }

    #[cfg(test)]
    pub fn authorization(&self) -> String {
        self.authorization.clone()
    }
}

impl Client for HTTPClient {
    fn download_job_artifact(
        &self,
        job_id: Uuid,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>> {
        let url = format!(
            "{0}/api/v0/workflows/jobs/{job_id}/artifacts/{name}",
            self.bountyhub_domain
        );
        let UrlResponse { url } = self
            .bountyhub_agent
            .get(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .call()?
            .body_mut()
            .read_json()?;

        let res = self.file_agent.get(url.as_str()).call()?;

        Ok(Box::new(res.into_body().into_reader()))
    }

    fn delete_job_artifact(&self, job_id: Uuid, name: &str) -> Result<()> {
        let url = format!(
            "{0}/api/v0/workflows/jobs/{job_id}/artifacts/{name}",
            self.bountyhub_domain
        );

        self.bountyhub_agent
            .delete(url)
            .header("Authorization", self.authorization.as_str())
            .call()?;

        Ok(())
    }

    fn delete_job(&self, job_id: Uuid) -> Result<()> {
        let url = format!("{0}/api/v0/workflows/jobs/{job_id}", self.bountyhub_domain);

        self.bountyhub_agent
            .delete(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .call()?;
        Ok(())
    }

    fn dispatch_scan(
        &self,
        workflow_id: Uuid,
        scan_name: String,
        inputs: Option<BTreeMap<String, Value>>,
    ) -> Result<()> {
        let url = format!(
            "{0}/api/v0/workflows/{workflow_id}/scans/dispatch",
            self.bountyhub_domain
        );

        self.bountyhub_agent
            .post(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .send_json(DispatchScanRequest { scan_name, inputs })?;

        Ok(())
    }

    fn download_blob_file(&self, path: &str) -> Result<Box<dyn Read + Send + Sync + 'static>> {
        let url = format!("{0}/api/v0/blobs/{1}", self.bountyhub_domain, encode(path),);
        let UrlResponse { url } = self
            .bountyhub_agent
            .get(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .call()?
            .body_mut()
            .read_json()?;

        let res = self.file_agent.get(url.as_str()).call()?;

        Ok(Box::new(res.into_body().into_reader()))
    }

    fn upload_blob_file(&self, file: File, dst: &str) -> Result<()> {
        let url = format!("{0}/api/v0/blobs/files", self.bountyhub_domain);
        let UrlResponse { url } = self
            .bountyhub_agent
            .post(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .send_json(UploadBlobFileRequest {
                path: dst.to_string(),
            })?
            .body_mut()
            .read_json()?;

        self.file_agent.put(&url).send(file)?;

        Ok(())
    }

    fn create_runner_registration(&self) -> Result<RunnerRegistrationResponse> {
        let url = format!("{0}/api/v0/runner-registrations", self.bountyhub_domain);

        Ok(self
            .bountyhub_agent
            .post(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .send_json(json!({}))?
            .body_mut()
            .read_json()?)
    }

    fn create_bhlast_domain(&self) -> Result<String> {
        let url = format!("{0}/api/v0/bhlast/domains", self.bountyhub_domain);

        let CreatedResponse { id } = self
            .bountyhub_agent
            .post(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .send_json(json!({}))?
            .body_mut()
            .read_json()?;

        Ok(id)
    }
}

fn encode(s: &str) -> String {
    percent_encoding::percent_encode(s.as_bytes(), percent_encoding::NON_ALPHANUMERIC).to_string()
}

#[derive(Deserialize, Debug)]
struct UrlResponse {
    url: String,
}

#[derive(Deserialize, Debug)]
struct CreatedResponse {
    id: String,
}

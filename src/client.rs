use error_stack::{Context, Report, Result, ResultExt};
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig};
use uuid::Uuid;

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

#[cfg_attr(test, automock)]
pub trait Client {
    fn download_job_artifact(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: &str,
        job_id: Uuid,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError>;

    fn delete_job_artifact(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: &str,
        job_id: Uuid,
        name: &str,
    ) -> Result<(), ClientError>;

    fn delete_job(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: &str,
        job_id: Uuid,
    ) -> Result<(), ClientError>;

    fn dispatch_scan(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: String,
        inputs: Option<BTreeMap<String, Value>>,
    ) -> Result<(), ClientError>;

    fn download_blob_file(
        &self,
        path: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError>;

    fn upload_blob_file(&self, file: File, dst: &str) -> Result<(), ClientError>;
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
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: &str,
        job_id: Uuid,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError> {
        let url = format!(
            "{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/revisions/{revision_id}/scans/{scan_name}/jobs/{job_id}/artifacts/{name}",
            self.bountyhub_domain
        );
        let UrlResponse { url } = self
            .bountyhub_agent
            .get(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to request download")?
            .body_mut()
            .read_json()
            .change_context(ClientError)
            .attach_printable("Failed to parse response")?;

        let res = self
            .file_agent
            .get(url.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to download file")?;

        Ok(Box::new(res.into_body().into_reader()))
    }

    fn delete_job_artifact(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: &str,
        job_id: Uuid,
        name: &str,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/revisions/{revision_id}/scans/{scan_name}/jobs/{job_id}/artifacts/{name}",
            self.bountyhub_domain
        );

        self.bountyhub_agent
            .delete(url)
            .header("Authorization", self.authorization.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to delete artifact")?;

        Ok(())
    }

    fn delete_job(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: &str,
        job_id: Uuid,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/revisions/{revision_id}/scans/{scan_name}/jobs/{job_id}",
            self.bountyhub_domain
        );

        self.bountyhub_agent
            .delete(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to delete job")?;
        Ok(())
    }

    fn dispatch_scan(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        scan_name: String,
        inputs: Option<BTreeMap<String, Value>>,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/revisions/{revision_id}/scans/dispatch",
            self.bountyhub_domain
        );

        match self
            .bountyhub_agent
            .post(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .send_json(DispatchScanRequest { scan_name, inputs })
        {
            Ok(_) => Ok(()),
            Err(ureq::Error::StatusCode(409)) => {
                Err(Report::new(ClientError).attach_printable("Scan is already scheduled"))
            }
            Err(e) => Err(Report::new(ClientError).attach_printable(e.to_string())),
        }
    }

    fn download_blob_file(
        &self,
        path: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError> {
        let url = format!("{0}/api/v0/blobs/{1}", self.bountyhub_domain, encode(path),);
        let UrlResponse { url } = self
            .bountyhub_agent
            .get(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to request download")?
            .body_mut()
            .read_json()
            .change_context(ClientError)
            .attach_printable("Failed to parse response")?;

        let res = self
            .file_agent
            .get(url.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to download file")?;

        Ok(Box::new(res.into_body().into_reader()))
    }

    fn upload_blob_file(&self, file: File, dst: &str) -> Result<(), ClientError> {
        let url = format!("{0}/api/v0/blobs/files", self.bountyhub_domain);
        let UrlResponse { url } = self
            .bountyhub_agent
            .post(url.as_str())
            .header("Authorization", self.authorization.as_str())
            .send_json(UploadBlobFileRequest {
                path: dst.to_string(),
            })
            .change_context(ClientError)
            .attach_printable("failed to create upload link")?
            .body_mut()
            .read_json()
            .change_context(ClientError)
            .attach_printable("failed to parse response")?;

        self.file_agent
            .put(&url)
            .send(file)
            .change_context(ClientError)
            .attach_printable("failed to send file")?;

        Ok(())
    }
}

fn encode(s: &str) -> String {
    percent_encoding::percent_encode(s.as_bytes(), percent_encoding::NON_ALPHANUMERIC).to_string()
}

#[derive(Debug)]
pub struct ClientError;

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Client error")
    }
}

impl Context for ClientError {}

#[derive(Deserialize, Debug)]
struct UrlResponse {
    url: String,
}

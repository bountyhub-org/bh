use error_stack::{Context, Report, Result, ResultExt};
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::time::Duration;
use ureq::Agent;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DispatchScanRequest {
    pub scan_name: String,
    pub inputs: Option<BTreeMap<String, Value>>,
}

#[cfg_attr(test, automock)]
pub trait Client {
    fn download_job_result_file(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        job_id: Uuid,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError>;

    fn delete_job(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
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
}

pub struct HTTPClient {
    authorization: String,
    bountyhub_domain: String,
    bountyhub_agent: Agent,
    file_agent: Agent,
}

impl HTTPClient {
    pub fn new(bountyhub_domain: &str, pat: &str, version: &str) -> Self {
        let ua = format!("bh/{}", version);
        let bountyhub_agent = ureq::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(ua.as_str())
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(10))
            .timeout_write(Duration::from_secs(10))
            .build();
        let file_agent = ureq::builder()
            .timeout(Duration::from_secs(240))
            .user_agent(ua.as_str())
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(240))
            .timeout_write(Duration::from_secs(10))
            .build();

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
    fn download_job_result_file(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        job_id: Uuid,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError> {
        let url = format!("{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/revisions/{revision_id}/jobs/{job_id}/result", self.bountyhub_domain);
        let FileResult { url } = self
            .bountyhub_agent
            .get(url.as_str())
            .set("Authorization", self.authorization.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to request download")?
            .into_json()
            .change_context(ClientError)
            .attach_printable("Failed to parse response")?;

        let res = self
            .file_agent
            .get(url.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to download file")?;

        Ok(res.into_reader())
    }

    fn delete_job(
        &self,
        project_id: Uuid,
        workflow_id: Uuid,
        revision_id: Uuid,
        job_id: Uuid,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/revisions/{revision_id}/jobs/{job_id}",
            self.bountyhub_domain
        );

        self.bountyhub_agent
            .delete(url.as_str())
            .set("Authorization", self.authorization.as_str())
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
            .set("Authorization", self.authorization.as_str())
            .send_json(DispatchScanRequest { scan_name, inputs })
        {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(409, _)) => {
                Err(Report::new(ClientError).attach_printable("Scan is already scheduled"))
            }
            Err(e) => Err(Report::new(ClientError).attach_printable(e.to_string())),
        }
    }
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
struct FileResult {
    url: String,
}

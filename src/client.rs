use error_stack::{Context, Result, ResultExt};
#[cfg(test)]
use mockall::automock;
use serde::Deserialize;
use std::fmt;
use std::io::Read;
use std::time::Duration;
use ureq::Agent;

#[cfg_attr(test, automock)]
pub trait Client {
    fn download_job_result_file(
        &self,
        project_id: &str,
        workflow_id: &str,
        revision_id: &str,
        job_id: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError>;

    fn delete_job(
        &self,
        project_id: &str,
        workflow_id: &str,
        revision_id: &str,
        job_id: &str,
    ) -> Result<(), ClientError>;
}

pub struct HTTPClient {
    authorization: String,
    agent: Agent,
    domain: String,
}

impl HTTPClient {
    pub fn new(domain: &str, token: &str, version: &str) -> Self {
        let agent = ureq::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(format!("bh/{}", version).as_str())
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(10))
            .timeout_write(Duration::from_secs(10))
            .build();

        Self {
            authorization: format!("Bearer {}", token),
            agent,
            domain: domain.to_string(),
        }
    }

    #[cfg(test)]
    pub fn domain(&self) -> String {
        self.domain.clone()
    }

    #[cfg(test)]
    pub fn authorization(&self) -> String {
        self.authorization.clone()
    }
}

impl Client for HTTPClient {
    fn download_job_result_file(
        &self,
        project_id: &str,
        workflow_id: &str,
        revision_id: &str,
        job_id: &str,
    ) -> Result<Box<dyn Read + Send + Sync + 'static>, ClientError> {
        let url = format!("{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/rev/{revision_id}/jobs/{job_id}/result", self.domain);
        let FileResult { url } = self
            .agent
            .get(url.as_str())
            .set("Authorization", self.authorization.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to request download")?
            .into_json()
            .change_context(ClientError)
            .attach_printable("Failed to parse response")?;

        let res = self
            .agent
            .get(url.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to download file")?;

        Ok(res.into_reader())
    }

    fn delete_job(
        &self,
        project_id: &str,
        workflow_id: &str,
        revision_id: &str,
        job_id: &str,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{0}/api/v0/projects/{project_id}/workflows/{workflow_id}/rev/{revision_id}/jobs/{job_id}",
            self.domain
        );

        self.agent
            .delete(url.as_str())
            .set("Authorization", self.authorization.as_str())
            .call()
            .change_context(ClientError)
            .attach_printable("Failed to delete job")?;
        Ok(())
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

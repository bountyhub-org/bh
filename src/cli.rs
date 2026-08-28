use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

use serde_json::Value;
use usage::{Args, Cli, Run, RunAsync, RunAsyncWith, Subcommands, ValueEnum};
use uuid::Uuid;

use crate::client::{Client, Error as ClientError, HTTPClient};
use crate::connect_api::{ConnectApi, ConnectClients};
use crate::validation;

type Result<T> = std::result::Result<T, String>;

/// BountyHub CLI.
///
/// Commands rely on `BOUNTYHUB_TOKEN` and `BOUNTYHUB_URL` environment
/// variables. RPC commands talk to the ConnectRPC services; artifact and
/// blob commands use the REST API.
#[derive(Cli, Debug)]
#[usage(bin = "bh", version, unknown_flags = "error", arg_required_else_help)]
pub struct Cli {
    #[usage(subcommand)]
    command: Commands,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
enum Commands {
    /// Job related commands
    Job(Job),

    /// Project related commands
    Project(Project),

    /// Scan related commands
    Scan(Scan),

    /// Blob related commands
    Blob(Blob),

    /// Runner related commands
    Runner(Runner),

    /// Bhlast related commands
    Bhlast(Bhlast),

    /// Markdown related commands
    #[usage(no_ctx)]
    Md(Md),

    /// Shell completion commands
    #[usage(no_ctx)]
    Completion(Completion),
}

impl Cli {
    pub async fn run() -> Result<()> {
        let cli = Cli::parse();
        cli.command.run_async_with_lazy(Ctx::new).await
    }
}

/// Shared context handed to every command that talks to BountyHub.
///
/// Construction is lazy (only commands that need it pay for it) and the
/// environment validation error is carried until a command consumes it.
pub struct Ctx(std::result::Result<Env, String>);

pub struct Env {
    rest: HTTPClient,
    connect: Arc<dyn ConnectApi>,
}

impl Ctx {
    fn new() -> Self {
        Self(env_config())
    }

    fn env(&self) -> std::result::Result<&Env, String> {
        self.0.as_ref().map_err(Clone::clone)
    }

    fn connect(&self) -> std::result::Result<Arc<dyn ConnectApi>, String> {
        self.env().map(|env| env.connect.clone())
    }

    fn rest(&self) -> std::result::Result<&HTTPClient, String> {
        self.env().map(|env| &env.rest)
    }
}

fn env_config() -> Result<Env> {
    let pat = match env::var("BOUNTYHUB_TOKEN") {
        Ok(token) => {
            if !token.starts_with("bhv") {
                return Err("Invalid token format: token does not start with bhv".to_string());
            }
            token
        }
        Err(err) => {
            return Err(format!("Failed to get BOUNTYHUB_TOKEN: {err:?}"));
        }
    };

    let bountyhub =
        env::var("BOUNTYHUB_URL").unwrap_or_else(|_| "https://bountyhub.org".to_string());
    // The ConnectRPC services may be deployed under a different base URL
    // (host or path prefix) than the REST API; BOUNTYHUB_CONNECT_URL
    // overrides the base used for Connect calls.
    let connect_url = env::var("BOUNTYHUB_CONNECT_URL").unwrap_or_else(|_| bountyhub.clone());
    let connect = ConnectClients::new(&connect_url, &pat)?;

    Ok(Env {
        rest: HTTPClient::new(&bountyhub, &pat, env!("CARGO_PKG_VERSION")),
        connect: Arc::new(connect),
    })
}

/// Job based commands
#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct Job {
    #[usage(subcommand)]
    command: JobCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum JobCommand {
    /// Job artifact related commands
    Artifact(JobArtifact),

    /// Delete a job
    Delete(JobDelete),
}

#[derive(Args, Debug, Clone)]
pub struct JobDelete {
    /// The ID of the job to delete
    #[usage(short, long, env = "BOUNTYHUB_JOB_ID")]
    job_id: Uuid,
}

#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct JobArtifact {
    #[usage(subcommand)]
    command: JobArtifactCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum JobArtifactCommand {
    /// Download an artifact uploaded by a job
    Download(JobArtifactDownload),

    /// Delete job artifact
    Delete(JobArtifactDelete),
}

#[derive(Args, Debug, Clone)]
pub struct JobArtifactDownload {
    /// The ID of the job to download the artifact from
    #[usage(short, long, env = "BOUNTYHUB_JOB_ID")]
    job_id: Uuid,

    /// Name of the artifact to download
    ///
    /// This is the name given when the artifact was uploaded
    #[usage(short, long, env = "BOUNTYHUB_JOB_ARTIFACT_NAME")]
    artifact_name: String,

    /// Directory where the output should be downloaded to.
    ///
    /// The artifact will be saved at `{output}/{artifact_name}`.
    /// If unzip is set, the artifact will be unzipped into the output
    /// directory. If not set, the current directory is used.
    #[usage(short, long, env = "BOUNTYHUB_OUTPUT")]
    output: Option<String>,

    /// Unzip the downloaded artifact to the output directory
    ///
    /// The zipped file will **not** be removed after unzipping.
    #[usage(long)]
    unzip: bool,
}

#[derive(Args, Debug, Clone)]
pub struct JobArtifactDelete {
    /// The ID of the job owning the artifact
    #[usage(short, long, env = "BOUNTYHUB_JOB_ID")]
    job_id: Uuid,

    /// Name of the artifact to delete
    #[usage(short, long, env = "BOUNTYHUB_JOB_ARTIFACT_NAME")]
    artifact_name: String,
}

#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct Scan {
    #[usage(subcommand)]
    command: ScanCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum ScanCommand {
    /// Dispatch a scan from the latest revision of the workflow
    Dispatch(ScanDispatch),
}

#[derive(Args, Debug, Clone)]
pub struct ScanDispatch {
    /// The ID of the workflow to dispatch the scan for
    #[usage(short, long, env = "BOUNTYHUB_WORKFLOW_ID")]
    workflow_id: Uuid,

    /// Name of the scan to dispatch
    #[usage(short, long, env = "BOUNTYHUB_SCAN_NAME")]
    scan_name: String,

    /// String inputs in `key=value` form, repeatable
    #[usage(long)]
    input_string: Vec<String>,

    /// Boolean inputs in `key=value` form, repeatable
    #[usage(long)]
    input_bool: Vec<String>,
}

#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct Blob {
    #[usage(subcommand)]
    command: BlobCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum BlobCommand {
    /// Download a file from bountyhub.org blob storage
    Download(BlobDownload),

    /// Upload a file to bountyhub.org blob storage
    Upload(BlobUpload),
}

#[derive(Args, Debug, Clone)]
pub struct BlobDownload {
    /// Path of the file on bountyhub.org blob storage
    #[usage(short, long)]
    src: String,

    /// Local destination path
    #[usage(short, long, env = "BOUNTYHUB_OUTPUT")]
    dst: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct BlobUpload {
    /// Source file on the local filesystem
    #[usage(short, long)]
    src: String,

    /// Destination path on bountyhub.org blobs
    #[usage(long)]
    dst: String,
}

/// Runner based commands
#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct Runner {
    #[usage(subcommand)]
    command: RunnerCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum RunnerCommand {
    /// Runner registration commands
    Registration(RunnerRegistration),

    /// List runners
    List(RunnerList),

    /// Delete a runner
    Delete(RunnerDelete),
}

#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct RunnerRegistration {
    #[usage(subcommand)]
    command: RunnerRegistrationCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum RunnerRegistrationCommand {
    /// Get newly created runner registration token
    #[usage(name = "token")]
    Token(RunnerRegistrationToken),

    /// Get runner registration command with newly created token
    #[usage(name = "command")]
    Command(RunnerRegistrationCommand_),
}

#[derive(Args, Debug)]
pub struct RunnerRegistrationToken;

#[derive(Args, Debug)]
pub struct RunnerRegistrationCommand_;

/// Project based commands
#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct Project {
    #[usage(subcommand)]
    command: ProjectCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum ProjectCommand {
    /// Create a project
    Create(ProjectCreate),

    /// List projects
    List(ProjectList),

    /// Show a single project
    Get(ProjectGet),

    /// Delete a project
    Delete(ProjectDelete),

    /// Activate a project
    Activate(ProjectActivate),

    /// Deactivate a project
    Deactivate(ProjectDeactivate),
}

#[derive(Args, Debug, Clone)]
pub struct ProjectCreate {
    /// Name of the project
    #[usage(short, long)]
    name: String,

    /// Description of the project
    #[usage(short, long)]
    description: String,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectList {
    /// Maximum number of projects to return
    #[usage(long)]
    page_size: Option<u32>,

    /// Token from a previous response to fetch the next page
    #[usage(long)]
    page_token: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectGet {
    /// The ID of the project
    #[usage(short, long)]
    id: String,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectDelete {
    /// The ID of the project
    #[usage(short, long)]
    id: String,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectActivate {
    /// The ID of the project
    #[usage(short, long)]
    id: String,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectDeactivate {
    /// The ID of the project
    #[usage(short, long)]
    id: String,
}

#[derive(Args, Debug, Clone)]
pub struct RunnerList {
    /// Maximum number of runners to return
    #[usage(long)]
    page_size: Option<u32>,

    /// Token from a previous response to fetch the next page
    #[usage(long)]
    page_token: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct RunnerDelete {
    /// The ID of the runner
    #[usage(short, long)]
    id: String,
}

#[derive(Args, Debug)]
#[usage(run_async_with)]
pub struct Bhlast {
    #[usage(subcommand)]
    command: BhlastCommand,
}

#[derive(Subcommands, Debug)]
#[usage(run_async_with)]
pub enum BhlastCommand {
    /// Create a new bhlast server
    Create(BhlastCreate),
}

#[derive(Args, Debug)]
pub struct BhlastCreate;

#[derive(Args, Debug)]
pub struct Md {
    #[usage(subcommand)]
    command: MdCommand,
}

impl RunAsync for Md {
    type Output = Result<()>;

    async fn run_async(self) -> Self::Output {
        self.command.run()
    }
}

#[derive(Subcommands, Debug)]
#[usage(run)]
pub enum MdCommand {
    /// Generate a usage spec (KDL) for the CLI
    Docs(MdDocs),
}

#[derive(Args, Debug)]
pub struct MdDocs;

impl Run for MdDocs {
    type Output = Result<()>;

    fn run(self) -> Self::Output {
        println!("{}", Cli::to_kdl());
        Ok(())
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

#[derive(Args, Debug)]
pub struct Completion {
    #[usage(value_enum)]
    shell: Shell,
}

impl RunAsync for Completion {
    type Output = Result<()>;

    async fn run_async(self) -> Self::Output {
        let shell = match self.shell {
            Shell::Bash => usage::complete::Shell::Bash,
            Shell::Zsh => usage::complete::Shell::Zsh,
            Shell::Fish => usage::complete::Shell::Fish,
        };

        print!("{}", Cli::app().completion_app().completion_script(shell));
        Ok(())
    }
}

fn split_input(input: &str) -> Result<(&str, &str)> {
    let split = input.splitn(2, '=');
    let mut k = split.take(2);
    Ok((
        k.next().ok_or("Failed to get the key from string input")?,
        k.next()
            .ok_or("Failed to get the value from string input")?,
    ))
}

fn parse_inputs(
    input_string: &[String],
    input_bool: &[String],
) -> Result<Option<BTreeMap<String, Value>>> {
    if input_string.is_empty() && input_bool.is_empty() {
        return Ok(None);
    }

    let mut m = BTreeMap::new();

    for v in input_string {
        let (k, v) = split_input(v)?;
        if !validation::valid_workflow_var_key(k) {
            return Err(format!("Key '{k}' is in invalid format"));
        }
        m.insert(k.to_string(), Value::String(v.to_string()));
    }

    for v in input_bool {
        let (k, v) = split_input(v)?;
        if !validation::valid_workflow_var_key(k) {
            return Err(format!("Key '{k}' is in invalid format"));
        }
        let b = v
            .parse::<bool>()
            .map_err(|_| format!("Value '{v}' is not a valid boolean"))?;
        m.insert(k.to_string(), Value::Bool(b));
    }

    Ok(Some(m))
}

fn print_project(project: &crate::connect_api::Project) {
    println!("{}: {}", project.id, project.name);
    if !project.description.is_empty() {
        println!("  description: {}", project.description);
    }
    println!("  status: {}", project.status);
    if let Some(seconds) = project.updated_at {
        println!("  updated_at: {seconds}");
    }
}

// ---------------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------------

impl RunAsyncWith<Ctx> for JobDelete {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let client = ctx.rest()?;
        delete_job(client, self)
    }
}

fn delete_job<C>(client: &C, cmd: JobDelete) -> Result<()>
where
    C: Client,
{
    client
        .delete_job(cmd.job_id)
        .map_err(|e| format!("failed to delete job: {e:?}"))
}

impl RunAsyncWith<Ctx> for JobArtifactDownload {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let client = ctx.rest()?;
        download_job_artifact(client, self)
    }
}

fn download_job_artifact<C>(client: &C, cmd: JobArtifactDownload) -> Result<()>
where
    C: Client,
{
    let output = match cmd.output {
        Some(output) => {
            let output = PathBuf::from(output);
            if output.is_dir() {
                output.join(&cmd.artifact_name)
            } else {
                output
            }
        }
        None => env::current_dir()
            .map_err(|err| format!("Failed to get current directory: {err:?}"))?
            .join(&cmd.artifact_name),
    };

    println!("Downloading artifact to {output:?}");

    let mut freader = client
        .download_job_artifact(cmd.job_id, &cmd.artifact_name)
        .map_err(|err| format!("Failed to download file: {err:?}"))?;

    let mut fwriter =
        fs::File::create(&output).map_err(|err| format!("Failed to create file: {err:?}"))?;

    std::io::copy(&mut *freader, &mut fwriter)
        .map_err(|err| format!("failed to write file: {err:?}"))?;

    if cmd.unzip {
        let file = fs::File::open(&output)
            .map_err(|err| format!("Failed to open file for unzip: {err:?}"))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|err| format!("Failed to read zip archive: {err:?}"))?;
        archive
            .extract(
                output
                    .parent()
                    .ok_or("Failed to get parent directory for unzip")?,
            )
            .map_err(|err| format!("Failed to extract zip archive: {err:?}"))?;

        println!("Unzipped artifact to {output:?}");
    }

    Ok(())
}

impl RunAsyncWith<Ctx> for JobArtifactDelete {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let client = ctx.rest()?;
        delete_job_artifact(client, self)
    }
}

fn delete_job_artifact<C>(client: &C, cmd: JobArtifactDelete) -> Result<()>
where
    C: Client,
{
    client
        .delete_job_artifact(cmd.job_id, &cmd.artifact_name)
        .map_err(|err| format!("failed to delete job artifact: {err:?}"))
}

impl RunAsyncWith<Ctx> for ScanDispatch {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let client = ctx.rest()?;
        dispatch_scan(client, self)
    }
}

fn dispatch_scan<C>(client: &C, cmd: ScanDispatch) -> Result<()>
where
    C: Client,
{
    if !validation::valid_scan_name(&cmd.scan_name) {
        return Err(format!("Invalid scan name: '{}'", cmd.scan_name));
    }

    let inputs = parse_inputs(&cmd.input_string, &cmd.input_bool)?;

    client
        .dispatch_scan(cmd.workflow_id, cmd.scan_name.clone(), inputs)
        .map_err(|e| format!("failed to dispatch scan: {e:?}"))
}

impl RunAsyncWith<Ctx> for BlobDownload {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let client = ctx.rest()?;
        download_blob(client, self)
    }
}

fn download_blob<C>(client: &C, cmd: BlobDownload) -> Result<()>
where
    C: Client,
{
    let output = match cmd.dst {
        Some(dst) => {
            let dst = PathBuf::from(dst);
            if dst.is_dir() {
                dst.join(&cmd.src)
            } else {
                dst
            }
        }
        None => env::current_dir()
            .map_err(|err| format!("Failed to get current directory: {err:?}"))?
            .join(Path::new(&cmd.src).file_name().unwrap_or_default()),
    };

    let mut freader = client
        .download_blob_file(&cmd.src)
        .map_err(|err| format!("Failed to download file: {err:?}"))?;

    let mut fwriter =
        fs::File::create(output).map_err(|err| format!("Failed to create output file: {err:?}"))?;

    std::io::copy(&mut *freader, &mut fwriter)
        .map_err(|err| format!("Failed to write to output: {err:?}"))?;
    Ok(())
}

impl RunAsyncWith<Ctx> for BlobUpload {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let client = ctx.rest()?;
        upload_blob(client, self)
    }
}

fn upload_blob<C>(client: &C, cmd: BlobUpload) -> Result<()>
where
    C: Client,
{
    let freader = fs::File::open(&cmd.src)
        .map_err(|err| format!("Failed to open file '{}': {err:?}", cmd.src))?;

    client
        .upload_blob_file(freader, &cmd.dst)
        .map_err(|err| format!("Failed to upload blob file: {err:?}"))
}

impl RunAsyncWith<Ctx> for RunnerRegistrationToken {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        let resp = api
            .create_runner_registration()
            .await
            .map_err(|err| format!("Failed to create runner registration: {err}"))?;

        print!("{}", resp.token);
        Ok(())
    }
}

impl RunAsyncWith<Ctx> for RunnerRegistrationCommand_ {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        let resp = api
            .create_runner_registration()
            .await
            .map_err(|err| format!("Failed to create runner registration: {err}"))?;

        println!(
            r#"runner configure --token "{}" --url "{}""#,
            resp.token, resp.url,
        );
        Ok(())
    }
}

impl RunAsyncWith<Ctx> for RunnerList {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        for runner in api
            .list_runners(self.page_size, self.page_token)
            .await
            .map_err(|err| format!("Failed to list runners: {err}"))?
        {
            println!("{}: {} ({})", runner.id, runner.name, runner.status);
        }

        Ok(())
    }
}

impl RunAsyncWith<Ctx> for RunnerDelete {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        api.delete_runner(self.id)
            .await
            .map_err(|err| format!("Failed to delete runner: {err}"))
    }
}

impl RunAsyncWith<Ctx> for ProjectCreate {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        let id = api
            .create_project(self.name, self.description)
            .await
            .map_err(|err| format!("Failed to create project: {err}"))?;

        println!("{id}");
        Ok(())
    }
}

impl RunAsyncWith<Ctx> for ProjectList {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        for project in api
            .list_projects(self.page_size, self.page_token)
            .await
            .map_err(|err| format!("Failed to list projects: {err}"))?
        {
            print_project(&project);
        }

        Ok(())
    }
}

impl RunAsyncWith<Ctx> for ProjectGet {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        let project = api
            .get_project(self.id)
            .await
            .map_err(|err| format!("Failed to get project: {err}"))?;

        print_project(&project);
        Ok(())
    }
}

impl RunAsyncWith<Ctx> for ProjectDelete {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        api.delete_project(self.id)
            .await
            .map_err(|err| format!("Failed to delete project: {err}"))
    }
}

impl RunAsyncWith<Ctx> for ProjectActivate {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        api.activate_project(self.id)
            .await
            .map_err(|err| format!("Failed to activate project: {err}"))
    }
}

impl RunAsyncWith<Ctx> for ProjectDeactivate {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let api = ctx.connect()?;
        api.deactivate_project(self.id)
            .await
            .map_err(|err| format!("Failed to deactivate project: {err}"))
    }
}

impl RunAsyncWith<Ctx> for BhlastCreate {
    type Output = Result<()>;

    async fn run_async_with(self, ctx: Ctx) -> Self::Output {
        let client = ctx.rest()?;
        match client.create_bhlast_domain() {
            Ok(id) => {
                println!("{id}");
                Ok(())
            }
            Err(ClientError::Forbidden) => Err("You cannot create more bhlast domains".to_string()),
            Err(ClientError::Unauthorized) => Err("Unauthorized: invalid token".to_string()),
            Err(e) => Err(format!("Failed to create bhlast domain: {e:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::MockClient;
    use mockall::predicate::*;

    #[test]
    fn test_download_failed() {
        let job_id = Uuid::now_v7();
        let artifact_name = "test.zip";

        let cmd = JobArtifactDownload {
            job_id,
            artifact_name: artifact_name.to_string(),
            output: None,
            unzip: false,
        };
        let mut client = MockClient::new();
        client
            .expect_download_job_artifact()
            .with(eq(job_id), eq(artifact_name))
            .times(1)
            .returning(|_, _| Err(ClientError::Unauthorized));

        let result = download_job_artifact(&client, cmd);
        assert!(result.is_err(), "expected error, got ok");
    }

    #[test]
    fn test_delete_job_call() {
        let job_id = Uuid::now_v7();

        let cmd = JobDelete { job_id };

        let mut client = MockClient::new();
        client
            .expect_delete_job()
            .with(eq(job_id))
            .times(1)
            .returning(|_| Ok(()));

        let result = delete_job(&client, cmd);
        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[test]
    fn test_dispatch_call_no_inputs() {
        let workflow_id = Uuid::now_v7();
        let cmd = ScanDispatch {
            workflow_id,
            scan_name: "example".to_string(),
            input_string: vec![],
            input_bool: vec![],
        };

        let mut client = MockClient::new();
        client
            .expect_dispatch_scan()
            .with(
                eq(workflow_id),
                function(|v: &String| v == "example"),
                function(|v: &Option<BTreeMap<String, Value>>| v.is_none()),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = dispatch_scan(&client, cmd);
        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[test]
    fn test_dispatch_call_with_inputs() {
        let workflow_id = Uuid::now_v7();
        let cmd = ScanDispatch {
            workflow_id,
            scan_name: "example".to_string(),
            input_string: vec!["s_key=s_val".to_string()],
            input_bool: vec!["b_key=true".to_string()],
        };

        let mut client = MockClient::new();
        client
            .expect_dispatch_scan()
            .with(
                eq(workflow_id),
                function(|v: &String| v == "example"),
                function(|v: &Option<BTreeMap<String, Value>>| match v {
                    None => false,
                    Some(input) => {
                        let ok = match input.get("s_key").expect("s_key to exist") {
                            Value::String(val) => val == "s_val",
                            _ => false,
                        };
                        match input.get("b_key").expect("b_key to exist") {
                            Value::Bool(val) => ok && *val,
                            _ => false,
                        }
                    }
                }),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = dispatch_scan(&client, cmd);
        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[test]
    fn test_split_inputs() {
        let input = "k=v";
        let result = split_input(input).unwrap_or_else(|_| panic!("{input}: want ok, got err"));
        assert_eq!(result.0, "k");
        assert_eq!(result.1, "v");

        let input = "test";
        let result = split_input(input);
        assert!(result.is_err(), "expected error, got {result:?}");

        let input = "k=v=a";
        let result = split_input(input).unwrap_or_else(|_| panic!("{input}: want ok, got err"));
        assert_eq!(result.0, "k");
        assert_eq!(result.1, "v=a");
    }

    #[test]
    fn test_download_blob_file() {
        let cmd = BlobDownload {
            src: "file.txt".to_string(),
            dst: None,
        };
        let mut client = MockClient::new();
        client
            .expect_download_blob_file()
            .with(function(|v: &str| v == "file.txt"))
            .times(1)
            .returning(|_| Err(ClientError::NotFound));

        let result = download_blob(&client, cmd);
        assert!(result.is_err(), "expected error, got ok");
    }

    #[test]
    fn test_spec_contains_commands() {
        let kdl = Cli::to_kdl();
        for cmd in [
            "job",
            "project",
            "scan",
            "blob",
            "runner",
            "bhlast",
            "md",
            "completion",
        ] {
            assert!(kdl.contains(cmd), "spec should mention '{cmd}'");
        }
    }
}

use crate::client::{Client, Error, HTTPClient};
use crate::validation;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::{Shell, generate};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs, io};
use uuid::Uuid;

type Result<T> = std::result::Result<T, String>;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run() -> Result<()> {
        let cli = Cli::parse();

        match cli.command {
            Some(command) => command.run()?,
            None => {
                Cli::command().print_help().expect("Failed to print help");
            }
        }

        Ok(())
    }
}

/// Commands rely on `BOUNTYHUB_TOKEN` and `BOUNTYHUB_URL`
/// environment variables.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Job related commands
    #[command(subcommand)]
    Job(Job),

    /// Scan related commands
    #[command(subcommand)]
    Scan(Scan),

    /// Blob related commands
    #[command(subcommand)]
    Blob(Blob),

    /// Runner related commands
    #[command(subcommand)]
    Runner(Runner),

    /// Bhlast related commands
    #[command(subcommand)]
    Bhlast(Bhlast),

    /// Markdown related commands
    #[command(subcommand)]
    Md(Md),

    /// Shell completion commands
    #[command(arg_required_else_help = true)]
    Completion(Completion),
}

impl Commands {
    fn run(self) -> Result<()> {
        match self {
            Commands::Md(md) => md.run()?,
            Commands::Completion(completion) => completion.run()?,
            Commands::Job(job) => {
                let client = new_client()?;
                job.run(client)?
            }
            Commands::Scan(scan) => {
                let client = new_client()?;
                scan.run(client)?
            }
            Commands::Runner(runner) => {
                let client = new_client()?;
                runner.run(client)?
            }
            Commands::Bhlast(bhlast) => {
                let client = new_client()?;
                bhlast.run(client)?
            }
            Commands::Blob(blob) => {
                let client = new_client()?;
                blob.run(client)?
            }
        }

        Ok(())
    }
}

fn new_client() -> Result<HTTPClient> {
    let pat = match env::var("BOUNTYHUB_TOKEN") {
        Ok(token) => {
            if !token.starts_with("bhv") {
                return Err("Invalid token format: token does not start with bhv".to_string());
            }
            token
        }
        Err(err) => {
            return Err(format!("Failed to get BOUNTYHUB_TOKEN: {:?}", err));
        }
    };

    let bountyhub = env::var("BOUNTYHUB_URL").unwrap_or("https://bountyhub.org".to_string());

    Ok(HTTPClient::new(&bountyhub, &pat, env!("CARGO_PKG_VERSION")))
}

/// Job based commands
#[derive(Subcommand, Debug, Clone)]
enum Job {
    /// Job artifact related commands
    #[command(subcommand)]
    Artifact(JobArtifact),

    /// Delete a job
    #[command(name = "delete")]
    #[command(about = "Delete a job")]
    Delete {
        #[arg(short, long, env = "BOUNTYHUB_JOB_ID")]
        #[arg(required = true)]
        job_id: Uuid,
    },
}

impl Job {
    fn run<C>(self, client: C) -> Result<()>
    where
        C: Client,
    {
        match self {
            Job::Delete { job_id } => {
                client
                    .delete_job(job_id)
                    .map_err(|e| format!("failed to delete job: {e:?}"))?;

                Ok(())
            }
            Job::Artifact(artifact) => artifact.run(client),
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum JobArtifact {
    /// Download an artifact uploaded by a job
    #[command(name = "download")]
    #[command(about = "Download a file from the internet")]
    Download {
        #[arg(short, long, env = "BOUNTYHUB_JOB_ID")]
        #[arg(required = true)]
        job_id: Uuid,

        #[arg(short, long, env = "BOUNTYHUB_JOB_ARTIFACT_NAME")]
        #[arg(required = true)]
        artifact_name: String,

        #[arg(short, long, env = "BOUNTYHUB_OUTPUT")]
        #[arg(value_hint = ValueHint::DirPath)]
        output: Option<String>,
    },

    /// Delete job artifact
    #[command(name = "delete")]
    #[command(about = "Delete job artifact")]
    Delete {
        #[arg(short, long, env = "BOUNTYHUB_JOB_ID")]
        #[arg(required = true)]
        job_id: Uuid,

        #[arg(short, long, env = "BOUNTYHUB_JOB_ARTIFACT_NAME")]
        #[arg(required = true)]
        artifact_name: String,
    },
}

impl JobArtifact {
    fn run<C>(self, client: C) -> Result<()>
    where
        C: Client,
    {
        match self {
            JobArtifact::Download {
                job_id,
                artifact_name,
                output,
            } => {
                let output = match output {
                    Some(output) => {
                        let output = PathBuf::from(output);
                        if output.is_dir() {
                            output.join(&artifact_name)
                        } else {
                            output
                        }
                    }
                    None => env::current_dir()
                        .map_err(|err| format!("Failed to get current directory: {err:?}"))?
                        .join(&artifact_name),
                };

                let mut freader = client
                    .download_job_artifact(job_id, &artifact_name)
                    .map_err(|err| format!("Failed to download file: {err:?}"))?;

                let mut fwriter = fs::File::create(output)
                    .map_err(|err| format!("Failed to create file: {err:?}"))?;

                std::io::copy(&mut *freader, &mut fwriter)
                    .map_err(|err| format!("failed to write file: {err:?}"))?;
            }
            JobArtifact::Delete {
                job_id,
                artifact_name,
            } => {
                client
                    .delete_job_artifact(job_id, &artifact_name)
                    .map_err(|err| format!("failed to delete job artifact: {err:?}"))?;
            }
        }
        Ok(())
    }
}

#[derive(Subcommand, Debug, Clone)]
enum Scan {
    /// Dispatch a scan from the latest revision of the workflow
    Dispatch {
        #[arg(short, long, env = "BOUNTYHUB_WORKFLOW_ID", required = true)]
        workflow_id: Uuid,

        #[arg(short, long, env = "BOUNTYHUB_SCAN_NAME", required = true)]
        scan_name: String,

        #[arg(long)]
        input_string: Option<Vec<String>>,

        #[arg(long)]
        input_bool: Option<Vec<String>>,
    },
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

impl Scan {
    fn run<C>(self, client: C) -> Result<()>
    where
        C: Client,
    {
        match self {
            Scan::Dispatch {
                workflow_id,
                scan_name,
                input_string,
                input_bool,
            } => {
                if !validation::valid_scan_name(&scan_name) {
                    return Err(format!("Invalid scan name: '{scan_name}'"));
                }

                let inputs = if input_string.is_some() || input_bool.is_some() {
                    let mut m = BTreeMap::new();

                    if let Some(input_string) = input_string {
                        for v in input_string {
                            let (k, v) = split_input(v.as_str())?;
                            if !validation::valid_workflow_var_key(k) {
                                return Err(format!("Key '{k}' is in invalid format"));
                            }
                            m.insert(k.to_string(), Value::String(v.to_string()));
                        }
                    }

                    if let Some(input_bool) = input_bool {
                        for v in input_bool {
                            let (k, v) = split_input(v.as_str())?;
                            if !validation::valid_workflow_var_key(k) {
                                return Err(format!("Key '{k}' is in invalid format"));
                            }
                            let b = v
                                .parse::<bool>()
                                .map_err(|_| format!("Value '{v}' is not a valid boolean"))?;
                            m.insert(k.to_string(), Value::Bool(b));
                        }
                    }

                    Some(m)
                } else {
                    None
                };

                client
                    .dispatch_scan(workflow_id, scan_name, inputs)
                    .map_err(|e| format!("failed to dispatch scan: {e:?}"))?;

                Ok(())
            }
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
enum Blob {
    /// Download a file from bountyhub.org blob storage
    Download {
        #[arg(short, long, required = true)]
        src: String,
        #[arg(short, long, env = "BOUNTYHUB_OUTPUT")]
        #[arg(value_hint = ValueHint::DirPath)]
        dst: Option<String>,
    },
    /// Upload a file to bountyhub.org blob storage
    Upload {
        /// src is the source file on the local filesystem
        #[arg(short, long, required = true)]
        #[arg(value_hint = ValueHint::DirPath)]
        src: String,

        /// dst is the destination path on bountyhub.org blobs
        #[arg(long, required = true)]
        dst: String,
    },
}

impl Blob {
    fn run<C>(self, client: C) -> Result<()>
    where
        C: Client,
    {
        match self {
            Blob::Download {
                src: path,
                dst: output,
            } => {
                let output = match output {
                    Some(output) => {
                        let output = PathBuf::from(output);
                        if output.is_dir() {
                            output.join(&path)
                        } else {
                            output
                        }
                    }
                    None => env::current_dir()
                        .map_err(|err| format!("Failed to get current directory: {err:?}"))?
                        .join(Path::new(&path).file_name().unwrap_or_default()),
                };

                let mut freader = client
                    .download_blob_file(&path)
                    .map_err(|err| format!("Failed to download file: {err:?}"))?;

                let mut fwriter = fs::File::create(output)
                    .map_err(|err| format!("Failed to create output file: {err:?}"))?;

                std::io::copy(&mut *freader, &mut fwriter)
                    .map_err(|err| format!("Failed to write to output: {err:?}"))?;
                Ok(())
            }
            Blob::Upload { src, dst } => {
                let freader = fs::File::open(&src)
                    .map_err(|err| format!("Failed to open file '{src}': {err:?}"))?;

                client
                    .upload_blob_file(freader, dst.as_str())
                    .map_err(|err| format!("Failed to upload blob file: {err:?}"))?;
                Ok(())
            }
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
enum Runner {
    /// Runner registration commands
    #[command(subcommand)]
    Registration(RunnerRegistration),
}

impl Runner {
    fn run<C>(self, client: C) -> Result<()>
    where
        C: Client,
    {
        match self {
            Runner::Registration(registration) => registration.run(client)?,
        }

        Ok(())
    }
}

#[derive(Subcommand, Debug, Clone)]
enum RunnerRegistration {
    /// Get newly created runner registration token
    #[command(name = "token")]
    Token,

    /// Get runner registration command with newly created token
    #[command(name = "command")]
    Command,
}

impl RunnerRegistration {
    fn run<C>(self, client: C) -> Result<()>
    where
        C: Client,
    {
        match self {
            RunnerRegistration::Token => {
                let resp = client
                    .create_runner_registration()
                    .map_err(|err| format!("Failed to create runner registration: {err:?}"))?;

                print!("{}", resp.token);
            }
            RunnerRegistration::Command => {
                let resp = client
                    .create_runner_registration()
                    .map_err(|err| format!("Failed to create runner registration: {err:?}"))?;

                println!(
                    r#"runner configure --token "{}" --url "{}""#,
                    resp.token, resp.url,
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod job_tests {
    use super::*;
    use crate::client::{Error as ClientError, MockClient};
    use mockall::predicate::*;
    use serde_json::Value;
    use uuid::Uuid;

    #[test]
    fn test_download_failed() {
        let job_id = Uuid::now_v7();
        let artifact_name = "test.zip";

        let cmd = JobArtifact::Download {
            job_id,
            artifact_name: artifact_name.to_string(),
            output: None,
        };
        let mut client = MockClient::new();
        client
            .expect_download_job_artifact()
            .with(eq(job_id), eq(artifact_name))
            .times(1)
            .returning(|_, _| Err(ClientError::Unauthorized));

        let result = cmd.run(client);
        assert!(result.is_err(), "expected error, got ok");
    }

    #[test]
    fn test_delete_job_call() {
        let job_id = Uuid::now_v7();

        let cmd = Job::Delete { job_id };

        let mut client = MockClient::new();
        client
            .expect_delete_job()
            .with(eq(job_id))
            .times(1)
            .returning(|_| Ok(()));

        let result = cmd.run(client);
        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[test]
    fn test_dispatch_call_no_inputs() {
        let revision_id = Uuid::now_v7();
        let cmd = Scan::Dispatch {
            workflow_id: revision_id,
            scan_name: "example".to_string(),
            input_string: None,
            input_bool: None,
        };

        let mut client = MockClient::new();
        client
            .expect_dispatch_scan()
            .with(
                eq(revision_id),
                function(|v| v == "example"),
                function(|v: &Option<BTreeMap<String, Value>>| v.is_none()),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = cmd.run(client);
        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[test]
    fn test_dispatch_call_with_inputs() {
        let revision_id = Uuid::now_v7();
        let cmd = Scan::Dispatch {
            workflow_id: revision_id,
            scan_name: "example".to_string(),
            input_string: Some(vec!["s_key=s_val".to_string()]),
            input_bool: Some(vec!["b_key=true".to_string()]),
        };

        let mut client = MockClient::new();
        client
            .expect_dispatch_scan()
            .with(
                eq(revision_id),
                function(|v| v == "example"),
                function(|v: &Option<BTreeMap<String, Value>>| match v {
                    None => false,
                    Some(input) => {
                        match input.get("s_key").expect("s_key to exist").to_owned() {
                            Value::String(val) => val == *"s_val",
                            _ => return false,
                        };
                        match input.get("b_key").expect("b_key to exist").to_owned() {
                            Value::Bool(val) => val,
                            _ => false,
                        }
                    }
                }),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        let result = cmd.run(client);
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
        let cmd = Blob::Download {
            src: "file.txt".to_string(),
            dst: None,
        };
        let mut client = MockClient::new();
        client
            .expect_download_blob_file()
            .with(function(|v| v == "file.txt"))
            .times(1)
            .returning(|_| Err(ClientError::NotFound));

        let result = cmd.run(client);
        assert!(result.is_err(), "expected error, got ok");
    }
}

#[derive(Subcommand, Debug)]
enum Bhlast {
    #[command(about = "Create a new bhlast server")]
    Create,
}

impl Bhlast {
    fn run<C>(self, client: C) -> Result<()>
    where
        C: Client,
    {
        match self {
            Bhlast::Create => {
                match client.create_bhlast_domain() {
                    Ok(id) => {
                        println!("{}", id);
                    }
                    Err(Error::Forbidden) => {
                        return Err("You cannot create more bhlast domains".to_string());
                    }
                    Err(Error::Unauthorized) => {
                        return Err("Unauthorized: invalid token".to_string());
                    }
                    Err(e) => {
                        return Err(format!("Failed to create bhlast domain: {e:?}"));
                    }
                }

                Ok(())
            }
        }
    }
}

#[derive(Subcommand, Debug)]
enum Md {
    #[command(about = "Generate markdown documentation for the CLI")]
    Docs,
}

impl Md {
    fn run(&self) -> Result<()> {
        match self {
            Md::Docs => {
                clap_markdown::print_help_markdown::<Cli>();
            }
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Completion {
    #[arg(value_enum)]
    shell: Shell,
}

impl Completion {
    fn run(&self) -> Result<()> {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();

        generate(self.shell, &mut cmd, name, &mut io::stdout());

        Ok(())
    }
}

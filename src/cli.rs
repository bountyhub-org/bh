use crate::client::{Client, HTTPClient};
use crate::validation;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::{Shell, generate};
use error_stack::{Context, Report, Result, ResultExt};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fmt, fs, io};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run() -> Result<(), CliError> {
        let cli = Cli::parse();

        match cli.command {
            Some(command) => command.run()?,
            None => {
                Cli::command()
                    .print_help()
                    .change_context(CliError)
                    .attach_printable("Failed to print help")?;
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

    #[command(subcommand)]
    Blob(Blob),

    /// Shell completion commands
    #[command(arg_required_else_help = true)]
    Completion(Completion),
}

impl Commands {
    fn run(self) -> Result<(), CliError> {
        if let Commands::Completion(completion) = self {
            completion.run()?;
            return Ok(());
        }

        let client = new_client()?;
        match self {
            Commands::Completion(_) => unreachable!(),
            Commands::Job(job) => job.run(client)?,
            Commands::Scan(scan) => scan.run(client)?,
            Commands::Blob(blob) => blob.run(client)?,
        }

        Ok(())
    }
}

fn new_client() -> Result<HTTPClient, CliError> {
    let pat = match env::var("BOUNTYHUB_TOKEN") {
        Ok(token) => {
            if !token.starts_with("bhv") {
                return Err(CliError)
                    .attach_printable("Invalid token format")
                    .attach_printable("token does not start with bhv");
            }
            token
        }
        Err(err) => {
            return Err(CliError).attach_printable(format!("Failed to get token: {:?}", err));
        }
    };

    let bountyhub = env::var("BOUNTYHUB_URL").unwrap_or("https://bountyhub.org".to_string());

    Ok(HTTPClient::new(&bountyhub, &pat, env!("CARGO_PKG_VERSION")))
}

/// Job based commands
#[derive(Subcommand, Debug, Clone)]
enum Job {
    #[clap(name = "download")]
    #[clap(about = "Download a file from the internet")]
    Download {
        #[clap(short, long, env = "BOUNTYHUB_PROJECT_ID")]
        #[clap(required = true)]
        project_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_WORKFLOW_ID")]
        #[clap(required = true)]
        workflow_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_REVISION_ID")]
        #[clap(required = true)]
        revision_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_JOB_ID")]
        #[clap(required = true)]
        job_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_OUTPUT")]
        #[arg(value_hint = ValueHint::DirPath)]
        output: Option<String>,
    },

    #[clap(name = "delete")]
    #[clap(about = "Delete a job")]
    Delete {
        #[clap(short, long, env = "BOUNTYHUB_PROJECT_ID")]
        #[clap(required = true)]
        project_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_WORKFLOW_ID")]
        #[clap(required = true)]
        workflow_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_REVISION_ID")]
        #[clap(required = true)]
        revision_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_JOB_ID")]
        #[clap(required = true)]
        job_id: Uuid,
    },
}

impl Job {
    fn run<C>(self, client: C) -> Result<(), CliError>
    where
        C: Client,
    {
        match self {
            Job::Download {
                project_id,
                workflow_id,
                revision_id,
                job_id,
                output,
            } => {
                let output = match output {
                    Some(output) => {
                        let output = PathBuf::from(output);
                        if output.is_dir() {
                            output.join(job_id.to_string())
                        } else {
                            output
                        }
                    }
                    None => env::current_dir()
                        .change_context(CliError)
                        .attach_printable("Failed to get current directory")?
                        .join(job_id.to_string()),
                };

                let mut freader = client
                    .download_job_result_file(project_id, workflow_id, revision_id, job_id)
                    .change_context(CliError)
                    .attach_printable("Failed to download file")?;

                let mut fwriter = fs::File::create(output)
                    .change_context(CliError)
                    .attach_printable("Failed to create file")?;

                std::io::copy(&mut *freader, &mut fwriter)
                    .change_context(CliError)
                    .attach_printable("failed to write file")?;
            }
            Job::Delete {
                project_id,
                workflow_id,
                revision_id,
                job_id,
            } => {
                client
                    .delete_job(project_id, workflow_id, revision_id, job_id)
                    .change_context(CliError)
                    .attach_printable("Failed to delete job")?;
            }
        }

        Ok(())
    }
}

#[derive(Subcommand, Debug, Clone)]
enum Scan {
    Dispatch {
        #[clap(short, long, env = "BOUNTYHUB_PROJECT_ID")]
        #[clap(required = true)]
        project_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_WORKFLOW_ID")]
        #[clap(required = true)]
        workflow_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_REVISION_ID")]
        #[clap(required = true)]
        revision_id: Uuid,

        #[clap(short, long, env = "BOUNTYHUB_SCAN_NAME")]
        #[clap(required = true)]
        scan_name: String,

        #[clap(long)]
        input_string: Option<Vec<String>>,

        #[clap(long)]
        input_bool: Option<Vec<String>>,
    },
}

fn split_input(input: &str) -> Result<(&str, &str), CliError> {
    let split = input.splitn(2, '=');
    let mut k = split.take(2);
    Ok((
        k.next()
            .ok_or(CliError)
            .attach_printable(format!("failed to get the key from string input {input}"))?,
        k.next()
            .ok_or(CliError)
            .attach_printable(format!("failed to get the value from string input {input}"))?,
    ))
}

impl Scan {
    fn run<C>(self, client: C) -> Result<(), CliError>
    where
        C: Client,
    {
        match self {
            Scan::Dispatch {
                project_id,
                workflow_id,
                revision_id,
                scan_name,
                input_string,
                input_bool,
            } => {
                if !validation::valid_scan_name(&scan_name) {
                    return Err(Report::new(CliError).attach_printable("invalid scan name"));
                }

                let inputs = if input_string.is_some() || input_bool.is_some() {
                    let mut m = BTreeMap::new();

                    if let Some(input_string) = input_string {
                        for v in input_string {
                            let (k, v) = split_input(v.as_str())?;
                            if !validation::valid_workflow_var_key(k) {
                                return Err(Report::new(CliError)
                                    .attach_printable(format!("Key {k} is in invalid format")));
                            }
                            m.insert(k.to_string(), Value::String(v.to_string()));
                        }
                    }

                    if let Some(input_bool) = input_bool {
                        for v in input_bool {
                            let (k, v) = split_input(v.as_str())?;
                            if !validation::valid_workflow_var_key(k) {
                                return Err(Report::new(CliError)
                                    .attach_printable(format!("Key {k} is in invalid format")));
                            }
                            let b = v
                                .parse::<bool>()
                                .change_context(CliError)
                                .attach_printable("value is not bool")?;
                            m.insert(k.to_string(), Value::Bool(b));
                        }
                    }

                    Some(m)
                } else {
                    None
                };

                client
                    .dispatch_scan(project_id, workflow_id, revision_id, scan_name, inputs)
                    .change_context(CliError)
                    .attach_printable("Failed to dispatch scan")
            }
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
enum Blob {
    Download {
        #[clap(short, long, required = true)]
        src: String,
        #[clap(short, long, env = "BOUNTYHUB_OUTPUT")]
        #[arg(value_hint = ValueHint::DirPath)]
        dst: Option<String>,
    },
    Upload {
        /// src is the source file on the local filesystem
        #[clap(short, long, required = true)]
        #[arg(value_hint = ValueHint::DirPath)]
        src: String,

        /// dst is the destination path on bountyhub.org blobs
        #[clap(long, required = true)]
        dst: String,
    },
}

impl Blob {
    fn run<C>(self, client: C) -> Result<(), CliError>
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
                        .change_context(CliError)
                        .attach_printable("Failed to get current directory")?
                        .join(Path::new(&path).file_name().unwrap_or_default()),
                };

                let mut freader = client
                    .download_blob_file(&path)
                    .change_context(CliError)
                    .attach_printable("Failed to download file")?;

                let mut fwriter = fs::File::create(output)
                    .change_context(CliError)
                    .attach_printable("Failed to create file")?;

                std::io::copy(&mut *freader, &mut fwriter)
                    .change_context(CliError)
                    .attach_printable("failed to write file")?;
            }
            Blob::Upload { src, dst } => {
                let freader = fs::File::open(&src)
                    .change_context(CliError)
                    .attach_printable(format!("failed to open file '{src}'"))?;

                client
                    .upload_blob_file(freader, dst.as_str())
                    .change_context(CliError)
                    .attach_printable("failed to call upload blob file")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod job_tests {
    use super::*;
    use crate::client::{ClientError, MockClient};
    use error_stack::Report;
    use mockall::predicate::*;
    use serde_json::Value;
    use uuid::Uuid;

    #[test]
    fn test_download_failed() {
        let project_id = Uuid::now_v7();
        let workflow_id = Uuid::now_v7();
        let revision_id = Uuid::now_v7();
        let job_id = Uuid::now_v7();

        let cmd = Job::Download {
            project_id,
            workflow_id,
            revision_id,
            job_id,
            output: None,
        };
        let mut client = MockClient::new();
        client
            .expect_download_job_result_file()
            .with(eq(project_id), eq(workflow_id), eq(revision_id), eq(job_id))
            .times(1)
            .returning(|_, _, _, _| Err(Report::new(ClientError)));

        let result = cmd.run(client);
        assert!(result.is_err(), "expected error, got ok");
    }

    #[test]
    fn test_delete_job_call() {
        let project_id = Uuid::now_v7();
        let workflow_id = Uuid::now_v7();
        let revision_id = Uuid::now_v7();
        let job_id = Uuid::now_v7();

        let cmd = Job::Delete {
            project_id,
            workflow_id,
            revision_id,
            job_id,
        };

        let mut client = MockClient::new();
        client
            .expect_delete_job()
            .with(eq(project_id), eq(workflow_id), eq(revision_id), eq(job_id))
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let result = cmd.run(client);
        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[test]
    fn test_dispatch_call_no_inputs() {
        let project_id = Uuid::now_v7();
        let workflow_id = Uuid::now_v7();
        let revision_id = Uuid::now_v7();
        let cmd = Scan::Dispatch {
            project_id,
            workflow_id,
            revision_id,
            scan_name: "example".to_string(),
            input_string: None,
            input_bool: None,
        };

        let mut client = MockClient::new();
        client
            .expect_dispatch_scan()
            .with(
                eq(project_id),
                eq(workflow_id),
                eq(revision_id),
                function(|v| v == "example"),
                function(|v: &Option<BTreeMap<String, Value>>| v.is_none()),
            )
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        let result = cmd.run(client);
        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[test]
    fn test_dispatch_call_with_inputs() {
        let project_id = Uuid::now_v7();
        let workflow_id = Uuid::now_v7();
        let revision_id = Uuid::now_v7();
        let cmd = Scan::Dispatch {
            project_id,
            workflow_id,
            revision_id,
            scan_name: "example".to_string(),
            input_string: Some(vec!["s_key=s_val".to_string()]),
            input_bool: Some(vec!["b_key=true".to_string()]),
        };

        let mut client = MockClient::new();
        client
            .expect_dispatch_scan()
            .with(
                eq(project_id),
                eq(workflow_id),
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
            .returning(|_, _, _, _, _| Ok(()));

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
            .returning(|_| Err(Report::new(ClientError)));

        let result = cmd.run(client);
        assert!(result.is_err(), "expected error, got ok");
    }
}

#[derive(Args, Debug)]
struct Completion {
    #[arg(value_enum)]
    shell: Shell,
}

impl Completion {
    fn run(&self) -> Result<(), CliError> {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();

        generate(self.shell, &mut cmd, name, &mut io::stdout());

        Ok(())
    }
}

#[derive(Debug)]
pub struct CliError;

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An error occurred while running the CLI")
    }
}

impl Context for CliError {}

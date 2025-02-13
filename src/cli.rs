use crate::client::{Client, HTTPClient};
use clap::Args;
use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::generate;
use clap_complete::Shell;
use error_stack::{Context, Result, ResultExt};
use std::env;
use std::fmt;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

/// The bh is a command line interface for BountyHub API
/// It allows you to interact with the BountyHub API from the command line
/// The CLI requires a token to be set in the BOUNTYHUB_TOKEN environment variable
/// The token should start with `bhv`
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

/// `bh` commands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Job related commands rely on `BOUNTYHUB_TOKEN` and `BOUNTYHUB_URL`
    /// environment variables.
    #[command(subcommand)]
    Job(Job),

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

#[cfg(test)]
mod new_client_tests {
    use super::*;

    fn unset_env() {
        env::remove_var("BOUNTYHUB_TOKEN");
        env::remove_var("BOUNTYHUB_URL");
    }

    #[test]
    fn test_new_client() {
        unset_env();
        assert!(new_client().is_err());

        env::set_var("BOUNTYHUB_TOKEN", "bhv1_1234");
        let client = new_client().expect("Failed to create client");
        assert_eq!(client.authorization(), "Bearer bhv1_1234");
        assert_eq!(client.bountyhub_domain(), "https://bountyhub.org");

        env::set_var("BOUNTYHUB_TOKEN", "bhv1_1234");
        env::set_var("BOUNTYHUB_URL", "https://my-custom-bountyhub.org");
        let client = new_client().expect("Failed to create client");
        assert_eq!(client.authorization(), "Bearer bhv1_1234");
        assert_eq!(client.bountyhub_domain(), "https://my-custom-bountyhub.org");

        env::set_var("BOUNTYHUB_TOKEN", "example");
        assert!(new_client().is_err());
    }
}

/// Job based commands
#[derive(Subcommand, Debug)]
enum Job {
    #[clap(name = "download")]
    #[clap(about = "Download a file from the internet")]
    Download {
        #[clap(short, long)]
        #[clap(required = true)]
        project_id: Uuid,

        #[clap(short, long)]
        #[clap(required = true)]
        workflow_id: Uuid,

        #[clap(short, long)]
        #[clap(required = true)]
        revision_id: Uuid,

        #[clap(short, long)]
        #[clap(required = true)]
        job_id: Uuid,

        #[clap(short, long)]
        #[clap(short, long)]
        #[arg(value_hint = ValueHint::DirPath)]
        output: Option<String>,
    },

    #[clap(name = "delete")]
    #[clap(about = "Delete a job")]
    Delete {
        #[clap(short, long)]
        #[clap(required = true)]
        project_id: Uuid,

        #[clap(short, long)]
        #[clap(required = true)]
        workflow_id: Uuid,

        #[clap(short, long)]
        #[clap(required = true)]
        revision_id: Uuid,

        #[clap(short, long)]
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

                let mut fwriter = std::fs::File::create(output)
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

#[cfg(test)]
mod job_tests {
    use super::*;
    use crate::client::{ClientError, MockClient};
    use error_stack::Report;
    use mockall::predicate::*;
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

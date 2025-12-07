# Command-Line Help for `bh`

This document contains the help content for the `bh` command-line program.

**Command Overview:**

* [`bh`↴](#bh)
* [`bh job`↴](#bh-job)
* [`bh job artifact`↴](#bh-job-artifact)
* [`bh job artifact download`↴](#bh-job-artifact-download)
* [`bh job artifact delete`↴](#bh-job-artifact-delete)
* [`bh job delete`↴](#bh-job-delete)
* [`bh scan`↴](#bh-scan)
* [`bh scan dispatch`↴](#bh-scan-dispatch)
* [`bh blob`↴](#bh-blob)
* [`bh blob download`↴](#bh-blob-download)
* [`bh blob upload`↴](#bh-blob-upload)
* [`bh runner`↴](#bh-runner)
* [`bh runner registration`↴](#bh-runner-registration)
* [`bh runner registration token`↴](#bh-runner-registration-token)
* [`bh runner registration command`↴](#bh-runner-registration-command)
* [`bh md`↴](#bh-md)
* [`bh md docs`↴](#bh-md-docs)
* [`bh completion`↴](#bh-completion)

## `bh`

BountyHub CLI

**Usage:** `bh [COMMAND]`

###### **Subcommands:**

* `job` — Job related commands
* `scan` — Scan related commands
* `blob` — Blob related commands
* `runner` — Runner related commands
* `md` — 
* `completion` — Shell completion commands



## `bh job`

Job related commands

**Usage:** `bh job <COMMAND>`

###### **Subcommands:**

* `artifact` — Job artifact related commands
* `delete` — Delete a job



## `bh job artifact`

Job artifact related commands

**Usage:** `bh job artifact <COMMAND>`

###### **Subcommands:**

* `download` — Download a file from the internet
* `delete` — Delete job artifact



## `bh job artifact download`

Download a file from the internet

**Usage:** `bh job artifact download [OPTIONS] --job-id <JOB_ID> --artifact-name <ARTIFACT_NAME>`

###### **Options:**

* `-j`, `--job-id <JOB_ID>`
* `-a`, `--artifact-name <ARTIFACT_NAME>`
* `-o`, `--output <OUTPUT>`



## `bh job artifact delete`

Delete job artifact

**Usage:** `bh job artifact delete --job-id <JOB_ID> --artifact-name <ARTIFACT_NAME>`

###### **Options:**

* `-j`, `--job-id <JOB_ID>`
* `-a`, `--artifact-name <ARTIFACT_NAME>`



## `bh job delete`

Delete a job

**Usage:** `bh job delete --job-id <JOB_ID>`

###### **Options:**

* `-j`, `--job-id <JOB_ID>`



## `bh scan`

Scan related commands

**Usage:** `bh scan <COMMAND>`

###### **Subcommands:**

* `dispatch` — Dispatch a scan from the latest revision of the workflow



## `bh scan dispatch`

Dispatch a scan from the latest revision of the workflow

**Usage:** `bh scan dispatch [OPTIONS] --workflow-id <WORKFLOW_ID> --scan-name <SCAN_NAME>`

###### **Options:**

* `-w`, `--workflow-id <WORKFLOW_ID>`
* `-s`, `--scan-name <SCAN_NAME>`
* `--input-string <INPUT_STRING>`
* `--input-bool <INPUT_BOOL>`



## `bh blob`

Blob related commands

**Usage:** `bh blob <COMMAND>`

###### **Subcommands:**

* `download` — Download a file from bountyhub.org blob storage
* `upload` — Upload a file to bountyhub.org blob storage



## `bh blob download`

Download a file from bountyhub.org blob storage

**Usage:** `bh blob download [OPTIONS] --src <SRC>`

###### **Options:**

* `-s`, `--src <SRC>`
* `-d`, `--dst <DST>`



## `bh blob upload`

Upload a file to bountyhub.org blob storage

**Usage:** `bh blob upload --src <SRC> --dst <DST>`

###### **Options:**

* `-s`, `--src <SRC>` — src is the source file on the local filesystem
* `--dst <DST>` — dst is the destination path on bountyhub.org blobs



## `bh runner`

Runner related commands

**Usage:** `bh runner <COMMAND>`

###### **Subcommands:**

* `registration` — Runner registration commands



## `bh runner registration`

Runner registration commands

**Usage:** `bh runner registration <COMMAND>`

###### **Subcommands:**

* `token` — Get newly created runner registration token
* `command` — Get runner registration command with newly created token



## `bh runner registration token`

Get newly created runner registration token

**Usage:** `bh runner registration token`



## `bh runner registration command`

Get runner registration command with newly created token

**Usage:** `bh runner registration command`



## `bh md`

**Usage:** `bh md <COMMAND>`

###### **Subcommands:**

* `docs` — Generate markdown documentation for the CLI



## `bh md docs`

Generate markdown documentation for the CLI

**Usage:** `bh md docs`



## `bh completion`

Shell completion commands

**Usage:** `bh completion <SHELL>`

###### **Arguments:**

* `<SHELL>`

  Possible values: `bash`, `elvish`, `fish`, `powershell`, `zsh`




<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>


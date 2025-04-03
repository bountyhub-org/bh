# BountyHub CLI

Usage: bh [COMMAND]

Commands:
  job         Job related commands
  scan        Scan related commands
  blob        Blob related commands
  completion  Shell completion commands
  generate    Generate
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help

  -V, --version
          Print version

## Job related commands

Usage: job <COMMAND>

Commands:
  download  Download a file from the internet
  delete    Delete a job
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help

### Download a file from the internet

Usage: download [OPTIONS] --project-id <PROJECT_ID> --workflow-id <WORKFLOW_ID> --revision-id <REVISION_ID> --job-id <JOB_ID>

Options:
  -p, --project-id <PROJECT_ID>
          [env: BOUNTYHUB_PROJECT_ID=]

  -w, --workflow-id <WORKFLOW_ID>
          [env: BOUNTYHUB_WORKFLOW_ID=]

  -r, --revision-id <REVISION_ID>
          [env: BOUNTYHUB_REVISION_ID=]

  -j, --job-id <JOB_ID>
          [env: BOUNTYHUB_JOB_ID=]

  -o, --output <OUTPUT>
          [env: BOUNTYHUB_OUTPUT=]

  -h, --help
          Print help

### Delete a job

Usage: delete --project-id <PROJECT_ID> --workflow-id <WORKFLOW_ID> --revision-id <REVISION_ID> --job-id <JOB_ID>

Options:
  -p, --project-id <PROJECT_ID>
          [env: BOUNTYHUB_PROJECT_ID=]

  -w, --workflow-id <WORKFLOW_ID>
          [env: BOUNTYHUB_WORKFLOW_ID=]

  -r, --revision-id <REVISION_ID>
          [env: BOUNTYHUB_REVISION_ID=]

  -j, --job-id <JOB_ID>
          [env: BOUNTYHUB_JOB_ID=]

  -h, --help
          Print help

### Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)

## Scan related commands

Usage: scan <COMMAND>

Commands:
  dispatch  
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help

### Usage: dispatch [OPTIONS] --project-id <PROJECT_ID> --workflow-id <WORKFLOW_ID> --revision-id <REVISION_ID> --scan-name <SCAN_NAME>

Options:
  -p, --project-id <PROJECT_ID>
          [env: BOUNTYHUB_PROJECT_ID=]

  -w, --workflow-id <WORKFLOW_ID>
          [env: BOUNTYHUB_WORKFLOW_ID=]

  -r, --revision-id <REVISION_ID>
          [env: BOUNTYHUB_REVISION_ID=]

  -s, --scan-name <SCAN_NAME>
          [env: BOUNTYHUB_SCAN_NAME=]

      --input-string <INPUT_STRING>
          

      --input-bool <INPUT_BOOL>
          

  -h, --help
          Print help

### Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)

## Blob related commands

Usage: blob <COMMAND>

Commands:
  download  
  upload    
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help

### Usage: download [OPTIONS] --path <PATH>

Options:
  -p, --path <PATH>
          

  -o, --output <OUTPUT>
          [env: BOUNTYHUB_OUTPUT=]

  -h, --help
          Print help

### Usage: upload --src <SRC> --dst <DST>

Options:
  -s, --src <SRC>
          src is the source file on the local filesystem

      --dst <DST>
          dst is the destination path on bountyhub.org blobs

  -h, --help
          Print help

### Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)

## Shell completion commands

Usage: completion <SHELL>

Arguments:
  <SHELL>
          [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -h, --help
          Print help

## Generate

Usage: generate <COMMAND>

Commands:
  docs  
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help

### Usage: docs

Options:
  -h, --help
          Print help

### Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)

## Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)


# BountyHub CLI

`bh` is BountyHub CLI tool that allows you to interact with the BountyHub API.
The idea is to provide a way to interact with the API from the terminal.

This is especially important for the BountyHub Runner. It can be used to download the results of previous runs
and use it in your pipeline.

## Quick Links

- [Installation](#installation)
- [Auto completion](#auto-completion)

## Installation

### Install using a release

1. Download the latest release from the [releases page](https://github.com/bountyhub-org/bh/releases)
1. Extract the archive
1. Move the `bh` binary to a directory in your PATH
1. Run `bh --help` to see if the installation was successful

### Install from source

1. Clone the repository
1. Change into the directory
1. Run `cargo install --path .`

## Auto completion

To enable auto-completion, you can run the following command:

If you are not sure which shell you are using, you can run the following command:

```bash
echo $SHELL
```

You can run the following command to see what shells are available:

```bash
bh completion --help
```

### Examples:

#### Bash

```bash
source <(bh completion bash)
```

#### Zsh
```bash
source <(bh completion zsh)
```



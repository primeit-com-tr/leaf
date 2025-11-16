# LEAF
Simple and powerful database deployment tool for `Oracle` Database.

- [LEAF](#leaf)
  - [Features](#features)
  - [Installation](#installation)
    - [Linux/macOS Users](#linuxmacos-users)
    - [Windows Users](#windows-users)
    - [Build from Source](#build-from-source)
  - [Usage](#usage)
    - [Getting Started](#getting-started)
  - [Concepts](#concepts)
    - [Connections](#connections)
      - [Create a connection](#create-a-connection)
    - [Plans](#plans)
    - [Add a plan](#add-a-plan)
    - [Running a plan](#running-a-plan)
    - [Rollback a plan](#rollback-a-plan)
  - [Deployments](#deployments)
  - [Development](#development)
    - [Running Tests](#running-tests)
    - [Hooks](#hooks)
  - [Examples](#examples)
    - [Release](#release)


## Features

- Reflect changes in your source database to your target database
- Keep track of changes and rollback to previous versions
- Supports Oracle Database

## Installation

- Installation is simple and straightforward. Just follow the instructions for your operating system.

### Linux/macOS Users

```bash
# Download the latest release for your operating system

# Linux
curl -L https://github.com/primeit-com-tr/leaf/releases/latest/download/leaf-linux-x86_64 -o leaf

# macOS
curl -L https://github.com/primeit-com-tr/leaf/releases/latest/download/leaf-macos-x86_64 -o leaf

# Make the binary executable
chmod +x leaf

# Optionally, move the binary to a directory in your PATH
sudo mv leaf /usr/local/bin/
```

### Windows Users

```bash
Invoke-WebRequest -Uri "https://github.com/primeit-com-tr/leaf/releases/latest/download/leaf-windows-x86_64.exe" -OutFile "$env:USERPROFILE\leaf.exe"
```
Optionally, move the binary to a directory in your PATH.
```bash
setx PATH "$env:PATH;$env:USERPROFILE"
```

### Build from Source

- Clone the repository
- Install [Rust](https://www.rust-lang.org/tools/install)
- Build the binary

```bash
git clone https://github.com/primeit-com-tr/leaf.git
cd leaf
cargo build --release
```
Check the `target/release` directory for the binary.


## Usage

Before you start, make sure you have [oracle-instant-client](https://www.oracle.com/database/technologies/instant-client/downloads.html) installed.

### Getting Started

Once you have installed the binary, you can start using it by running the `leaf` command.

```bash
# ❯ leaf --help
LEAF - Simple and powerful database deployment tool.
by primeit - https://primeit.com.tr

Usage: leaf [COMMAND]

Commands:
  db           Manage repository database
  connections  Manage connections
  plans        Plan and run database deployments
  deployments  Deployment commands
  deploy       Deploy a plan, alias for `plans run`
  init         Initialize application
  version      Print version
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')
```

First, you need to initialize the application. This will create the database repository and the configuration file.

```bash
❯ leaf init --help
Initialize application

Usage: leaf init <COMMAND>

Commands:
  all   initialize .env file and database using defaults
  env   initialize .env file
  db    initializes database. Run this command after initializing .env file Runs migrations
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

You can just run `leaf init` to initialize the application with default values. `leaf init all` is alias for `leaf init`.
This will create a `dot env` file in the current directory with the following content:

```bash
# === Database configuration ===

# particularly used for migrations, sea-orm will use this value
DATABASE_URL=sqlite://leaf.db?mode=rwc

# Application repository
# Supported databases are: Mysql, Postgres, SQLite
LEAF__DATABASE__URL=sqlite://leaf.db?mode=rwc


# === Logging configuration ===

# Supported values: info, debug, error, trace, warn (default: info)
# LEAF__LOGS__LEVEL=info

# Log directory (default: ./logs)
# LEAF__LOGS__DIR=./logs

# Log format (default: json). Supported values: pretty, json
# LEAF__LOGS__CONSOLE_FORMAT=json

# Enable file logging (default: true)
# LEAF__LOGS__FILE_ENABLED=true

# Set external module log level (default: info)
# To set multiple levels, use comma-separated list (e.g., sqlx:error,hyper:info)
# LEAF__LOGS__EXT_LEVEL=sqlx:error

# === Rules configuration ===

# Exclude object types from the plan.
# Following values will be combined with the values given during plan creation.
# The default list includes the following object types:
# - DATABASE LINK
# - INDEX PARTITION
# - JAVA CLASS
# - JAVA SOURCE
# - JOB
# - LIBRARY
# - SCHEDULE
# - SCHEDULE
# - SYNONYM
# - TABLE PARTITION
# - TABLE SUBPARTITION
#
# LEAF__RULES__EXCLUDE_OBJECT_TYPES=

# Exclude object names from the plan.
# Following values will be combined with the values given during plan creation.
# The default list is empty.
# LEAF__RULES__EXCLUDE_OBJECT_NAMES=

# Disable dropping the following object types from the plan.
# Following values will be combined with the values given during plan creation.
# The default list is empty.
# LEAF__RULES__DISABLED_DROP_TYPES=

# Disable all DROP operations (default: false)
# LEAF__RULES__DISABLE_ALL_DROPS=true


# === Hooks configuration ===

# Hooks are list of scripts that will be executed before/after certain operations.
# Hooks can be jinja templates. and app passes [plan] currently as plan name.

# Pre-prepare-deployment hooks
# eg. LEAF__HOOKS__PRE_PREPARE_DEPLOYMENT="
# begin my_user.kill_processes('{{ plan }}'); end;
# begin my_user.lock_users(); end;
# "
# This will call system.kill_processes(plan) and system.lock_users() before deployment.
# Note that you don't have pass the plan name as a parameter, but it will be passed as a context.
# This way you can create plsql procedures that will be called by hooks.
# LEAF__HOOKS__PRE_PREPARE_DEPLOYMENT=

# Post-prepare-deployment hooks
# LEAF__HOOKS__POST_PREPARE_DEPLOYMENT=

# Pre-apply-deployment hooks
# LEAF__HOOKS__PRE_APPLY_DEPLOYMENT=

# Post-apply-deployment hooks
# LEAF__HOOKS__POST_APPLY_DEPLOYMENT=

# Pre-rollback hooks
# LEAF__HOOKS__PRE_ROLLBACK=

# Post-rollback hooks
# LEAF__HOOKS__POST_ROLLBACK=
```

Also `leaf init` will create the application database in the `sqlite` default database (file `leaf.db` in the current directory).
If you want to use a different database, you can set the `LEAF__DATABASE__URL` and `DATABASE_URL` environment variables.
And run `leaf init db` to create application tables in given database.


For arguments accepting multiple values like `LEAF__RULES__EXCLUDE_OBJECT_TYPES` and `LEAF__RULES__DISABLED_DROP_TYPES`,...
you can pass multiple values by using the following syntax:

```bash
# Arguments with multiple values
LEAF__RULES__EXCLUDE_OBJECT_TYPES="
TYPE_A
TYPE_B
TYPE_C
"
```

## Concepts

There are three main concepts in LEAF:

- 🔌 Connections
- 🧭 Plans
- 🚀 Deployments

Also there is a concept called `Hooks` which is used to execute scripts during the plan creation, apply and rollback processes.
See [Hooks](#hooks) section for more details.

### Connections
Connections are used to connect to your source and target databases. You can create as many connections as you need.

#### Create a connection
To create a connection, use the `connections add` command:

```bash
leaf connections add --help
```

> [!WARNING]
> - The user in source database should have access to dba_objects and dba_tables.
> - The user in target database should have access to dba_objects and dba_tables.
> - Also should be able create/drop objects in target database in target schemas.

You can see all available connection commands with `leaf connections --help`.

### Plans

Plans are used to store variables to calculate the changes in your source database to your target database.
Basically, plan stores source and target connections, schemas to check and other variables to control which objects to include in the plan.


### Add a plan
To add a plan, use the `plans add` command:

> [!NOTE]
> Before adding a plan, you need to create related connections first.

```bash
leaf plans add --help
```

Add command will create a new plan with the given name, source and target connections, and schemas to include in the plan.
This won't run the plan, you need to use the `run` command for that.

- Several parameters in plans commands are combined with the values in `.env` file.
- The `exclude-object-types`, `exclude-object-names` and `disabled-drop-types` parameters are combined with the values in `.env` file.

> [!NOTE]
> For example if you have
> ```bash
> LEAF__RULES__EXCLUDE_OBJECT_TYPES=TABLE,VIEW
> ...
> ```
> and you run `leaf plans add ... --exclude-object-types PROCEDURE,FUNCTION` then the final value will be `TABLE,VIEW,PROCEDURE,FUNCTION`.
- Above rule applies to `exclude-object-names` and `disabled-drop-types` parameters as well.

- `fail-fast` parameter is used to enable fail fast mode. This means if any change fails, deployment will stop and return an error.
Also you can override this setting by passing `--fail-fast` flag to the `run` command.


### Running a plan

To run a plan, use the `plans run` command:

```bash
leaf plans run --help
```

- The `cutoff-date` parameter is used to specify the cutoff date for the plan. If not specified, the last successful deployment start date will be used. For example if the last deployment for `the plan` started at `2023.01.01:00.00.00` and you run `leaf plans run the-plan`
All changes after `2023.01.01:00.00.00` will be applied to the target database.

> [!TIP]
> `--cutoff-date` can be given as date or date time. It will be converted to date time internally.
> For example if you run `leaf plans run the-plan --cutoff-date 2023.01.01` then the final value will be `2023.01.01:00.00.00`.
> You can also specify date time as `2023.01.01:23.59.59` or `2023.01.01:00.00.00:00.00.00`.

> [!INFO]
> You can also use `leaf deploy the-plan` instead of `leaf plans run the-plan`.
> It's an alias for `leaf plans run`.


### Rollback a plan
To rollback a plan, use the `plans rollback` command:

```bash
❯ leaf plans rollback --help
```

> [!NOTE]
> This will only rollback the last SUCCESSFUL deployment for the given plan.

> [!TIP]
> If your plan is stuck in `Running` status, you can use `leaf plans reset` command to reset the status to `Idle`.
> So you can run the plan again. In ideal case, you should not have to use this command. It's just a safety measure.

## Deployments

You can monitor the deployments using the `deployments` command:

```bash
leaf deployments --help
```

Use the `leaf deployments show --help` command to see the available options for the `show` command:


## Development

- Clone the repository
- Install [Rust](https://www.rust-lang.org/tools/install)
- Build the binary

```bash
git clone https://github.com/primeit-com-tr/leaf.git
cd leaf
cargo build --release
```
Check the `target/release` directory for the binary.


### Running Tests

Repository contains testing environment setup using [docker-compose](https://docs.docker.com/compose/).
Before running tests, make sure;

- Run `docker-compose up` in the root directory of the repository
- Make sure [oracle-instant-client](https://www.oracle.com/database/technologies/instant-client/linux-x86-64-downloads.html) is in the `PATH`

```bash
cargo test
```

> [!TIP]
> For manual tests you can generate source and target objects using the files under `scripts/sql` directory.


### Hooks

Hooks are the scripts that are defined in the `.env` file. You can use jinja templates in the hooks.
Hooks are executed in given stub in the `.env` file variable. They are better can be explained with an example.

Suppose you have the following hooks in the `.env` file:

```bash
# .env file

# other variables
...

# Pre-prepare-deployment hooks
LEAF__HOOKS__PRE_PREPARE_DEPLOYMENT="
begin my_user.kill_processes('{{ plan }}'); end;
begin my_user.lock_users(); end;
"

# other variables
...

```
> [!NOTE]
> Hooks are list of values which are delimited by new line like above and must be defined enclosed by double quotes.

Given the above setup in .env file;
- You will have two scripts that will be before the plan is prepared for deployment.
- There scripts are:
  - `begin my_user.kill_processes('{{ plan }}'); end;`
  - `begin my_user.lock_users(); end;`
- LEAF will execute these scripts in on the `target` database before the plan is prepared for deployment.

Which means when you call;

```bash
leaf plans run demo3 ...
# or
leaf deployments prepare --plan demo3 ...
```
The scripts will be executed in the `target` database.

> [!NOTE]
> The reason why `run` calls the `prepare` hooks is because `run` command flow is like this:
> 1. Call `prepare` command
> 2. Call `apply` command

Also notice that `begin my_user.kill_processes('{{ plan }}'); end;` has a variable `{{ plan }}` which will be replaced with the plan name
during the execution of the script. This is provided during the execution time of the related command.


## Examples

```bash

# Add a connection to use for source of the deployment
leaf connections add --name=demo_source --username=system --password=Welcome1 --connection-string="localhost:1521/XEPDB1";

# Add a connection to use for target of the deployment
leaf connections add --name=demo_target --username=system --password=Welcome1 --connection-string="localhost:1522/XEPDB1";

# Add a plan with source and target connections
# This will only sync schemas SCHEMA3 and SCHEMA4 from source to target
leaf plans add --name demo3 --source demo_source --target demo_target --schemas "SCHEMA3,SCHEMA4";

# Run in dry-run mode and collect scripts that will be executed
# This will output two scripts to `./deployment-scripts/output` directory
# - Migration scripts
# - Rollback scripts
# This wont apply any changes to the target database
# Also wont create any deployment plan in the repository
leaf plans run demo3 --dry --collect-scripts --output-path ./.uncommitted/output --cutoff-date 2021.01.01


# Run the plan
# This will create a deployment plan in the repository and apply the changes to the target database immediately
leaf plans run demo3 --cutoff-date 2021.01.01


# Rollback the plan
# This will rollback the last successful deployment for the plan
leaf plans rollback --plan demo3


# Prepare a deployment for the plan
# This will create a deployment plan in the repository and prepare the changes to the target database.
# This will NOT apply the changes to the target database.
leaf deployments prepare --plan demo3 --cutoff-date 2021.01.01


# List all deployments
leaf deployments list

# Apply the deployment
# - Deployment ID is the ID of the deployment plan in the repository
# - This will apply the changes to the target database for the given deployment ID
leaf deployments apply --deployment-id 1
```

### Release

- Update version in `Cargo.toml`

```bash
git tag v[VERSION]
git push origin v[VERSION]
```
Example:
```bash
git tag v0.1.0
git push origin v0.1.0

# And check the github actions tab
```

This will create a new tag and push it to the remote repository.
Which will trigger the release workflow in the `.github/workflows/release.yml` file.
The release workflow will generate binaries for platforms:
- Linux x86_64
- macOS x86_64
- Windows x86_64

> [!NOTE]
> You can also manually run the release workflow by clicking on the `Run workflow` button in the `Actions` tab.


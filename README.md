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
```

Also `leaf init` will create the application database in the `sqlite` default database (file `leaf.db` in the current directory).
If you want to use a different database, you can set the `LEAF__DATABASE__URL` and `DATABASE_URL` environment variables.
And run `leaf init db` to create application tables in given database.


## Concepts

There are three main concepts in LEAF:

- 🔌 Connections
- 🧭 Plans
- 🚀 Deployments

### Connections
Connections are used to connect to your source and target databases. You can create as many connections as you need.

#### Create a connection
To create a connection, use the `connections create` command:

```bash
# ❯ leaf connections add --help

Create a new connection

Usage: leaf connections add --name <NAME> --username <USERNAME> --password <PASSWORD> --connection-string <CONNECTION_STRING>

Options:
      --name <NAME>                            Connection name (unique) eg. my-source-db
      --username <USERNAME>                    Username
      --password <PASSWORD>                    Password
      --connection-string <CONNECTION_STRING>  Connection string eg. localhost:1521/XEPDB1
  -h, --help
```

Example:
```bash
leaf connections add \
    --name my-source-db \
    --username system \
    --password Welcome1
    --connection-string localhost:1521/XEPDB1
```

> [!WARNING]
> - The user in source database should have access to dba_objects and dba_tables.
> - The user in target database should have access to dba_objects and dba_tables.
> - Also should be able create/drop objects in target database in target schemas.

You can see all available connection commands with `leaf connections --help`.

```bash
# ❯ leaf connections --help
Manage connections

Usage: leaf connections <COMMAND>

Commands:
  add     Create a new connection
  test    Test a connection
  ping    Same as test, but for saved connections
  remove  Delete a connection
  prune   Remove all connections
  list    List all connections
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```


### Plans

Plans are used to store variables to calculate the changes in your source database to your target database.
Basically, plan stores source and target connections, schemas to check and other variables to control which objects to include in the plan.

```bash
# ❯ leaf plans --help
Plan and run database deployments

Usage: leaf plans <COMMAND>

Commands:
  add       Add a plan
  list      List plans, schemas, excluded object types
  remove    Remove a plan
  prune     Remove all plans
  reset     Reset plan status to IDLE
  run       Run a plan
  rollback  Rollback a plan
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### Add a plan
To add a plan, use the `plans add` command:

> [!NOTE]
> Before adding a plan, you need to create related connections first.

```bash
# ❯ leaf plans add --help
Add a plan

Usage: leaf plans add [OPTIONS] --name <NAME> --source <SOURCE> --target <TARGET> --schemas <SCHEMAS>

Options:
      --name <NAME>
          Name of the plan, unique, case insensitive
      --source <SOURCE>
          Source connection name
      --target <TARGET>
          Target connection name
      --schemas <SCHEMAS>
          Comma-separated list of schemas to include in the plan
      --exclude-object-types <EXCLUDE_OBJECT_TYPES>
          Comma-separated list of object types to exclude from the plan
      --exclude-object-names <EXCLUDE_OBJECT_NAMES>
          Comma-separated list of object names to exclude from the plan
      --disabled-drop-types <DISABLED_DROP_TYPES>
          Comma-separated list of disabled object types do drop (e.g., TABLE, VIEW, PROCEDURE, FUNCTION, TRIGGER, etc.)
      --fail-fast
          Fail fast mode
  -h, --help
          Print help
```

This will create a new plan with the given name, source and target connections, and schemas to include in the plan.
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
# ❯ leaf plans run --help
Run a plan

Usage: leaf plans run [OPTIONS] <NAME>

Arguments:
  <NAME>
          Plan name, case insensitive

Options:
      --cutoff-date <CUTOFF_DATE>
          Cutoff date — deploy everything changed after this date.
          If not specified, the last successful deployment start date will be used.
          If not found, then the app will exit.

          Example formats:
          - 2023.01.01
          - 2023.01.01:00.00.00
          - 2023.01.01:23.59.59

      --fail-fast <FAIL_FAST>
          Fail fast mode

          [possible values: true, false]

  -d, --dry
          Dry run mode, this will not apply changes to the database

  -s, --show-report
          Show report after running the plan

  -h, --help
          Print help (see a summary with '-h')
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
Rollback a plan

Usage: leaf plans rollback --plan <PLAN>

Options:
  -p, --plan <PLAN>
  -h, --help         Print help
```

> [!NOTE]
> This will only rollback the last deployment for the given plan.

> [!TIP]
> If your plan is stuck in `Running` status, you can use `leaf plans reset` command to reset the status to `Idle`.
> So you can run the plan again. In ideal case, you should not have to use this command. It's just a safety measure.

## Deployments

You can monitor the deployments using the `deployments` command:

```bash
# ❯ leaf deployments --help
Deployment commands

Usage: leaf deployments <COMMAND>

Commands:
  list  List deployments
  show
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
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


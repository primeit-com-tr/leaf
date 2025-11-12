use clap::Args;
use clap::crate_version;

use crate::config::Settings;

#[derive(Args, Debug)]
pub struct VersionCommand;

pub async fn execute(action: &VersionCommand, _: &Settings) {
    match action {
        VersionCommand {} => print_version().await,
    }
}

pub async fn print_version() {
    println!("LEAF version: {}", crate_version!());
}

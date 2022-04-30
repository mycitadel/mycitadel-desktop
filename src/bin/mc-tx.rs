// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

#[macro_use]
extern crate clap;
#[macro_use]
extern crate amplify;

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use colored::Colorize;
use mycitadel::model::{file, FileDocument, Wallet};

/// Command-line arguments
#[derive(Parser)]
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[clap(
    author,
    version,
    name = "mc-tx",
    about = "Command-line tool for working with MyCitadel wallets"
)]
pub struct Args {
    /// Command to execute
    #[clap(subcommand)]
    pub command: Command,
}

/// Wallet command to execute
#[derive(Subcommand)]
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum Command {
    DumpWallet {
        /// Wallet *.mcw file
        wallet: PathBuf,
    },

    ConstructWallet {
        /// Wallet YAML file
        source: PathBuf,

        /// Wallet *.mcw file
        destination: PathBuf,
    },
}

#[derive(Debug, Display, Error, From)]
#[display(inner)]
pub enum Error {
    #[from]
    File(file::Error),

    #[from]
    Yaml(serde_yaml::Error),
}

impl Args {
    pub fn exec(self) -> Result<(), Error> {
        match &self.command {
            Command::DumpWallet { wallet } => {
                let wallet = Wallet::read_file(wallet)?;
                println!("{}", serde_yaml::to_string(&wallet)?);
            }
            Command::ConstructWallet {
                source,
                destination,
            } => {
                let wallet: Wallet =
                    serde_yaml::from_reader(fs::File::open(source).map_err(file::Error::File)?)?;
                wallet.write_file(destination)?;
            }
        }
        Ok(())
    }
}

fn main() {
    let args = Args::parse();
    if let Err(err) = args.exec() {
        eprintln!("{}: {}\n", "Error".bright_red(), err);
    }
}

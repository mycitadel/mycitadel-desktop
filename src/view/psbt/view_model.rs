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

use bitcoin::consensus::Encodable;
use bitcoin::psbt::PartiallySignedTransaction;
use std::path::PathBuf;
use std::{fs, io};

use wallet::psbt::Psbt;

#[derive(Getters, Default)]
pub struct ViewModel {
    psbt: Psbt,
    path: PathBuf,
}

impl ViewModel {
    pub fn with(psbt: Psbt, path: PathBuf) -> ViewModel {
        ViewModel { psbt, path }
    }

    pub fn save(&mut self) -> Result<usize, io::Error> {
        let psbt = PartiallySignedTransaction::from(self.psbt.clone());
        let file = fs::File::create(&self.path)?;
        psbt.consensus_encode(file)
    }
}

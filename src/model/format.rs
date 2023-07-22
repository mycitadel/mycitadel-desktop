// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime SA, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use bpro::{OnchainStatus, OnchainTxid};

pub trait FormatDate {
    fn format_date(&self) -> String;
}

impl FormatDate for OnchainTxid {
    fn format_date(&self) -> String {
        match self.status {
            OnchainStatus::Blockchain(height) => self
                .date_time()
                .map(|dt| dt.format("%F %H:%M").to_string())
                .unwrap_or_else(|| format!("{height}")),
            OnchainStatus::Mempool => s!("mempool"),
        }
    }
}

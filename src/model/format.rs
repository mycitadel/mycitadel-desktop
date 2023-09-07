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
use gtk::prelude::LabelExt;
use gtk::Label;

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

pub fn display_accounting_amount(
    amount: u64,
    precision: impl Into<u8>,
    label1: &Label,
    label2: &Label,
    label3: &Label,
) {
    let precision = precision.into();
    let pow = 10u64.pow(precision as u32);
    let int = amount / pow;
    let fract = amount - int * pow;
    let f = format!("{:01$}", fract, precision as usize);
    let remain = f.trim_start_matches('0');
    let zeros = precision as usize - remain.len();

    match (int, fract) {
        (0, _) => {
            label1.set_text(&format!("0.{:01$}", 0, zeros));
            label2.set_text(remain);
            label3.set_text("");
        }
        (_, 0) => {
            label1.set_text("");
            label2.set_text(&format!("{}", int));
            label3.set_text(".0");
        }
        (_, _) => {
            label1.set_text("");
            label2.set_text(&format!("{}.{:0<2$}", int, remain, zeros));
            label3.set_text("");
        }
    }
}

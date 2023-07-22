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
) {
    let precision = precision.into();
    let pow = 10u64.pow(precision as u32);
    let int = amount / pow;
    let fract = amount - int * pow;
    let remain = format!("{fract}").trim_end_matches('0').to_string();
    let zeros = precision as usize - remain.len();

    let main = if int == 0 {
        label1.set_text(&format!("0.{:01$}", "", zeros));
        remain
    } else if fract != 0 {
        label1.set_text("");
        format!("{}.{}", int, remain)
    } else {
        label1.set_text("");
        format!("{}", int)
    };
    label2.set_text(&main);
}

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

pub(super) mod beneficiary_row;
mod widget;

pub(super) use widget::Widgets;

use ::wallet::psbt;
use bitcoin::util::address;
use gtk::ResponseType;

use crate::view::pay::beneficiary_row::AmountError;

#[derive(Msg)]
pub enum Msg {
    Show,
    BeneficiaryAdd,
    BeneficiaryRemove,
    BeneficiaryEdit(u32),
    SelectBeneficiary(u32),
    FeeChange,
    FeeSetBlocks(u8),
    Response(ResponseType),
}

#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// Available wallet funds are insufficient to cover the transaction
    InsufficientFunds,

    /// One or more of beneficiaries has incorrect address (please see exclamation marks next to the addresses).
    #[from(address::Error)]
    Address,

    /// One or more of payment amounts are invalid (please see exclamation marks next to the addresses).
    #[from(AmountError)]
    Amount,

    /// Internal error (wallet descriptor inconsistency)
    #[from]
    Miniscript(miniscript::Error),

    /// Internal error (PSBT constructor inconsistency)
    #[from]
    PsbtConstruct(psbt::construct::Error),

    /// Unable to compute proper fee
    FeeFailure,
}

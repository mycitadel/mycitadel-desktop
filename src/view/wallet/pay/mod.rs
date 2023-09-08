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

pub(super) mod beneficiary_row;
mod widget;

use ::wallet::psbt;
use bitcoin::util::address;
use gtk::ResponseType;
pub(super) use widget::Widgets;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[display(doc_comments)]
pub enum FeeRate {
    /// In one block
    OneBlock,
    /// In two blocks
    TwoBlocks,
    /// In three blocks
    ThreeBlocks,
    /// Unknown
    Unknown,
}

#[derive(Msg)]
pub enum Msg {
    Show,
    BeneficiaryAdd,
    BeneficiaryRemove,
    BeneficiaryEdit(u32),
    SelectBeneficiary(u32),
    FeeSet,
    FeeSetBlocks(FeeRate),
    Response(ResponseType),
}

#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// Available wallet funds are insufficient to cover the transaction
    InsufficientFunds,

    /// One or more of beneficiaries has incorrect address (please see exclamation marks next to
    /// the addresses).
    #[from(address::Error)]
    Address,

    /// Please add at least one beneficiary.
    NoBeneficiaries,

    /// One or more of payment amounts are zeros.
    Amount,

    /// Internal error in descriptor; please provide the developer with the following details:
    ///
    /// {0}
    #[from]
    Miniscript(miniscript::Error),

    /// Internal error in PSBT; please provide the developer with the following details:
    ///
    /// {0}
    #[from]
    PsbtConstruct(psbt::construct::Error),

    /// Unable to compute proper fee.
    FeeFailure,

    /// The wallet lacks funds to pay for the transaction fees.
    NoFundsForFee,

    /// Multiple outputs have flag "MAX" set.
    MultipleMaxOutputs,
}

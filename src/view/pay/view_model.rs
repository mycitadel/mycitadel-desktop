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

use super::beneficiary_row::BeneficiaryModel;
use crate::model::{Wallet, WalletSettings};

#[derive(Getters)]
pub struct ViewModel {
    #[getter(skip)]
    wallet: Wallet,
    #[getter(as_mut)]
    beneficiaries: BeneficiaryModel,
    #[getter(as_copy)]
    fee_rate: f32,
}

impl ViewModel {
    pub fn with(wallet: Wallet) -> ViewModel {
        ViewModel {
            fee_rate: wallet.ephemerals().fees.0,
            beneficiaries: BeneficiaryModel::new(),
            wallet,
        }
    }

    pub fn as_wallet(&self) -> &Wallet {
        &self.wallet
    }
    pub fn as_settings(&self) -> &WalletSettings {
        self.wallet.as_settings()
    }
    pub fn to_settings(&self) -> WalletSettings {
        self.wallet.to_settings()
    }
}

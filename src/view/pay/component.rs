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

use ::wallet::psbt::Psbt;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{Dialog, ResponseType};
use relm::{Relm, StreamHandle, Update, Widget};

use super::{Msg, ViewModel, Widgets};
use crate::model::Wallet;
use crate::view::pay::beneficiary_row::Beneficiary;
use crate::view::wallet;

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
    wallet_stream: Option<StreamHandle<wallet::Msg>>,
}

impl Component {
    pub fn compose_psbt(&self) -> Option<Psbt> {
        for no in 0..self.model.beneficiaries().n_items() {
            let item = self
                .model
                .beneficiaries()
                .item(no)
                .expect("BeneficiaryModel is broken");
        }
        None
    }
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = Wallet;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, wallet: Self::ModelParam) -> Self::Model {
        ViewModel::with(wallet)
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => {
                self.model.beneficiaries_mut().clear();
                self.model.beneficiaries_mut().append(&Beneficiary::new());
                self.widgets.init_ui(&self.model);
                self.widgets.show();
            }
            Msg::BeneficiaryAdd => {
                self.model.beneficiaries_mut().append(&Beneficiary::new());
            }
            Msg::BeneficiaryRemove => {
                self.widgets.selected_beneficiary_index().map(|index| {
                    self.model.beneficiaries_mut().remove(index);
                });
            }
            Msg::SelectBeneficiary(index) => self.widgets.select_beneficiary(index),
            Msg::BeneficiaryEdit(index) => {
                self.widgets.select_beneficiary(index);
                /* Check correctness of the model data */
            }
            Msg::FeeChange => { /* Update fee and total tx amount */ }
            Msg::FeeSetBlocks(_) => { /* Update fee and total tx amount */ }
            Msg::Response(ResponseType::Ok) => {
                self.widgets.hide();
            }
            Msg::Response(ResponseType::Cancel) => {
                self.widgets.hide();
            }
            Msg::Response(_) => {}
            Msg::SetWallet(stream) => {
                self.wallet_stream = Some(stream);
            }
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.to_root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("pay.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.connect(relm);
        widgets.bind_beneficiary_model(relm, &model);
        widgets.init_ui(&model);

        Component {
            model,
            widgets,
            wallet_stream: None,
        }
    }
}

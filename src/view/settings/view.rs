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

use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use bitcoin::util::bip32::ExtendedPubKey;
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    glib, Adjustment, Button, Dialog, Entry, Image, Label, ListBox, ListStore, TextBuffer,
    ToggleButton, ToolButton, TreeView,
};
use miniscript::Descriptor;
use relm::{init, Channel, Component, Relm, Update, Widget};
use wallet::hd::{SegmentIndexes, TrackingAccount};

use super::{spending_row, Msg, ViewModel};
use crate::model::{DescriptorClass, Signer};
use crate::view::devices;
use crate::view::settings::spending_row::Condition;

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    save_btn: Button,
    cancel_btn: Button,

    devices_btn: ToolButton,
    addsign_btn: ToolButton,
    removesign_btn: ToolButton,
    signers_tree: TreeView,
    signers_store: ListStore,

    spending_list: ListBox,
    addcond_btn: ToolButton,
    removecond_btn: ToolButton,

    name_fld: Entry,
    fingerprint_fld: Entry,
    xpub_fld: Entry,
    account_adj: Adjustment,
    accfp_fld: Entry,
    derivation_fld: Entry,
    device_lbl: Label,
    device_img: Image,
    device_status_img: Image,
    seed_mine_tgl: ToggleButton,
    seed_extern_tgl: ToggleButton,

    descriptor_buf: TextBuffer,
    descr_legacy_tgl: ToggleButton,
    descr_segwit_tgl: ToggleButton,
    descr_nested_tgl: ToggleButton,
    descr_taproot_tgl: ToggleButton,
    export_core_tgl: ToggleButton,
    export_lnpbp_tgl: ToggleButton,
}

impl Widgets {
    pub fn update_signer_details(&self, details: Option<(&Signer, TrackingAccount)>) {
        if let Some((signer, ref derivation)) = details {
            self.name_fld.set_text(&signer.name);
            self.fingerprint_fld
                .set_text(&signer.fingerprint.to_string());
            self.xpub_fld.set_text(&signer.xpub.to_string());
            self.account_adj
                .set_value(signer.account.first_index() as f64);
            self.accfp_fld
                .set_text(&signer.xpub.fingerprint().to_string());
            self.derivation_fld.set_text(&derivation.to_string());
        }
        if let Some((device, model)) =
            details.and_then(|(s, _)| s.device.as_ref().map(|d| (d, &s.name)))
        {
            self.device_img.set_visible(true);
            self.device_status_img.set_visible(true);
            self.device_lbl.set_text(&format!("{} ({})", device, model));
        } else {
            self.device_img.set_visible(false);
            self.device_status_img.set_visible(false);
            self.device_lbl.set_text("Unknown");
        }
    }

    pub fn update_signers(&mut self, signers: &BTreeSet<Signer>) {
        let store = &mut self.signers_store;
        store.clear();
        for signer in signers {
            store.insert_with_values(
                None,
                &[
                    (0, &signer.name),
                    (1, &signer.fingerprint.to_string()),
                    (2, &signer.account.to_string()),
                    (3, &signer.xpub.to_string()),
                    (4, &signer.device.clone().unwrap_or_default()),
                ],
            );
        }
    }

    pub fn update_descriptor(
        &mut self,
        descriptor: Option<&Descriptor<TrackingAccount>>,
        format: bool,
    ) {
        let text = match (descriptor, format) {
            (Some(descriptor), false) => format!("{:#}", descriptor),
            (Some(descriptor), true) => format!("{}", descriptor),
            (None, _) => s!(""),
        };
        self.descriptor_buf.set_text(&text);
    }

    fn descr_class_toggle(&self, class: DescriptorClass) -> &ToggleButton {
        match class {
            DescriptorClass::PreSegwit => &self.descr_legacy_tgl,
            DescriptorClass::SegwitV0 => &self.descr_segwit_tgl,
            DescriptorClass::NestedV0 => &self.descr_nested_tgl,
            DescriptorClass::TaprootC0 => &self.descr_taproot_tgl,
        }
    }

    pub fn should_update_descr_class(&mut self, class: DescriptorClass) -> bool {
        self.descr_class_toggle(class).is_active()
    }

    pub fn update_descr_class(&mut self, class: DescriptorClass) {
        self.descr_legacy_tgl
            .set_active(class == DescriptorClass::PreSegwit);
        self.descr_segwit_tgl
            .set_active(class == DescriptorClass::SegwitV0);
        self.descr_nested_tgl
            .set_active(class == DescriptorClass::NestedV0);
        self.descr_taproot_tgl
            .set_active(class == DescriptorClass::TaprootC0);
    }
}

pub struct Win {
    model: ViewModel,
    origin_model: Option<Arc<Mutex<ViewModel>>>,
    widgets: Widgets,
    devices_win: Component<devices::Win>,
}

impl Win {
    fn update_descriptor(&mut self) {
        self.model.update_descriptor();
        self.widgets
            .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
    }

    fn condition_selection_change(&mut self) {
        let removable = self.widgets.spending_list.selected_row().is_some()
            && self.model.spendings.n_items() > 1;
        self.widgets.removecond_btn.set_sensitive(removable);
    }
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = Arc<Mutex<ViewModel>>;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        relm.stream().emit(Msg::Init(model.clone()));
        model.lock().expect("wallet model locked").deref().clone()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Show => self.widgets.dialog.show(),
            Msg::Init(origin_model) => {
                self.origin_model = Some(origin_model);
            }
            Msg::Devices => {
                self.devices_win.emit(devices::Msg::Show);
            }
            Msg::AddDevice(fingerprint, device) => {
                self.model.devices.insert(fingerprint, device);
                self.model.update_signers();
                self.widgets.update_signers(&self.model.signers);
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
            }
            Msg::SignerSelect => {
                let signer = self
                    .widgets
                    .signers_tree
                    .selection()
                    .selected()
                    .map(|(list_model, iter)| list_model.value(&iter, 3))
                    .as_ref()
                    .map(glib::Value::get::<String>)
                    .transpose()
                    .expect("unable to get xpub value from tree column")
                    .as_deref()
                    .map(ExtendedPubKey::from_str)
                    .transpose()
                    .expect("invalid signer xpub")
                    .and_then(|xpub| self.model.signer_by(xpub));
                self.widgets
                    .update_signer_details(signer.map(|s| (s, self.model.derivation_for(s))));
            }
            Msg::ToggleDescr(class) => {
                if self.widgets.should_update_descr_class(class)
                    && self.model.toggle_descr_class(class)
                {
                    self.widgets.update_descr_class(self.model.class);
                    self.widgets
                        .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
                }
            }
            Msg::ExportFormat(lnpbp) => {
                self.model.format_lnpbp = lnpbp;
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
            }
            Msg::Save => {
                self.origin_model.as_ref().map(|model| {
                    *(model.lock().expect("wallet model locked").deref_mut()) = self.model.clone();
                });
                self.widgets.dialog.hide();
            }
            Msg::Cancel => {
                self.widgets.dialog.hide();
            }
            Msg::SpendingConditionAdd => {
                self.model.spendings.append(&Condition::default());
                self.condition_selection_change();
                self.update_descriptor();
            }
            Msg::SpendingConditionRemove => {
                let index = if let Some(row) = self.widgets.spending_list.selected_row() {
                    row.index()
                } else {
                    return;
                };
                self.model.spendings.remove(index as u32);
                self.update_descriptor();
            }
            Msg::SpendingConditionSelect => {
                self.condition_selection_change();
            }
            Msg::SpendingConditionChange => {
                self.update_descriptor();
            }
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.dialog.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("settings.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let stream = relm.stream().clone();
        let (_channel, sender) = Channel::new(move |msg| {
            stream.emit(msg);
        });

        let devices_win = init::<devices::Win>((model.scheme.clone(), model.network, sender))
            .expect("error in devices dialog");

        connect!(relm, widgets.save_btn, connect_clicked(_), Msg::Save);
        connect!(relm, widgets.cancel_btn, connect_clicked(_), Msg::Cancel);
        connect!(relm, widgets.devices_btn, connect_clicked(_), Msg::Devices);

        connect!(
            relm,
            widgets.signers_tree,
            connect_cursor_changed(_),
            Msg::SignerSelect
        );

        connect!(
            relm,
            widgets.export_core_tgl,
            connect_toggled(_),
            Msg::ExportFormat(false)
        );
        connect!(
            relm,
            widgets.export_lnpbp_tgl,
            connect_toggled(_),
            Msg::ExportFormat(true)
        );

        connect!(
            relm,
            widgets.descr_legacy_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::PreSegwit)
        );
        connect!(
            relm,
            widgets.descr_segwit_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::SegwitV0)
        );
        connect!(
            relm,
            widgets.descr_nested_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::NestedV0)
        );
        connect!(
            relm,
            widgets.descr_taproot_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::TaprootC0)
        );

        connect!(
            relm,
            widgets.addcond_btn,
            connect_clicked(_),
            Msg::SpendingConditionAdd
        );
        connect!(
            relm,
            widgets.removecond_btn,
            connect_clicked(_),
            Msg::SpendingConditionRemove
        );
        connect!(
            relm,
            widgets.spending_list,
            connect_selected_rows_changed(_),
            Msg::SpendingConditionSelect
        );

        let stream = relm.stream().clone();
        widgets
            .spending_list
            .bind_model(Some(&model.spendings), move |item| {
                spending_row::RowWidgets::init(stream.clone(), item)
            });

        Win {
            model,
            widgets,
            devices_win,
            origin_model: None,
        }
    }
}

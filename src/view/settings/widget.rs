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

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::str::FromStr;

use bitcoin::util::bip32::ExtendedPubKey;
use bpro::{
    DerivationStandardExt, DerivationType, ElectrumPreset, ElectrumSec, OriginFormat, Ownership,
    Requirement, Signer, WalletTemplate,
};
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    gdk, glib, Adjustment, Box, Button, ButtonBox, ComboBoxText, Dialog, Entry, Grid, HeaderBar,
    Image, Label, ListBox, ListBoxRow, ListStore, Notebook, ResponseType, SpinButton, Spinner,
    TextBuffer, ToggleButton, ToolButton, Toolbar, TreePath, TreeView,
};
use miniscript::Descriptor;
use relm::{Relm, Sender};
use wallet::descriptors::DescriptorClass;
use wallet::hd::{Bip43, DerivationAccount, DerivationStandard, HardenedIndex, SegmentIndexes};
use wallet::onchain::PublicNetwork;

use super::spending_row::SpendingModel;
use super::{spending_row, ElectrumModel, Msg, ViewModel};
use crate::view::NotificationBoxExt;

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    header_bar: HeaderBar,
    save_btn: Button,
    cancel_btn: Button,
    pages: Notebook,

    msg_box: Box,
    msg_lbl: Label,
    msg_img: Image,

    devices_btn: ToolButton,
    addsign_btn: ToolButton,
    removesign_btn: ToolButton,
    signers_tree: TreeView,
    signers_store: ListStore,
    signers_tb: Toolbar,

    spending_box: Box,
    spending_list: ListBox,
    spending_buf: TextBuffer,
    addcond_btn: ToolButton,
    removecond_btn: ToolButton,

    signer_grid: Grid,
    name_fld: Entry,
    fingerprint_fld: Entry,
    path_cmb: ComboBoxText,
    path_fld: Entry,
    xpub_fld: Entry,
    account_stp: SpinButton,
    account_adj: Adjustment,
    accfp_fld: Entry,
    derivation_fld: Entry,
    device_lbl: Label,
    device_img: Image,
    device_status_img: Image,
    seed_mine_tgl: ToggleButton,
    seed_extern_tgl: ToggleButton,

    descriptor_buf: TextBuffer,
    descriptor_box: ButtonBox,
    derivation_box: Box,
    descr_legacy_tgl: ToggleButton,
    descr_segwit_tgl: ToggleButton,
    descr_nested_tgl: ToggleButton,
    descr_taproot_tgl: ToggleButton,

    network_box: ButtonBox,
    mainnet_tgl: ToggleButton,
    testnet_tgl: ToggleButton,
    signet_tgl: ToggleButton,
    export_core_tgl: ToggleButton,
    export_lnpbp_tgl: ToggleButton,
    electr_blockstream_tgl: ToggleButton,
    electr_mycitadel_tgl: ToggleButton,
    electr_custom_tgl: ToggleButton,
    tor_tgl: ToggleButton,
    tls_tgl: ToggleButton,
    nosec_tgl: ToggleButton,
    electrum_fld: Entry,
    port_stp: SpinButton,
    port_adj: Adjustment,
    test_btn: Button,
    connection_img: Image,
    connection_spin: Spinner,
}

impl Widgets {
    pub fn reset_ui(&mut self, model: &ViewModel) {
        self.hide_message();

        self.header_bar
            .set_subtitle(model.path().file_name().and_then(OsStr::to_str));
        self.save_btn.set_label(if model.is_new_wallet() {
            "Create"
        } else {
            "Save"
        });

        // New wallet
        if let Some(ref template) = model.template {
            self.update_template(template);
            self.update_signer_details(None, template.network, template.bip43());
            self.pages.set_page(0);
        } else {
            let new_wallet = model.is_new_wallet();
            self.signers_tb.set_sensitive(new_wallet);
            self.spending_box.set_sensitive(new_wallet);
            self.derivation_box.set_sensitive(new_wallet);
            self.descriptor_box.set_sensitive(new_wallet);

            self.network_box.set_sensitive(new_wallet);
            self.spending_buf
                .set_text("Spending conditions can't be edited for an initialized wallet");

            // Disable already used classes
            let multiclass = model.support_multiclass;
            for class in &model.descriptor_classes {
                match class {
                    DescriptorClass::PreSegwit => self
                        .descr_legacy_tgl
                        .set_sensitive(new_wallet || multiclass),
                    DescriptorClass::SegwitV0 => self
                        .descr_segwit_tgl
                        .set_sensitive(new_wallet || multiclass),
                    DescriptorClass::NestedV0 => self
                        .descr_nested_tgl
                        .set_sensitive(new_wallet || multiclass),
                    DescriptorClass::TaprootC0 => self
                        .descr_taproot_tgl
                        .set_sensitive(new_wallet || multiclass),
                }
            }
        }

        self.mainnet_tgl
            .set_active(model.network == PublicNetwork::Mainnet);
        self.testnet_tgl
            .set_active(model.network == PublicNetwork::Testnet);
        self.signet_tgl
            .set_active(model.network == PublicNetwork::Signet);

        self.update_electrum(&mut model.electrum_model.clone(), true, true);
        self.update_network();

        self.update_signers(&model.signers);
        self.update_signer_details(None, model.network, model.bip43());
        self.update_descr_classes(&model.descriptor_classes);
        self.update_descriptor(model.descriptor.as_ref(), model.export_lnpbp);

        self.dialog.show();
    }

    pub fn hide(&self) { self.dialog.hide() }
    pub fn close(&self) { self.dialog.close() }

    pub fn to_root(&self) -> Dialog { self.dialog.clone() }
    pub fn as_root(&self) -> &Dialog { &self.dialog }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.devices_btn, connect_clicked(_), Msg::AddDevices);
        connect!(relm, self.addsign_btn, connect_clicked(_), Msg::AddReadOnly);
        connect!(
            relm,
            self.removesign_btn,
            connect_clicked(_),
            Msg::RemoveSigner
        );

        connect!(
            relm,
            self.path_fld,
            connect_changed(_),
            Msg::SignerOriginUpdate
        );
        connect!(
            relm,
            self.fingerprint_fld,
            connect_changed(_),
            Msg::SignerFingerprintChange
        );
        connect!(
            relm,
            self.name_fld,
            connect_changed(_),
            Msg::SignerNameChange
        );
        connect!(
            relm,
            self.seed_mine_tgl,
            connect_clicked(_),
            Msg::ExportFormat(false)
        );
        connect!(
            relm,
            self.seed_extern_tgl,
            connect_clicked(_),
            Msg::SignerOwnershipChange
        );
        connect!(
            relm,
            self.account_adj,
            connect_value_changed(_),
            Msg::SignerAccountChange
        );

        connect!(
            relm,
            self.signers_tree,
            connect_cursor_changed(_),
            Msg::SignerSelect
        );

        connect!(
            relm,
            self.mainnet_tgl,
            connect_clicked(_),
            Msg::NetworkChange(PublicNetwork::Mainnet)
        );
        connect!(
            relm,
            self.testnet_tgl,
            connect_clicked(_),
            Msg::NetworkChange(PublicNetwork::Testnet)
        );
        connect!(
            relm,
            self.signet_tgl,
            connect_clicked(_),
            Msg::NetworkChange(PublicNetwork::Signet)
        );

        connect!(
            relm,
            self.export_core_tgl,
            connect_clicked(_),
            Msg::ExportFormat(false)
        );
        connect!(
            relm,
            self.export_lnpbp_tgl,
            connect_clicked(_),
            Msg::ExportFormat(true)
        );

        connect!(
            relm,
            self.descr_legacy_tgl,
            connect_clicked(_),
            Msg::ToggleClass(DescriptorClass::PreSegwit)
        );
        connect!(
            relm,
            self.descr_segwit_tgl,
            connect_clicked(_),
            Msg::ToggleClass(DescriptorClass::SegwitV0)
        );
        connect!(
            relm,
            self.descr_nested_tgl,
            connect_clicked(_),
            Msg::ToggleClass(DescriptorClass::NestedV0)
        );
        connect!(
            relm,
            self.descr_taproot_tgl,
            connect_clicked(_),
            Msg::ToggleClass(DescriptorClass::TaprootC0)
        );

        connect!(
            relm,
            self.addcond_btn,
            connect_clicked(_),
            Msg::ConditionAdd
        );
        connect!(
            relm,
            self.removecond_btn,
            connect_clicked(_),
            Msg::ConditionRemove
        );
        connect!(
            relm,
            self.spending_list,
            connect_selected_rows_changed(_),
            Msg::ConditionSelect
        );

        self.spending_list.connect_row_activated(|list, row| {
            list.select_row(Some(row));
        });

        for entry in [
            &self.fingerprint_fld,
            &self.name_fld,
            &self.derivation_fld,
            &self.xpub_fld,
            &self.accfp_fld,
        ] {
            entry.connect_icon_press(|entry, _, _| {
                let val = entry.text();
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&val);
            });
        }

        connect!(
            relm,
            self.electr_blockstream_tgl,
            connect_clicked(_),
            Msg::ElectrumSelect(ElectrumPreset::Blockstream)
        );
        connect!(
            relm,
            self.electr_mycitadel_tgl,
            connect_clicked(_),
            Msg::ElectrumSelect(ElectrumPreset::MyCitadel)
        );
        connect!(
            relm,
            self.electr_custom_tgl,
            connect_clicked(_),
            Msg::ElectrumSelect(ElectrumPreset::Custom)
        );

        connect!(
            relm,
            self.tor_tgl,
            connect_clicked(_),
            Msg::ElectrumSecChange(ElectrumSec::Tor)
        );
        connect!(
            relm,
            self.tls_tgl,
            connect_clicked(_),
            Msg::ElectrumSecChange(ElectrumSec::Tls)
        );
        connect!(
            relm,
            self.nosec_tgl,
            connect_clicked(_),
            Msg::ElectrumSecChange(ElectrumSec::None)
        );

        connect!(
            relm,
            self.electrum_fld,
            connect_changed(_),
            Msg::ElectrumEdit
        );
        connect!(
            relm,
            self.port_adj,
            connect_value_changed(_),
            Msg::ElectrumPortChange
        );
        connect!(relm, self.test_btn, connect_clicked(_), Msg::ElectrumTest);

        connect!(
            relm,
            self.dialog,
            connect_response(_, resp),
            Msg::Response(resp)
        );
        self.save_btn.set_sensitive(false);
        self.dialog.set_response_sensitive(ResponseType::Ok, false);

        connect!(
            relm,
            self.dialog,
            connect_delete_event(_, _),
            return (None, Inhibit(true))
        );
    }

    pub(super) fn bind_spending_model(&self, sender: Sender<()>, model: &SpendingModel) {
        self.spending_list.bind_model(Some(model), move |item| {
            spending_row::RowWidgets::init(sender.clone(), item)
        });
    }

    pub fn signer_fingerprint(&self) -> String { self.fingerprint_fld.text().to_string() }

    pub fn signer_name(&self) -> String { self.name_fld.text().to_string() }

    pub fn signer_origin(&self) -> String {
        self.path_fld
            .text()
            .replace("*h", &format!("{}", self.signer_account()))
            .to_string()
    }

    pub fn signer_account(&self) -> HardenedIndex {
        HardenedIndex::from_index(self.account_adj.value() as u32)
            .expect("account adjustment max value exceeded")
    }

    pub fn signer_ownership(&self) -> Ownership {
        if self.seed_mine_tgl.is_active() {
            Ownership::Mine
        } else {
            Ownership::External
        }
    }

    pub fn network(&self) -> PublicNetwork {
        match (
            self.mainnet_tgl.is_active(),
            self.testnet_tgl.is_active(),
            self.signet_tgl.is_active(),
        ) {
            (true, false, false) => PublicNetwork::Mainnet,
            (_, true, false) => PublicNetwork::Testnet,
            (_, _, true) => PublicNetwork::Signet,
            _ => unreachable!("inconsistent network togglers state"),
        }
    }

    pub fn update_network(&self) {
        let network = self.network();
        self.devices_btn.set_tooltip_text(if network.is_testnet() {
            Some("Hardware signers can be only used on mainnet")
        } else {
            None
        });
    }

    pub fn electrum_server(&self) -> String { self.electrum_fld.text().to_string() }

    pub fn electrum_port(&self) -> u16 { self.port_adj.value() as u16 }

    pub fn update_electrum(
        &self,
        model: &mut ElectrumModel,
        update_preset: bool,
        update_sec: bool,
    ) {
        if update_preset {
            self.electr_mycitadel_tgl
                .set_active(model.electrum_preset == ElectrumPreset::MyCitadel);
            self.electr_blockstream_tgl
                .set_active(model.electrum_preset == ElectrumPreset::Blockstream);
            self.electr_custom_tgl
                .set_active(model.electrum_preset == ElectrumPreset::Custom);
        }
        if model.electrum_preset != ElectrumPreset::Custom {
            model.electrum_server = model.electrum_preset.to_string();
            model.electrum_port = model
                .electrum_preset
                .electrum_port(model.electrum_sec, self.network());
        }
        if update_sec {
            self.tor_tgl
                .set_active(model.electrum_sec == ElectrumSec::Tor);
            self.tls_tgl
                .set_active(model.electrum_sec == ElectrumSec::Tls);
            self.nosec_tgl
                .set_active(model.electrum_sec == ElectrumSec::None);
        }
        self.electrum_fld.set_text(&model.electrum_server);
        self.port_adj.set_value(model.electrum_port as f64);
        self.connection_img.set_icon_name(None);
        self.electrum_fld
            .set_sensitive(model.electrum_preset == ElectrumPreset::Custom);
        self.port_stp
            .set_sensitive(model.electrum_preset == ElectrumPreset::Custom);
    }

    pub fn start_electrum_test(&self) {
        self.connection_spin.set_visible(true);
        self.connection_spin.set_active(true);
        self.connection_img.set_visible(false);
        self.test_btn.set_sensitive(false);
    }

    pub fn complete_electrum_test(&self, err: Option<String>) {
        self.connection_spin.set_visible(false);
        if let Some(err) = err {
            self.connection_img
                .set_icon_name(Some("emblem-important-symbolic"));
            self.connection_img.set_tooltip_text(Some(&err));
        } else {
            self.connection_img
                .set_icon_name(Some("emblem-default-symbolic"));
            self.connection_img.set_tooltip_text(None);
        }
        self.connection_img.set_visible(true);
        self.test_btn.set_sensitive(true);
    }

    fn update_derivation(&self, format: &DerivationType, network: PublicNetwork) {
        match format {
            DerivationType::LnpBp(_) => {
                self.descr_legacy_tgl.set_active(true);
                self.descr_segwit_tgl.set_active(true);
                self.descr_nested_tgl.set_active(true);
                self.descr_taproot_tgl.set_active(true);
            }
            DerivationType::Bip43(ref bip43) => {
                let class = bip43
                    .descriptor_class()
                    .unwrap_or(DescriptorClass::SegwitV0);
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

        self.mainnet_tgl
            .set_active(network == PublicNetwork::Mainnet);
        self.testnet_tgl
            .set_active(network == PublicNetwork::Testnet);
        self.signet_tgl.set_active(network == PublicNetwork::Signet);
    }

    fn update_template(&self, template: &WalletTemplate) {
        // TODO: Update widgets to match new descriptor or descriptor template
        self.devices_btn
            .set_visible(template.hardware_req != Requirement::Deny);
        self.addsign_btn
            .set_visible(template.watch_only_req != Requirement::Deny);
        self.spending_list
            .set_sensitive(template.max_signer_count != Some(1));
        self.addcond_btn
            .set_visible(template.max_signer_count != Some(1));
        self.removecond_btn
            .set_visible(template.max_signer_count != Some(1));
        self.spending_buf
            .set_text(if template.max_signer_count == Some(1) {
                "Single-sig wallets always can be spent with a single signature and does not allow \
                 to customize spending conditions."
            } else {
                "Each row means alternative spending condition.\nIf all of the requirements at \
                 least in a single row are satisfied, than the funds from this wallet may be spent."
            });
    }

    pub fn set_remove_condition(&self, allow: bool) { self.removecond_btn.set_sensitive(allow) }

    pub fn selected_condition_index(&self) -> Option<i32> {
        self.spending_list
            .selected_row()
            .as_ref()
            .map(ListBoxRow::index)
    }

    pub fn selected_signer_xpub(&self) -> Option<ExtendedPubKey> {
        self.signers_tree
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
    }

    pub fn update_signer_details(
        &self,
        details: Option<(&Signer, DerivationAccount)>,
        network: PublicNetwork,
        standard: Bip43,
    ) {
        let signer = details.as_ref().map(|d| d.0);

        self.removesign_btn.set_sensitive(details.is_some());
        self.signer_grid.set_sensitive(details.is_some());

        self.name_fld.set_editable(true);
        self.name_fld
            .set_text(&signer.map(|s| s.name.clone()).unwrap_or_default());
        self.fingerprint_fld
            .set_text(&signer.map(|s| s.master_fp.to_string()).unwrap_or_default());
        self.xpub_fld
            .set_text(&signer.map(|s| s.xpub.to_string()).unwrap_or_default());
        self.accfp_fld.set_text(
            &signer
                .map(|s| s.xpub.fingerprint().to_string())
                .unwrap_or_default(),
        );

        if let Some((signer, device)) = signer.and_then(|s| s.device.as_ref().map(|d| (s, d))) {
            self.seed_mine_tgl.set_sensitive(false);
            self.seed_extern_tgl.set_sensitive(false);
            self.fingerprint_fld.set_editable(false);
            self.device_img.set_visible(true);
            self.device_status_img.set_visible(true);
            self.device_lbl.set_visible(true);
            self.device_lbl
                .set_text(&format!("{} ({})", device, signer.name));
        } else {
            self.device_img.set_visible(false);
            self.device_status_img.set_visible(false);
            self.device_lbl.set_visible(false);
            self.device_lbl.set_text("none / unknown");
        }

        if let Some((signer, ref derivation)) = details {
            let origin_format = signer.origin_format(network);

            gtk::prelude::ComboBoxTextExt::remove(&self.path_cmb, 3);
            gtk::prelude::ComboBoxTextExt::remove(&self.path_cmb, 2);
            self.path_cmb.append(
                Some("purpose"),
                &standard.account_template_string(network.into()),
            );

            self.account_stp.set_visible(true);
            self.account_stp.set_sensitive(false);
            self.path_fld.set_sensitive(false);
            self.path_cmb.set_sensitive(false);
            let active_id = match origin_format {
                OriginFormat::Master => Some("master"),
                OriginFormat::SubMaster(_) => Some("account"),
                OriginFormat::Standard(..) => Some("purpose"),
                OriginFormat::Custom(ref path) => {
                    self.account_stp.set_visible(false);
                    self.path_fld.set_sensitive(true);
                    self.path_cmb.set_sensitive(true);
                    self.path_cmb.append(
                        Some("custom"),
                        format!("{:#}", path).trim_start_matches("m/"),
                    );
                    Some("custom")
                }
                OriginFormat::CustomAccount(ref path) => {
                    self.account_stp.set_sensitive(true);
                    self.path_fld.set_sensitive(true);
                    self.path_cmb.set_sensitive(true);
                    self.path_cmb.append(Some("custom"), &format!("{:#}", path));
                    Some("custom")
                }
            };
            self.path_cmb.set_active_id(active_id);
            if let Some(account) = signer.account {
                self.account_adj.set_value(account.first_index() as f64);
            } else {
                self.account_adj.set_value(0.0);
            }

            self.update_signer_derivation(derivation);
            self.seed_mine_tgl
                .set_active(signer.ownership == Ownership::Mine);
            self.seed_extern_tgl
                .set_active(signer.ownership == Ownership::External);

            if signer.device.is_none() {
                self.seed_mine_tgl.set_sensitive(true);
                self.seed_extern_tgl.set_sensitive(true);
                self.fingerprint_fld
                    .set_editable(signer.xpub.depth > 1 || signer.master_fp == zero!());
                self.device_lbl.set_visible(true);
            }
        } else {
            self.account_adj.set_value(0.0);
            self.path_fld.set_text("");
            self.derivation_fld.set_text("");
        }
    }

    pub fn update_signer_derivation(&self, derivation: &DerivationAccount) {
        self.derivation_fld.set_text(&derivation.to_string());
    }

    pub fn update_signers(&mut self, signers: &Vec<Signer>) {
        let store = &mut self.signers_store;
        store.clear();
        for signer in signers {
            store.insert_with_values(None, &[
                (0, &signer.name),
                (1, &signer.master_fp.to_string()),
                (2, &signer.account_string()),
                (3, &signer.xpub.to_string()),
                (4, &signer.device.clone().unwrap_or_default()),
            ]);
        }
    }

    pub fn replace_signer(&mut self, signer: &Signer) -> bool {
        if let Some((_, item)) = self.signers_tree.selection().selected() {
            self.signers_store.set(&item, &[
                (0, &signer.name),
                (1, &signer.master_fp.to_string()),
                (2, &signer.account_string()),
                (3, &signer.xpub.to_string()),
                (4, &signer.device.clone().unwrap_or_default()),
            ]);
            true
        } else {
            false
        }
    }

    pub fn remove_signer(&mut self) -> Option<usize> {
        let selection = self
            .signers_tree
            .selection()
            .selected()
            .map(|(_, iter)| iter);
        if let Some(selection) = selection {
            let index = self
                .signers_store
                .path(&selection)
                .as_ref()
                .map(TreePath::indices)
                .and_then(|indicies| indicies.first().map(|index| *index as usize));
            self.signers_store.remove(&selection);
            index
        } else {
            None
        }
    }

    pub fn update_descriptor(
        &mut self,
        descriptor: Option<&Descriptor<DerivationAccount>>,
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

    pub fn update_descr_classes(&mut self, classes: &BTreeSet<DescriptorClass>) {
        self.descr_legacy_tgl
            .set_active(classes.contains(&DescriptorClass::PreSegwit));
        self.descr_segwit_tgl
            .set_active(classes.contains(&DescriptorClass::SegwitV0));
        self.descr_nested_tgl
            .set_active(classes.contains(&DescriptorClass::NestedV0));
        self.descr_taproot_tgl
            .set_active(classes.contains(&DescriptorClass::TaprootC0));
    }
}

impl NotificationBoxExt for Widgets {
    fn notification_box(&self) -> &Box { &self.msg_box }
    fn main_dialog(&self) -> &Dialog { &self.dialog }
    fn main_action_button(&self) -> &Button { &self.save_btn }
    fn notification_image(&self) -> &Image { &self.msg_img }
    fn notification_label(&self) -> &Label { &self.msg_lbl }
}

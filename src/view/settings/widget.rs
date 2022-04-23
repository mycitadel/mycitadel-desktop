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

use bitcoin::util::bip32::ExtendedPubKey;
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    gdk, glib, Adjustment, Box, Button, ButtonsType, ComboBoxText, Dialog, DialogFlags, Entry,
    Grid, HeaderBar, Image, Label, ListBox, ListBoxRow, ListStore, MessageDialog, MessageType,
    Notebook, ResponseType, SpinButton, TextBuffer, ToggleButton, ToolButton, TreeView,
};
use miniscript::Descriptor;
use relm::Relm;
use wallet::hd::{DerivationStandard, SegmentIndexes, TrackingAccount};

use super::Msg;
use crate::model::{
    DerivationStandardExt, DescriptorClass, OriginFormat, Ownership, PublicNetwork, Requirement,
    Signer, WalletStandard, WalletTemplate,
};
use crate::view::settings::spending_row;
use crate::view::settings::spending_row::SpendingModel;

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
    descr_legacy_tgl: ToggleButton,
    descr_segwit_tgl: ToggleButton,
    descr_nested_tgl: ToggleButton,
    descr_taproot_tgl: ToggleButton,
    mainnet_tgl: ToggleButton,
    testnet_tgl: ToggleButton,
    signet_tgl: ToggleButton,
    export_core_tgl: ToggleButton,
    export_lnpbp_tgl: ToggleButton,
}

impl Widgets {
    pub fn reinit_ui(&self, template: &Option<WalletTemplate>, path: &Path) {
        self.header_bar
            .set_subtitle(Some(&path.display().to_string()));
        self.set_new_wallet_mode(template.is_some());
        // New wallet
        if let Some(template) = template {
            self.update_template(template);
            self.update_derivation(&template.format, template.network);
            self.update_signer_details(None, template.network);
            self.pages.set_page(0);
        }
        self.dialog.show();
    }

    pub fn hide(&self) {
        self.dialog.hide()
    }

    pub fn close(&self) {
        self.dialog.close()
    }

    pub(super) fn to_root(&self) -> Dialog {
        self.dialog.clone()
    }

    pub fn show_notification(&self) {
        self.msg_box.show_all();
    }

    pub fn error_dlg(&self, title: &str, message: &str, details: Option<&str>) {
        let err_dlg = MessageDialog::new(
            Some(&self.dialog),
            DialogFlags::all(),
            MessageType::Error,
            ButtonsType::Close,
            message,
        );
        err_dlg.set_title(title);
        err_dlg.set_secondary_text(details);
        err_dlg.run();
        err_dlg.close();
    }

    pub fn show_error(&self, msg: &str) {
        self.dialog.set_response_sensitive(ResponseType::Ok, false);
        self.msg_img.set_icon_name(Some("dialog-error-symbolic"));
        self.msg_lbl.set_label(msg);
        self.msg_box.show_all();
    }

    pub fn show_info(&self, msg: &str) {
        self.dialog.set_response_sensitive(ResponseType::Ok, true);
        self.msg_img
            .set_icon_name(Some("dialog-information-symbolic"));
        self.msg_lbl.set_label(msg);
        self.msg_box.show_all();
    }

    pub fn show_warning(&self, msg: &str) {
        self.dialog.set_response_sensitive(ResponseType::Ok, true);
        self.msg_img.set_icon_name(Some("dialog-warning-symbolic"));
        self.msg_lbl.set_label(msg);
        self.msg_box.show_all();
    }

    pub fn hide_message(&self) {
        self.dialog.set_response_sensitive(ResponseType::Ok, true);
        self.msg_box.hide()
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.save_btn, connect_clicked(_), Msg::Apply);
        connect!(relm, self.cancel_btn, connect_clicked(_), Msg::Close);

        connect!(relm, self.devices_btn, connect_clicked(_), Msg::AddDevices);
        connect!(relm, self.addsign_btn, connect_clicked(_), Msg::AddReadOnly);

        connect!(
            relm,
            self.path_cmb,
            connect_active_id_notify(_),
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
            connect_toggled(_),
            Msg::ExportFormat(false)
        );
        connect!(
            relm,
            self.seed_extern_tgl,
            connect_toggled(_),
            Msg::SignerOwnershipChange
        );

        connect!(
            relm,
            self.signers_tree,
            connect_cursor_changed(_),
            Msg::SignerSelect
        );

        connect!(
            relm,
            self.export_core_tgl,
            connect_toggled(_),
            Msg::ExportFormat(false)
        );
        connect!(
            relm,
            self.export_lnpbp_tgl,
            connect_toggled(_),
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
            self.dialog,
            connect_delete_event(_, _),
            return (None, Inhibit(true))
        );
    }

    pub(super) fn bind_spending_model(&self, relm: &Relm<super::Component>, model: &SpendingModel) {
        let stream = relm.stream().clone();
        self.spending_list.bind_model(Some(model), move |item| {
            spending_row::RowWidgets::init(stream.clone(), item)
        });
    }

    pub fn signer_fingerprint(&self) -> String {
        self.fingerprint_fld.text().to_string()
    }

    pub fn signer_name(&self) -> String {
        self.name_fld.text().to_string()
    }

    pub fn signer_ownership(&self) -> Ownership {
        if self.seed_mine_tgl.is_active() {
            Ownership::Mine
        } else {
            Ownership::External
        }
    }

    fn update_derivation(&self, format: &WalletStandard, network: PublicNetwork) {
        match format {
            WalletStandard::LnpBp(_) => {
                self.descr_legacy_tgl.set_active(true);
                self.descr_segwit_tgl.set_active(true);
                self.descr_nested_tgl.set_active(true);
                self.descr_taproot_tgl.set_active(true);
            }
            WalletStandard::Bip43(ref bip43) => {
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
            "Single-sig wallets always can be spent with a single signature and does not allow to customize spending conditions."
        } else {
            "Each row means alternative spending condition.\nIf all of the requirements at least in a single row are satisfied, than the funds from this wallet may be spent."
        });
    }

    fn set_new_wallet_mode(&self, new_wallet: bool) {
        self.save_btn
            .set_label(if new_wallet { "Create" } else { "Save" });
        // TODO: Disable UI for non-new model
    }

    pub fn set_remove_condition(&self, allow: bool) {
        self.removecond_btn.set_sensitive(allow)
    }

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
        details: Option<(&Signer, TrackingAccount)>,
        network: PublicNetwork,
    ) {
        self.removesign_btn.set_sensitive(details.is_some());
        self.signer_grid.set_sensitive(details.is_some());
        if let Some((signer, ref derivation)) = details {
            self.name_fld.set_text(&signer.name);
            self.fingerprint_fld
                .set_text(&signer.fingerprint.to_string());
            self.xpub_fld.set_text(&signer.xpub.to_string());

            let origin_format = signer.origin_format(network);
            gtk::prelude::ComboBoxTextExt::remove(&self.path_cmb, 2);
            self.account_stp.set_visible(true);
            self.account_stp.set_sensitive(false);
            self.path_fld.set_sensitive(false);
            self.path_cmb.set_sensitive(false);
            let active_id = match origin_format {
                OriginFormat::Master => Some("master"),
                OriginFormat::SubMaster(_) => Some("account"),
                OriginFormat::Standard(ref schema, _, network) => {
                    self.path_cmb.append(
                        Some("purpose"),
                        &schema.account_template_string(network.into()),
                    );
                    Some("purpose")
                }
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

            self.accfp_fld
                .set_text(&signer.xpub.fingerprint().to_string());
            self.derivation_fld.set_text(&derivation.to_string());
            self.seed_mine_tgl
                .set_active(signer.ownership == Ownership::Mine);
            self.seed_extern_tgl
                .set_active(signer.ownership == Ownership::External);

            if let Some(ref device) = signer.device {
                self.seed_mine_tgl.set_sensitive(false);
                self.seed_extern_tgl.set_sensitive(false);
                self.name_fld.set_editable(false);
                self.fingerprint_fld.set_editable(false);
                self.device_img.set_visible(true);
                self.device_status_img.set_visible(true);
                self.device_lbl
                    .set_text(&format!("{} ({})", device, signer.name));
            } else {
                self.seed_mine_tgl.set_sensitive(true);
                self.seed_extern_tgl.set_sensitive(true);
                self.name_fld.set_editable(true);
                self.fingerprint_fld
                    .set_editable(origin_format.master_fingerprint_editable());
                self.device_img.set_visible(false);
                self.device_status_img.set_visible(false);
                self.device_lbl.set_text("none / unknown");
            }
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
                    (2, &signer.account_string()),
                    (3, &signer.xpub.to_string()),
                    (4, &signer.device.clone().unwrap_or_default()),
                ],
            );
        }
    }

    pub fn replace_signer(&mut self, signer: &Signer) -> bool {
        if let Some((_, item)) = self.signers_tree.selection().selected() {
            self.signers_store.set(
                &item,
                &[
                    (0, &signer.name),
                    (1, &signer.fingerprint.to_string()),
                    (2, &signer.account_string()),
                    (3, &signer.xpub.to_string()),
                    (4, &signer.device.clone().unwrap_or_default()),
                ],
            );
            true
        } else {
            false
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

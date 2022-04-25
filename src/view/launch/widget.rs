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

use crate::model::{PublicNetwork, Requirement, WalletTemplate};
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{Adjustment, ApplicationWindow, ListBox, RecentChooserWidget, Switch};
use relm::Relm;
use std::path::PathBuf;

use super::Msg;

#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    hwcount_adj: Adjustment,
    taproot_swch: Switch,
    testnet_swch: Switch,
    rgb_swch: Switch,
    create_box: ListBox,
    import_box: ListBox,
    open_box: ListBox,
    recent: RecentChooserWidget,
}

impl Widgets {
    pub fn show(&self) {
        self.window.show()
    }

    pub fn hide(&self) {
        self.window.hide()
    }

    pub fn to_root(&self) -> ApplicationWindow {
        self.window.clone()
    }
    pub fn as_root(&self) -> &ApplicationWindow {
        &self.window
    }

    fn is_taproot(&self) -> bool {
        self.taproot_swch.is_active()
    }

    fn network(&self) -> PublicNetwork {
        match self.testnet_swch.is_active() {
            true => PublicNetwork::Testnet,
            false => PublicNetwork::Mainnet,
        }
    }

    pub fn selected_template(&self) -> Option<WalletTemplate> {
        let index = if let Some(row) = self.create_box.selected_row() {
            row.index()
        } else {
            return None;
        };
        let taproot = self.is_taproot();
        let network = self.network();
        match index {
            0 => Some(WalletTemplate::singlesig(taproot, network, false)),
            1 => Some(WalletTemplate::singlesig(taproot, network, true)),
            2 => Some(WalletTemplate::hodling(
                network,
                4,
                Requirement::Allow,
                Requirement::Allow,
            )),
            3 => {
                let count = self.hwcount_adj.value() as u16;
                Some(WalletTemplate::multisig(
                    network,
                    Some(count),
                    Requirement::Require,
                    Requirement::Deny,
                ))
            }
            4 => Some(WalletTemplate::multisig(
                network,
                None,
                Requirement::Allow,
                Requirement::Require,
            )),
            5 => Some(WalletTemplate::multisig(
                network,
                None,
                Requirement::Allow,
                Requirement::Allow,
            )),
            6 => None,
            _ => unreachable!("unknown template"),
        }
    }

    pub fn selected_recent(&self) -> Option<PathBuf> {
        self.recent
            .current_uri()
            .map(String::from)
            .map(|s| s.trim_start_matches("file://").to_owned())
            .map(PathBuf::from)
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(
            relm,
            self.create_box,
            connect_row_selected(_, _),
            Msg::TemplateSelected
        );
        connect!(
            relm,
            self.import_box,
            connect_row_selected(_, _),
            Msg::ImportSelected
        );
        connect!(
            relm,
            self.open_box,
            connect_row_selected(_, _),
            Msg::OpenSelected
        );
        connect!(
            relm,
            self.recent,
            connect_item_activated(_),
            Msg::RecentSelected
        );
        connect!(
            relm,
            self.window,
            connect_delete_event(_, _),
            return (Some(Msg::Close), Inhibit(true))
        );
    }
}

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

pub mod devices;
pub mod invoice;
pub mod launch;
pub mod psbt;
pub mod settings;
pub mod tx;
pub mod wallet;

use gtk::prelude::*;

pub fn error_dlg(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    message: &str,
    details: Option<&str>,
) {
    use gtk::{ButtonsType, DialogFlags, MessageDialog, MessageType};

    let err_dlg = MessageDialog::new(
        Some(parent),
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

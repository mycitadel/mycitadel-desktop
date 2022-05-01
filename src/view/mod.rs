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

pub mod about;
pub mod devices;
pub mod launch;
pub mod psbt;
pub mod settings;
pub mod wallet;

pub const APP_ICON: &[u8] = include_bytes!("../../res/applogo-big.png");
pub const APP_ICON_TOOL: &[u8] = include_bytes!("../../res/applogo.png");

use std::path::PathBuf;

use gtk::prelude::*;
use gtk::{
    ButtonsType, DialogFlags, FileChooserAction, FileChooserDialog, FileFilter, MessageDialog,
    MessageType, ResponseType,
};

pub trait NotificationBoxExt {
    fn notification_box(&self) -> &gtk::Box;
    fn main_dialog(&self) -> &gtk::Dialog;
    fn main_action_button(&self) -> &gtk::Button;
    fn notification_image(&self) -> &gtk::Image;
    fn notification_label(&self) -> &gtk::Label;

    fn show_notification(&self) { self.notification_box().show_all(); }
    fn show_error(&self, msg: &str) {
        self.main_dialog()
            .set_response_sensitive(ResponseType::Ok, false);
        self.main_action_button().set_sensitive(false);
        self.notification_image()
            .set_icon_name(Some("dialog-error-symbolic"));
        self.notification_label().set_label(msg);
        self.notification_box().show_all();
    }
    fn show_info(&self, msg: &str) {
        self.main_dialog()
            .set_response_sensitive(ResponseType::Ok, true);
        self.main_action_button().set_sensitive(true);
        self.notification_image()
            .set_icon_name(Some("dialog-information-symbolic"));
        self.notification_label().set_label(msg);
        self.notification_box().show_all();
    }
    fn show_warning(&self, msg: &str) {
        self.main_dialog()
            .set_response_sensitive(ResponseType::Ok, true);
        self.main_action_button().set_sensitive(true);
        self.notification_image()
            .set_icon_name(Some("dialog-warning-symbolic"));
        self.notification_label().set_label(msg);
        self.notification_box().show_all();
    }
    fn hide_message(&self) {
        self.main_dialog()
            .set_response_sensitive(ResponseType::Ok, true);
        self.main_action_button().set_sensitive(true);
        self.notification_box().hide()
    }
}

pub fn msg_dlg(
    parent: &impl IsA<gtk::Window>,
    ty: MessageType,
    title: &str,
    message: &str,
    details: Option<&str>,
) {
    let err_dlg = MessageDialog::new(
        Some(parent),
        DialogFlags::all(),
        MessageType::Error,
        ButtonsType::Close,
        message,
    );
    err_dlg.set_title(title);
    err_dlg.set_message_type(ty);
    err_dlg.set_secondary_text(details);
    err_dlg.run();
    err_dlg.close();
}

pub fn error_dlg(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    message: &str,
    details: Option<&str>,
) {
    msg_dlg(parent, MessageType::Error, title, message, details);
}

pub fn file_dlg(
    parent: Option<&impl IsA<gtk::Window>>,
    title: &str,
    action: FileChooserAction,
    type_name: &str,
    mask: &str,
    default_name: Option<&str>,
) -> Option<PathBuf> {
    let button = match action {
        FileChooserAction::Open => "Open",
        FileChooserAction::Save => "Save",
        FileChooserAction::SelectFolder => "Select",
        FileChooserAction::CreateFolder => "Create",
        _ => unimplemented!(),
    };

    let file_dlg =
        FileChooserDialog::with_buttons(Some(title), parent, action, &[(button, ResponseType::Ok)]);
    file_dlg.set_default_response(ResponseType::Ok);
    file_dlg.set_do_overwrite_confirmation(action == FileChooserAction::Save);
    if let Some(name) = default_name {
        file_dlg.set_current_name(name);
    }

    let filter = FileFilter::new();
    filter.add_pattern(mask);
    filter.set_name(Some(type_name));
    file_dlg.add_filter(&filter);
    file_dlg.set_filter(&filter);

    let resp = file_dlg.run();
    let path = file_dlg.filename();
    file_dlg.hide();
    file_dlg.close();
    if resp != ResponseType::Ok {
        return None;
    }
    path
}

pub fn file_open_dlg(
    parent: Option<&gtk::ApplicationWindow>,
    title: &str,
    type_name: &str,
    mask: &str,
) -> Option<PathBuf> {
    file_dlg(
        parent,
        title,
        FileChooserAction::Open,
        type_name,
        mask,
        None,
    )
}

pub fn file_save_dlg(
    parent: Option<&gtk::ApplicationWindow>,
    title: &str,
    type_name: &str,
    mask: &str,
) -> Option<PathBuf> {
    file_dlg(
        parent,
        title,
        FileChooserAction::Save,
        type_name,
        mask,
        None,
    )
}

pub fn file_create_dlg(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    type_name: &str,
    mask: &str,
    default_name: &str,
) -> Option<PathBuf> {
    file_dlg(
        Some(parent),
        title,
        FileChooserAction::Save,
        type_name,
        mask,
        Some(default_name),
    )
}

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

use relm::Widget;

mod model;
mod view;

mod device_row;
mod devices;
mod settings;
mod spending_row;
mod types;
mod wallet;

fn main() {
    wallet::Win::run(()).expect("wallet::Win::run failed");
}

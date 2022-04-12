#[macro_use]
extern crate amplify;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

use relm::Widget;

mod settings;
mod wallet;

fn main() {
    wallet::Win::run(()).expect("wallet::Win::run failed");
}

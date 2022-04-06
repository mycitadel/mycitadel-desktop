#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

use relm::Widget;

mod wallet;
mod settings;

fn main() {
    wallet::Win::run(()).expect("wallet::Win::run failed");
}

use crate::gui;
use std::sync::mpsc::channel;

pub fn entry() {
    let (tx, rx) = channel();

    gui::run(rx);
}

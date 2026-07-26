// Timestamped logging. Lines are echoed to stdout and forwarded to the UI
// so they appear in the log panel like the JavaFX LogArea.

use chrono::Local;
use std::sync::mpsc::Sender;

use crate::app::UsbToUi;

pub fn timestamp() -> String {
    Local::now().format("[%H:%M:%S] ").to_string()
}

pub fn log(sender: &Sender<UsbToUi>, msg: &str) {
    let line = format!("{}{}", timestamp(), msg);
    println!("{line}");
    let _ = sender.send(UsbToUi::Log(line));
}
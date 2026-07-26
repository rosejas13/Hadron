// USB transport via nusb (pure-Rust, no libusb). Native on Apple Silicon.
// Ported from xortroll.goldleaf.quark.usb.USBInterface (usb4java/LibUsb).

use std::sync::mpsc::Sender;
use std::time::Duration;

use nusb::transfer::{Bulk, Buffer, In, Out};
use nusb::{Endpoint, MaybeFuture};

use crate::app::UsbToUi;
use crate::command_block::{CommandBlock, BLOCK_SIZE};
use crate::command_framework;
use crate::config::Config;
use crate::logging::log;
use crate::version::Version;

pub const VENDOR_ID: u16 = 0x057E;
pub const PRODUCT_ID: u16 = 0x3000;
pub const WRITE_ENDPOINT: u8 = 0x01;
pub const READ_ENDPOINT: u8 = 0x81;

// Effectively "wait forever" for the next command, matching Java's timeout=0.
// A large-but-finite value avoids any Duration::MAX arithmetic edge cases.
const READ_TIMEOUT: Duration = Duration::from_secs(365 * 86400);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const RECONNECT_DELAY: Duration = Duration::from_millis(750);

pub struct UsbTransport {
    _intf: nusb::Interface,
    ep_in: Endpoint<Bulk, In>,
    ep_out: Endpoint<Bulk, Out>,
}

impl UsbTransport {
    pub fn read_block(&mut self, len: usize) -> Option<Vec<u8>> {
        let comp = self.ep_in.transfer_blocking(Buffer::new(len), READ_TIMEOUT);
        if comp.status.is_err() {
            return None;
        }
        Some(comp.buffer[..comp.actual_len].to_vec())
    }

    pub fn read_bytes(&mut self, len: usize, timeout: Duration) -> Option<Vec<u8>> {
        let comp = self.ep_in.transfer_blocking(Buffer::new(len), timeout);
        if comp.status.is_err() {
            return None;
        }
        let mut v = comp.buffer[..comp.actual_len].to_vec();
        if v.len() < len {
            v.resize(len, 0);
        }
        Some(v)
    }

    pub fn write_bytes(&mut self, data: Vec<u8>) -> bool {
        let comp = self.ep_out.transfer_blocking(data.into(), WRITE_TIMEOUT);
        comp.status.is_ok()
    }

    pub fn write_block_padded(&mut self, data: Vec<u8>, block_size: usize) -> bool {
        let mut padded = data;
        if padded.len() < block_size {
            padded.resize(block_size, 0);
        }
        self.write_bytes(padded)
    }
}

pub struct Connection {
    pub transport: UsbTransport,
    pub version: Version,
    pub is_dev: bool,
}

fn parse_version(serial: &str) -> Option<Version> {
    let tokens: Vec<&str> = serial.split('.').collect();
    if tokens.len() < 2 {
        return None;
    }
    let major = tokens[0].parse::<u8>().ok()?;
    let minor = tokens[1].parse::<u8>().ok()?;
    let micro = tokens.get(2).and_then(|t| t.parse::<u8>().ok()).unwrap_or(0);
    Some(Version::new(major, minor, micro))
}

fn try_connect(sender: &Sender<UsbToUi>) -> Option<Connection> {
    let di = nusb::list_devices()
        .wait()
        .ok()?
        .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)?;

    let device = di.open().wait().ok()?;
    let interface = device.claim_interface(0).wait().ok()?;

    let product = di.product_string().unwrap_or("").to_string();
    log(sender, &format!("USB Product: '{product}'"));
    if !product.contains("Goldleaf") {
        log(sender, "Connection found doesn't seem to be Goldleaf");
        return None;
    }

    let mut serial = di.serial_number().unwrap_or("").to_string();
    log(sender, &format!("USB Serial number: '{serial}'"));

    let mut is_dev = false;
    if let Some(stripped) = serial.strip_suffix("-dev") {
        is_dev = true;
        serial = stripped.to_string();
    }

    let version = match parse_version(&serial) {
        Some(v) => v,
        None => {
            log(sender, "Could not parse Goldleaf version from serial");
            return None;
        }
    };

    if version.older_than(Version::CURRENT) {
        log(
            sender,
            &format!(
                "Goldleaf Quark connected is outdated (v{version}); please update to v{} or higher.",
                Version::CURRENT
            ),
        );
        return None;
    }

    if is_dev {
        log(
            sender,
            &format!(
                "The connected Goldleaf (v{version}) is a development build. \
                 This build might be unstable. Use it at your own risk!"
            ),
        );
    }

    let ep_in: Endpoint<Bulk, In> = interface.endpoint::<Bulk, In>(READ_ENDPOINT).ok()?;
    let ep_out: Endpoint<Bulk, Out> = interface.endpoint::<Bulk, Out>(WRITE_ENDPOINT).ok()?;

    Some(Connection {
        transport: UsbTransport { _intf: interface, ep_in, ep_out },
        version,
        is_dev,
    })
}

pub fn run_usb_loop(cfg: std::sync::Arc<std::sync::Mutex<Config>>, sender: Sender<UsbToUi>) {
    let _ = sender.send(UsbToUi::Status("Searching for Goldleaf...".into()));

    loop {
        let conn = match try_connect(&sender) {
            Some(c) => c,
            None => {
                std::thread::sleep(RECONNECT_DELAY);
                continue;
            }
        };

        let _ = sender.send(UsbToUi::Status(format!(
            "Connected to Goldleaf v{}{} - Processing USB input...",
            conn.version,
            if conn.is_dev { " (dev build)" } else { "" }
        )));

        let mut transport = conn.transport;

        loop {
            let block = match transport.read_block(BLOCK_SIZE) {
                Some(b) if b.len() == BLOCK_SIZE => b,
                _ => break,
            };
            let mut cmd = CommandBlock::from(block);
            let cmd_id = match cmd.validate() {
                Some(id) => id,
                None => {
                    log(&sender, "An invalid command block was received from Goldleaf.");
                    break;
                }
            };
            let handled = command_framework::handle(cmd_id, &mut cmd, &mut transport, &cfg, &sender);
            if !handled {
                log(&sender, &format!("Unrecognized command id {cmd_id}"));
                if !cmd.respond_failure(&mut transport, 0xBAF5) {
                    break;
                }
            }
        }

        let _ = sender.send(UsbToUi::Status("Reconnecting...".into()));
        std::thread::sleep(RECONNECT_DELAY);
    }
}
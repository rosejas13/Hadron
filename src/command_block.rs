// Goldleaf command-block protocol: 0x1000-byte blocks with 'GLCI'/'GLCO' magic.

use crate::buffer::{LeReader, LeWriter};
use crate::usb::UsbTransport;

pub const BLOCK_SIZE: usize = 0x1000;

pub const INPUT_MAGIC: u32 = 0x49434C47; // 'GLCI'
pub const OUTPUT_MAGIC: u32 = 0x4F434C47; // 'GLCO'

pub const RESULT_SUCCESS: u32 = 0;
pub const RESULT_EXCEPTION: u32 = 0xBAF1;
pub const RESULT_INVALID_INDEX: u32 = 0xBAF2;
pub const RESULT_INVALID_FILE_MODE: u32 = 0xBAF3;
pub const RESULT_SELECTION_CANCELLED: u32 = 0xBAF4;

pub const INVALID_COMMAND_ID: u32 = 0;

pub const PATH_TYPE_INVALID: u32 = 0;
pub const PATH_TYPE_FILE: u32 = 1;
pub const PATH_TYPE_DIRECTORY: u32 = 2;

pub const FILE_MODE_READ: u32 = 1;
pub const FILE_MODE_WRITE: u32 = 2;
pub const FILE_MODE_APPEND: u32 = 3;

pub struct CommandBlock {
    reader: LeReader,
    writer: LeWriter,
}

impl CommandBlock {
    pub fn from(inner: Vec<u8>) -> Self {
        CommandBlock {
            reader: LeReader::new(inner),
            writer: LeWriter::new(BLOCK_SIZE),
        }
    }

    pub fn validate(&mut self) -> Option<u32> {
        let magic = self.reader.read_u32();
        if magic == INPUT_MAGIC {
            Some(self.reader.read_u32())
        } else {
            None
        }
    }

    pub fn read_u32(&mut self) -> u32 {
        self.reader.read_u32()
    }

    pub fn read_u64(&mut self) -> u64 {
        self.reader.read_u64()
    }

    pub fn read_string(&mut self) -> String {
        self.reader.read_string()
    }

    pub fn write_u32(&mut self, v: u32) {
        self.writer.push_u32(v);
    }

    pub fn write_u64(&mut self, v: u64) {
        self.writer.push_u64(v);
    }

    pub fn write_string(&mut self, s: &str) {
        self.writer.push_string(s);
    }

    pub fn response_start(&mut self) {
        self.writer.push_u32(OUTPUT_MAGIC);
        self.writer.push_u32(RESULT_SUCCESS);
    }

    pub fn response_end(&mut self, usb: &mut UsbTransport) -> bool {
        usb.write_block_padded(self.writer.buf.clone(), BLOCK_SIZE)
    }

    pub fn respond_empty(&mut self, usb: &mut UsbTransport) -> bool {
        self.response_start();
        self.response_end(usb)
    }

    pub fn respond_failure(&mut self, usb: &mut UsbTransport, rc: u32) -> bool {
        self.writer.push_u32(OUTPUT_MAGIC);
        self.writer.push_u32(rc);
        self.response_end(usb)
    }

    pub fn send_buffer(&mut self, usb: &mut UsbTransport, data: &[u8]) -> bool {
        usb.write_bytes(data.to_vec())
    }

    pub fn get_buffer(&mut self, usb: &mut UsbTransport, len: usize) -> Option<Vec<u8>> {
        usb.read_bytes(len, std::time::Duration::from_secs(30))
    }
}
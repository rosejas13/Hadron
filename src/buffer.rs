// Little-endian buffer reader/writer matching Goldleaf's binary protocol.
// LeReader owns its bytes so a CommandBlock can hold it without lifetime juggling.

pub struct LeReader {
    data: Vec<u8>,
    pos: usize,
}

impl LeReader {
    pub fn new(data: Vec<u8>) -> Self {
        LeReader { data, pos: 0 }
    }

    pub fn read_u32(&mut self) -> u32 {
        let mut b = [0u8; 4];
        self.read_into(&mut b);
        u32::from_le_bytes(b)
    }

    pub fn read_u64(&mut self) -> u64 {
        let mut b = [0u8; 8];
        self.read_into(&mut b);
        u64::from_le_bytes(b)
    }

    pub fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        let end = (self.pos + len).min(self.data.len());
        let out = self.data[self.pos..end].to_vec();
        self.pos = end;
        out
    }

    pub fn read_string(&mut self) -> String {
        let len = self.read_u32() as usize;
        let raw = self.read_bytes(len);
        String::from_utf8_lossy(&raw).into_owned()
    }

    fn read_into(&mut self, dst: &mut [u8]) {
        let n = dst.len();
        let avail = self.data.len().saturating_sub(self.pos);
        if avail == 0 {
            return;
        }
        let take = n.min(avail);
        dst[..take].copy_from_slice(&self.data[self.pos..self.pos + take]);
        self.pos += take;
    }
}

pub struct LeWriter {
    pub buf: Vec<u8>,
}

impl LeWriter {
    pub fn new(capacity: usize) -> Self {
        LeWriter { buf: Vec::with_capacity(capacity) }
    }

    pub fn push_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn push_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn push_bytes(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(v);
    }

    pub fn push_string(&mut self, s: &str) {
        let raw = s.as_bytes();
        self.push_u32(raw.len() as u32);
        self.push_bytes(raw);
    }
}
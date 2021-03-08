use sha3::{Digest, Sha3_256};
use std::io::{self, prelude::*};

#[derive(Clone)]
pub struct Hasher(Sha3_256);

impl Hasher {
    pub fn new() -> Self {
        Self(Sha3_256::new())
    }

    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        self.0.update(data);
    }

    pub fn finalize(self) -> String {
        let mut spec = data_encoding::Specification::new();
        spec.symbols.push_str(
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz@+"
        );
        let encoding = spec.encoding().unwrap();
        let hash_wrapper = self.0.finalize();
        let hash = encoding.encode(hash_wrapper.as_slice());
        format!("{}/{}", &hash[..2], &hash[2..])
    }
}

impl io::Write for Hasher {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

pub fn hash<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut hasher = Hasher::new();
    io::copy(reader, &mut hasher)?;
    Ok(hasher.finalize())
}

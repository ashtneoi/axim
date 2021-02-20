//use sha3::{Digest, Sha3_256};
use std::env::args;
use std::io;

mod nar;

fn main() {
    let argv: Vec<_> = args().collect();
    assert_eq!(argv.len(), 2);
    let top = &argv[1];

    let mut writer = io::stdout();

    crate::nar::dump_nar(&mut writer, &top.as_ref()).unwrap();

/*
 *    let hasher = Sha3_256::new();
 *
 *    let hash = hasher.finalize();
 *    let hash_bytes = hash.as_slice();
 *
 *    let mut spec = data_encoding::Specification::new();
 *    spec.symbols.push_str(
 *        &"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz@%");
 *    let encoding = spec.encoding().unwrap();
 *    let output = encoding.encode(hash_bytes);
 *    println!("{}/{}", &output[..2], &output[2..]);
 */
}

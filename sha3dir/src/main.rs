use sha3::{Digest, Sha3_256};
use std::collections::VecDeque;
use std::env::args;
use std::fs::{File, read_dir};
use std::io::{self, prelude::*};
use std::path::PathBuf;
use std::os::unix::ffi::OsStrExt;

fn main() {
    let argv: Vec<_> = args().collect();
    assert_eq!(argv.len(), 2);
    let mut hasher = Sha3_256::new();
    let mut queue = VecDeque::new();
    queue.push_back(PathBuf::from(&argv[1]));
    while !queue.is_empty() {
        println!("{:?}", &queue);
        let mut entries: Vec<_> =
            read_dir(queue.pop_front().unwrap())
            .unwrap()
            .map(|x| match x {
                Ok(e) => (e.file_name(), e),
                Err(e) => panic!("{:?}", e),
            })
            .collect();

        // TODO: Guarantee that Ord for OsString will never change, then
        // document its behavior.
        entries.sort_by(|x, y| x.0.cmp(&y.0));

        for entry in entries {
            let file_type = entry.1.file_type().unwrap();
            let len = entry.1.metadata().unwrap().len();
            hasher.update(&len.to_le_bytes());
            let path = entry.1.path();
            hasher.update(path.as_os_str().as_bytes());
            if file_type.is_file() {
                let mut f = File::open(&path).unwrap();
                const BLOCK_SIZE: usize = 8192;
                let mut buf = vec![0_u8; BLOCK_SIZE];
                loop {
                    match f.read(&mut buf) {
                        Ok(0) => break,
                        Ok(count) => hasher.update(&buf[..count]),
                        Err(e) => {
                            if e.kind() != io::ErrorKind::Interrupted {
                                panic!("{:?}", e);
                            }
                        },
                    }
                }
                println!("{:?}", entry.0);
            } else if file_type.is_dir() {
                println!("{:?}/", &entry.0);
                queue.push_back(path);
            }
            // TODO: symlinks etc.
        }
    }

    let hash = hasher.finalize();
    let hash_bytes = hash.as_slice();

    let mut spec = data_encoding::Specification::new();
    spec.symbols.push_str(
        &"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz@%");
    let encoding = spec.encoding().unwrap();
    println!("{}", encoding.encode(hash_bytes));
}

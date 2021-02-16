use sha3::{Digest, Sha3_256};
use std::collections::VecDeque;
use std::env::args;
use std::fs::{File, read_dir, read_link};
use std::io::{self, prelude::*};
use std::path::PathBuf;
use std::os::unix::ffi::OsStrExt;

fn main() {
    let argv: Vec<_> = args().collect();
    assert_eq!(argv.len(), 2);
    let mut hasher = Sha3_256::new();
    let mut queue = VecDeque::new();
    let top = &argv[1];
    queue.push_back(PathBuf::from(top));
    while !queue.is_empty() {
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
            let stripped_path = path.strip_prefix(top).unwrap();
            hasher.update(stripped_path.as_os_str().as_bytes());
            if file_type.is_file() {
                let mut f = File::open(&path).unwrap();
                const BLOCK_SIZE: usize = 1<<12;
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
            } else if file_type.is_symlink() {
                let target = read_link(&path).unwrap();
                hasher.update(target.as_os_str().as_bytes());
            } else if file_type.is_dir() {
                hasher.update(b"/");
                queue.push_back(path);
            } else {
                panic!("{:?} file type is unsupported", &path);
            }
        }
    }

    let hash = hasher.finalize();
    let hash_bytes = hash.as_slice();

    let mut spec = data_encoding::Specification::new();
    spec.symbols.push_str(
        &"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz@%");
    let encoding = spec.encoding().unwrap();
    let output = encoding.encode(hash_bytes);
    println!("{}/{}", &output[..2], &output[2..]);
}

use sha3::{Digest, Sha3_256};
use std::env::args;
use std::io::{self, ErrorKind, prelude::*};
use std::os::unix::io::AsRawFd;

mod nar;

fn isatty<F: AsRawFd>(f: &F) -> bool {
    let fd = f.as_raw_fd();
    (unsafe { libc::isatty(fd) }) == 1
}

fn read_retry<R: Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    loop {
        match reader.read(buf) {
            Ok(count) => return Ok(count),
            Err(e) => if e.kind() == ErrorKind::Interrupted {
                return Err(e);
            },
        }
    }
}

fn hash() -> io::Result<()> {
    let mut hasher = Sha3_256::new();

    let mut stdin = io::stdin();
    let mut chunk = vec![0; 1<<12];
    loop {
        let count = read_retry(&mut stdin, &mut chunk)?;
        if count == 0 {
            break;
        }
        hasher.update(&chunk[..count]);
    }

    let hash = hasher.finalize();
    let hash_bytes = hash.as_slice();

    let mut spec = data_encoding::Specification::new();
    spec.symbols.push_str(
        &"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz@%"
    );
    let encoding = spec.encoding().unwrap();
    let output = encoding.encode(hash_bytes);
    println!("{}/{}", &output[..2], &output[2..]);
    Ok(())
}

fn main() {
    let argv: Vec<_> = args().collect();
    let cmd = &argv[1];

    let r = if cmd == &"dump-nar" {
        let top = &argv[2];
        let mut stdout = io::stdout();
        if isatty(&stdout) {
            eprintln!("Error: refusing to dump binary data to a TTY");
            std::process::exit(1);
        }
        crate::nar::dump_nar(&mut stdout, &top.as_ref())
    } else if cmd == &"hash" {
        hash()
    } else {
        eprintln!("Error: invalid command '{}'", &cmd);
        std::process::exit(10);
    };

    if let Err(e) = r {
        eprintln!("Error: {}", &e);
        std::process::exit(1);
    }
}

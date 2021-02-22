use sha3::{Digest, Sha3_256};
use std::env::args;
use std::fs::File;
use std::io::{self, ErrorKind, prelude::*};
use std::os::unix::io::AsRawFd;
use std::process::exit;

mod meta;
mod nar;

fn isatty<F: AsRawFd>(f: &F) -> bool {
    let fd = f.as_raw_fd();
    (unsafe { libc::isatty(fd) }) == 1
}

#[derive(Clone)]
struct Hasher(Sha3_256);

impl Hasher {
    fn new() -> Self {
        Self(Sha3_256::new())
    }

    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.0.update(data);
    }

    fn finalize(self) -> String {
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

fn hash<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut hasher = Hasher::new();
    let mut chunk = vec![0; 1<<12];
    loop {
        let count = read_retry(reader, &mut chunk)?;
        if count == 0 {
            break;
        }
        hasher.update(&chunk[..count]);
    }
    Ok(hasher.finalize())
}

fn do_cmd(argv: &[String]) -> io::Result<()> {
    let cmd = &argv[1];
    if cmd == "dump-nar" {
        let top = &argv[2];
        let mut stdout = io::stdout();
        if isatty(&stdout) {
            eprintln!("Error: refusing to dump binary data to a TTY");
            exit(1);
        }
        crate::nar::dump_nar(&mut stdout, &top.as_ref())?;
    } else if cmd == "hash" {
        let mut f: Box<dyn Read>;
        if &argv[2] == "-" {
            f = Box::new(io::stdin());
        } else {
            f = Box::new(File::open(&argv[2])?);
        }

        println!("{}", hash(&mut f)?);
    } else if cmd == "normalize-meta" {
        let f: Box<dyn Read>;
        if &argv[2] == "-" {
            f = Box::new(io::stdin());
        } else {
            f = Box::new(File::open(&argv[2])?);
        }

        match meta::Meta::parse(f) {
            Err(meta::MetaParseError::IoError(e)) => return Err(e),
            Err(e) => {
                eprintln!("Error: {:?}", &e);
                exit(1);
            },
            Ok(m) => m.dump(io::stdout())?,
        }
    } else {
        eprintln!("Error: invalid command '{}'", cmd);
        exit(10);
    }
    Ok(())
}

fn main() {
    let argv: Vec<_> = args().collect();

    if let Err(e) = do_cmd(&argv) {
        eprintln!("Error: {}", &e);
        exit(1);
    }
}

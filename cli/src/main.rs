use sha3::{Digest, Sha3_256};
use std::env::args;
use std::fs::File;
use std::io::{self, ErrorKind, prelude::*};
use std::os::unix::io::AsRawFd;
use std::process::exit;

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
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz@%"
        );
        let encoding = spec.encoding().unwrap();
        let hash_wrapper = self.0.finalize();
        let hash = encoding.encode(hash_wrapper.as_slice());
        format!("{}/{}", &hash[..2], &hash[2..])
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
        println!("{}", hash(&mut io::stdin())?);
    } else if cmd == "fix-meta" {
        static TYPES: &'static str = "nvxbioh";

        let mut meta_lines: Vec<String> = vec![];
        let f: Box<dyn Read>;
        if &argv[2] == "-" {
            f = Box::new(io::stdin());
        } else {
            f = Box::new(File::open(&argv[2])?);
        }

        let mut hasher = Hasher::new();
        // TODO: We can use .map() for this, if we want.
        for line in io::BufReader::new(f).lines() {
            let line = line?;
            fn bad_line(line: &str) -> ! {
                eprintln!("Error: invalid line '{}'", line);
                exit(1);
            }
            let mut fields = line.splitn(3, " ");
            let typ = fields.next().unwrap_or_else(|| bad_line(&line));
            if !(typ.len() == 1 && TYPES.contains(typ)) {
                bad_line(&line);
            }
            let line = if typ == "o" || typ == "h" {
                let alias = fields.next().unwrap_or_else(|| bad_line(&line));
                format!("{} {} -", typ, alias)
            } else {
                line
            };
            hasher.update(line.as_bytes());
            hasher.update(b"\n");
            meta_lines.push(line);
        }

        meta_lines.sort_unstable_by(|x, y|
            TYPES.find(&x[0..1]).unwrap().cmp(&TYPES.find(&y[0..1]).unwrap())
        );

        for line in meta_lines {
            let mut fields = line.splitn(3, " ");
            let typ = fields.next().unwrap();
            if typ == "o" {
                let alias = fields.next().unwrap();
                let mut id_hasher = hasher.clone();
                id_hasher.update(b"z ");
                id_hasher.update(&alias.as_bytes());
                id_hasher.update(b"\n");
                let id = id_hasher.finalize();
                println!("o {} {}", alias, id);
            } else {
                println!("{}", line);
            }
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

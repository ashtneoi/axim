use std::env::args;
use std::fs;
use std::io::{self, ErrorKind, prelude::*};
use std::os::unix::fs::symlink;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::exit;

pub mod hash;
pub mod meta;
pub mod nar;

fn isatty<F: AsRawFd>(f: &F) -> bool {
    let fd = f.as_raw_fd();
    (unsafe { libc::isatty(fd) }) == 1
}

fn add_file(name: &str, version: &str, file_path: impl AsRef<Path>)
    -> io::Result<()>
{
    // TODO: The Meta stuff here is pretty messy.
    let mut m = meta::Meta {
        name: name.to_string(),
        version: version.to_string(),
        opts: Vec::new(),
        build_cmd: None,
        inputs: Vec::new(),
        output_id: None,
        output_digest: None,
        runtime_deps: Vec::new(),
    };
    m.do_fixed_digest(&file_path)?;

    let mut output_dir: PathBuf = "/axim/".into();
    output_dir.push(&m.output_digest.as_ref().unwrap());

    match fs::remove_dir_all(&output_dir) {
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(e) => return Err(e),
        Ok(_) => (),
    }
    fs::create_dir_all(&output_dir)?;

    let mut src_file = fs::File::open(&file_path)?;
    let mut dest_file = fs::File::create(
        &output_dir.join(file_path.as_ref().file_name().unwrap()))?;
    io::copy(&mut src_file, &mut dest_file)?;
    dest_file.sync_data()?;

    let mut hasher = crate::hash::Hasher::new();
    m.dump(&mut hasher)?;
    let meta_digest = hasher.finalize();

    let axim_dir: &Path = "/axim".as_ref();
    let meta_file_path = axim_dir.join(meta_digest).with_extension("meta");
    // If we didn't remove the file first, a symlink cycle would prevent us
    // from opening the file.
    match fs::remove_file(&meta_file_path) {
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(e) => return Err(e),
        Ok(_) => (),
    }
    let mut meta_file = fs::File::create(&meta_file_path)?;
    m.dump(&mut meta_file)?;
    meta_file.sync_data()?;

    let meta_file_link_path = output_dir.with_extension("meta");
    fs::create_dir_all(&meta_file_link_path.parent().unwrap())?;
    match fs::remove_file(&meta_file_link_path) {
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(e) => return Err(e),
        Ok(_) => (),
    }
    symlink(meta_file_path, &meta_file_link_path)?;
    let meta_file_link = fs::File::open(&meta_file_link_path)?;
    meta_file_link.sync_data()?;

    println!("{}", output_dir.to_str().unwrap());

    Ok(())
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
            f = Box::new(fs::File::open(&argv[2])?);
        }

        println!("{}", hash::hash(&mut f)?);
    } else if cmd == "normalize-meta" {
        let f: Box<dyn Read>;
        if &argv[2] == "-" {
            f = Box::new(io::stdin());
        } else {
            f = Box::new(fs::File::open(&argv[2])?);
        }

        match meta::Meta::parse(f) {
            Err(meta::MetaParseError::IoError(e)) => return Err(e),
            Err(e) => {
                eprintln!("Error: {:?}", &e);
                exit(1);
            },
            Ok(m) => m.dump(io::stdout())?,
        }
    } else if cmd == "add-file" {
        add_file(&argv[2], &argv[3], &argv[4])?;
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

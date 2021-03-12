use nix::sys::stat::{Mode, umask};
use nix::unistd::isatty;
use std::env::args;
use std::fs;
use std::io::{self, ErrorKind, prelude::*};
use std::os::unix;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::exit;

pub mod hash;
pub mod meta;
pub mod nar;

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

    let dest_path = output_dir.join(file_path.as_ref().file_name().unwrap());

    let src_metadata = fs::symlink_metadata(&file_path)?;
    let src_file_type = src_metadata.file_type();
    if src_file_type.is_file() {
        let mode;
        if src_metadata.permissions().mode() & 0o100 != 0 {
            mode = 0o555;
        } else {
            mode = 0o444;
        }
        let mut dest_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&dest_path)?;
        let mut src_file = fs::File::open(&file_path)?;
        io::copy(&mut src_file, &mut dest_file)?;
        dest_file.sync_data()?;
    } else if src_file_type.is_dir() {
        unimplemented!(); // TODO: do we need to implement this?
    } else if src_file_type.is_symlink() {
        unimplemented!(); // TODO: is it a good idea to implement this?
        //let target = fs::read_link(&file_path)?;
        //unix::fs::symlink(target, &dest_path)?;
    }
    fs::File::open(output_dir.parent().unwrap())?.sync_data()?;
    fs::File::open(&output_dir)?.sync_data()?;

    let mut hasher = crate::hash::Hasher::new();
    m.dump(&mut hasher)?;
    let meta_digest = hasher.finalize();

    let axim_dir: &Path = "/axim".as_ref();
    let meta_file_path = axim_dir.join(meta_digest).with_extension("meta");
    fs::create_dir_all(&meta_file_path.parent().unwrap())?;
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
    fs::File::open(meta_file_path.parent().unwrap())?.sync_data()?;

    let meta_file_link_path = output_dir.with_extension("meta");
    fs::create_dir_all(&meta_file_link_path.parent().unwrap())?;
    match fs::remove_file(&meta_file_link_path) {
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(e) => return Err(e),
        Ok(_) => (),
    }
    unix::fs::symlink(meta_file_path, &meta_file_link_path)?;
    fs::File::open(&meta_file_link_path)?.sync_data()?;
    fs::File::open(meta_file_link_path.parent().unwrap())?.sync_data()?;

    println!("{}", output_dir.to_str().unwrap());

    Ok(())
}

fn do_cmd(argv: &[String]) -> anyhow::Result<()> {
    let cmd = &argv[1];
    if cmd == "dump-nar" {
        let top = &argv[2];
        let mut stdout = io::stdout();
        if isatty(stdout.as_raw_fd())? {
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
            Err(meta::MetaParseError::IoError(e)) => Err(e)?,
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

    umask(Mode::from_bits(0o022).unwrap());

    if let Err(e) = do_cmd(&argv) {
        eprintln!("Error: {}", &e);
        exit(1);
    }
}

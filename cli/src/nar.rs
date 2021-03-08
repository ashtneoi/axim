use std::collections::VecDeque;
use std::fs::{self, DirEntry, File, read_dir, read_link};
use std::io::{self, prelude::*};
use std::os::unix::{
    ffi::OsStrExt,
    fs::PermissionsExt,
};
use std::path::{Path, PathBuf};

fn pad_from_len<W: Write>(writer: &mut W, len: u64) -> io::Result<()> {
    let padding = ((len + 7) & !0x7) - len;
    for _ in 0..padding {
        writer.write_all(&[0])?;
    }
    Ok(())
}

fn serialise_str<W: Write>(writer: &mut W, x: &[u8]) -> io::Result<()> {
    let len: u64 = x.len() as u64;
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(x)?;
    pad_from_len(writer, len)?;
    Ok(())
}

fn serialise_node<W: Write>(writer: &mut W, entry: &NarEntry)
    -> io::Result<()>
{
    let file_type = &entry.file_type;
    if file_type.is_file() {
        let metadata = entry.metadata.as_ref().unwrap();
        serialise_str(writer, b"type")?;
        serialise_str(writer, b"regular")?;
        let executable = metadata.permissions().mode() & 0o100;
        if executable != 0 {
            serialise_str(writer, b"executable")?;
            serialise_str(writer, b"")?;
        }
        serialise_str(writer, b"contents")?;

        let len = metadata.len();
        writer.write_all(&len.to_le_bytes())?;
        let mut f = File::open(&entry.path)?;
        io::copy(&mut f, writer)?;
        pad_from_len(writer, len)?;
    } else if file_type.is_symlink() {
        serialise_str(writer, b"type")?;
        serialise_str(writer, b"symlink")?;
        serialise_str(writer, b"target")?;
        serialise_str(writer, read_link(&entry.path)?.as_os_str().as_bytes())?;
    } else if file_type.is_dir() {
        serialise_str(writer, b"type")?;
        serialise_str(writer, b"directory")?;
    }
    Ok(())
}

struct NarEntry {
    path: PathBuf,
    file_type: fs::FileType,
    metadata: Option<fs::Metadata>,
}

impl NarEntry {
    fn from_dir_entry(dir_entry: DirEntry) -> io::Result<Self> {
        let file_type = dir_entry.file_type()?;
        let metadata = if !file_type.is_dir() {
            Some(dir_entry.metadata()?)
        } else {
            None
        };
        Ok(Self {
            path: dir_entry.path(),
            file_type,
            metadata,
        })
    }
}

pub(crate) fn dump_nar<W: Write>(mut writer: &mut W, top: &Path)
    -> io::Result<()>
{
    let top_metadata = fs::symlink_metadata(top)?;
    let mut top_entry = NarEntry {
        path: top.into(),
        file_type: top_metadata.file_type(),
        metadata: None,
    };
    if !top_entry.file_type.is_dir() {
        top_entry.metadata = Some(top_metadata);
    }
    let mut stack: Vec<VecDeque<NarEntry>> =
        vec![vec![top_entry.into()].into()];

    serialise_str(&mut writer, b"nix-archive-1")?;
    serialise_str(&mut writer, b"(")?;

    while !stack.is_empty() {
        if let Some(entry) = stack.last_mut().unwrap().pop_front() {
            if stack.len() > 1 {
                serialise_str(&mut writer, b"entry")?;
                serialise_str(&mut writer, b"(")?;
                serialise_str(&mut writer, b"name")?;
                let name = entry.path.file_name().unwrap();
                serialise_str(&mut writer, name.as_bytes())?;
                serialise_str(&mut writer, b"node")?;
                serialise_str(&mut writer, b"(")?;
            }
            serialise_node(&mut writer, &entry)?;
            if entry.file_type.is_dir() {
                let mut entries: Vec<_> = read_dir(&entry.path)?
                    .map(|x| NarEntry::from_dir_entry(x?))
                    .collect::<io::Result<_>>()?;
                entries.sort_unstable_by(
                    |x, y| x.path.file_name().cmp(&y.path.file_name()));
                stack.push(entries.into());
            } else {
                if stack.len() > 1 {
                    serialise_str(&mut writer, b")")?; // node
                    serialise_str(&mut writer, b")")?; // entry
                }
            }
        } else {
            stack.pop().unwrap();

            if stack.len() > 1 {
                serialise_str(&mut writer, b")")?; // node
                serialise_str(&mut writer, b")")?; // entry
            }
        }
    }

    serialise_str(&mut writer, b")")?;

    Ok(())
}

pub(crate) fn dump_file_nar<W: Write>(mut writer: &mut W, path: &Path)
    -> io::Result<()>
{
    let metadata = fs::symlink_metadata(path)?;
    let mut entry = NarEntry {
        path: path.into(),
        file_type: metadata.file_type(),
        metadata: None,
    };
    assert!(entry.file_type.is_file());
    entry.metadata = Some(metadata);

    serialise_str(&mut writer, b"nix-archive-1")?;
    serialise_str(&mut writer, b"(")?;

    serialise_str(writer, b"type")?;
    serialise_str(writer, b"directory")?;

    serialise_str(&mut writer, b"entry")?;
    serialise_str(&mut writer, b"(")?;
    serialise_str(&mut writer, b"name")?;
    let name = entry.path.file_name().unwrap();
    serialise_str(&mut writer, name.as_bytes())?;
    serialise_str(&mut writer, b"node")?;
    serialise_str(&mut writer, b"(")?;

    serialise_node(&mut writer, &entry)?;

    serialise_str(&mut writer, b")")?; // node
    serialise_str(&mut writer, b")")?; // entry

    serialise_str(&mut writer, b")")?;

    Ok(())
}

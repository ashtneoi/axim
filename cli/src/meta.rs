use std::io::{self, prelude::*};

#[derive(Debug)]
struct MetaInput {
    alias: String,
    id: String,
}

#[derive(Debug)]
struct MetaOutput {
    alias: String,
    id: Option<String>,
    digest: Option<String>,
}

#[derive(Debug)]
pub enum MetaParseError {
    IoError(io::Error),
    InvalidType { line: usize }, // zero-based
    MissingField { line: usize }, // zero-based
    DuplicateType { line: usize }, // zero-based
    DuplicateAlias { line: usize }, // zero-based
    InvalidOption { line: usize }, // zero-based
    InvalidId { line: usize }, // zero-based
    MissingName,
    MissingVersion,
    InvalidData(String),
}

impl From<io::Error> for MetaParseError {
    fn from(io_error: io::Error) -> Self {
        Self::IoError(io_error)
    }
}

#[derive(Debug)]
pub(crate) struct Meta {
    name: String,
    version: String,
    opts: Vec<String>,
    build_cmd: Option<String>,
    inputs: Vec<MetaInput>,
    outputs: Vec<MetaOutput>,
}

impl Meta {
    pub(crate) fn parse(reader: impl Read) -> Result<Self, MetaParseError> {
        let mut name = None;
        let mut version = None;
        let mut opts = Vec::new();
        let mut build_cmd = None;
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        for (i, line) in io::BufReader::new(reader).lines().enumerate() {
            let line = line?;
            if line.starts_with('#') {
                continue;
            }
            let fields: Vec<_> = line.splitn(3, " ").collect();
            if fields.len() < 2 {
                return Err(MetaParseError::MissingField { line: i });
            }

            let typ = fields[0];
            if typ.len() != 1 {
                return Err(MetaParseError::InvalidType { line: i });
            }

            if typ == "n" {
                if name.is_some() {
                    return Err(MetaParseError::DuplicateType { line: i });
                }
                name = Some(line["n ".len()..].to_string());
            } else if typ == "v" {
                if version.is_some() {
                    return Err(MetaParseError::DuplicateType { line: i });
                }
                version = Some(line["v ".len()..].to_string());
            } else if typ == "x" {
                let opt = &line["x ".len()..];
                if opt == "fixed-digest" {
                    opts.push(opt.to_string());
                } else {
                    return Err(MetaParseError::InvalidOption { line: i });
                }
            } else if typ == "b" {
                let cmd = &line["b ".len()..];
                if build_cmd.is_some() {
                    return Err(MetaParseError::DuplicateType { line: i });
                }
                build_cmd = Some(cmd.to_string());
            } else if typ == "i" {
                let alias = fields[1];
                let id = *fields.get(2).ok_or(
                    MetaParseError::MissingField { line: i })?;

                if id == "-" {
                    return Err(MetaParseError::InvalidId { line: i });
                }

                // WATCH OUT: This is quadratic time in the number of aliases.
                let dup = inputs.iter().find(
                    |&&MetaInput { alias: ref a, .. }| a == alias);
                if dup.is_some() {
                    return Err(MetaParseError::DuplicateAlias { line: i });
                }
                inputs.push(MetaInput {
                    alias: fields[1].to_string(),
                    id: id.to_string(),
                });
            } else if typ == "o" {
                let alias = fields[1];
                let id = match fields.get(2) {
                    Some(&"-") => None,
                    Some(id) => Some(*id),
                    None => return Err(
                        MetaParseError::MissingField { line: i }),
                };

                // WATCH OUT: This is quadratic time in the number of aliases.
                let dup = outputs.iter_mut().find(
                    |&&mut MetaOutput { alias: ref a, .. }| a == alias);
                if let Some(dup) = dup {
                    if dup.id.is_some() {
                        return Err(MetaParseError::DuplicateAlias { line: i });
                    }
                    dup.id = id.map(|x| x.to_string());
                } else {
                    outputs.push(MetaOutput {
                        alias: alias.to_string(),
                        id: id.map(|x| x.to_string()),
                        digest: None,
                    });
                }
            } else if typ == "d" {
                let alias = fields[1];
                let digest = match fields.get(2) {
                    Some(&"-") => None,
                    Some(digest) => Some(*digest),
                    None => return Err(
                        MetaParseError::MissingField { line: i }),
                };

                // WATCH OUT: This is quadratic time in the number of aliases.
                let dup = outputs.iter_mut().find(
                    |&&mut MetaOutput { alias: ref a, .. }| a == alias);
                if let Some(dup) = dup {
                    if dup.digest.is_some() {
                        return Err(MetaParseError::DuplicateAlias { line: i });
                    }
                    dup.digest = digest.map(|x| x.to_string());
                } else {
                    outputs.push(MetaOutput {
                        alias: alias.to_string(),
                        id: None,
                        digest: digest.map(|x| x.to_string()),
                    });
                }
            } else {
                return Err(MetaParseError::InvalidType { line: i });
            }
        }

        let mut obj = Self {
            name: name.ok_or(MetaParseError::MissingName)?,
            version: version.ok_or(MetaParseError::MissingVersion)?,
            opts,
            build_cmd,
            inputs,
            outputs,
        };

        if obj.option("fixed-digest") {
            // FIXME: Set `o` values to `d` values (error if already set).
            for output in &mut obj.outputs {
                if output.id.is_some() {
                    // TODO: workshop this message
                    return Err(MetaParseError::InvalidData(
                        "fixed-digest requires output IDs to be unset"
                        .to_string()
                    ));
                }
                if let Some(ref digest) = output.digest {
                    output.id = Some(digest.to_string());
                } else {
                    // TODO: workshop this message
                    return Err(MetaParseError::InvalidData(
                        "fixed-digest requires output digests to be set"
                        .to_string()
                    ));
                }
            }
        } else {
            // FIXME: ???
            unimplemented!();
        }

        Ok(obj)
    }

    fn option(&self, opt: &str) -> bool {
        self.opts.iter().find(|&opt2| opt2 == opt).is_some()
    }

    pub(crate) fn dump(&self, mut writer: impl Write) -> io::Result<()> {
        writeln!(writer, "n {}", &self.name)?;
        writeln!(writer, "v {}", &self.version)?;
        for opt in &self.opts {
            writeln!(writer, "x {}", opt)?;
        }
        for input in &self.inputs {
            writeln!(writer, "i {} {}", &input.alias, &input.id)?;
        }
        if let Some(ref build_cmd) = self.build_cmd {
            writeln!(writer, "b {}", build_cmd)?;
        }
        for output in &self.outputs {
            writeln!(
                writer,
                "o {} {}",
                &output.alias,
                match output.id { Some(ref s) => s.as_ref(), None => "-" },
            )?;
            if let Some(ref digest) = output.digest {
                writeln!(writer, "d {} {}", &output.alias, digest)?;
            }
        }
        Ok(())
    }
}

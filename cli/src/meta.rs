use std::io::{self, prelude::*};

#[derive(Debug)]
struct MetaInput {
    alias: String,
    id: String,
}

#[derive(Debug)]
pub enum MetaParseError {
    IoError(io::Error),
    InvalidType { line: usize }, // zero-based
    MissingField { line: usize }, // zero-based
    DuplicateType { line: usize }, // zero-based
    DuplicateAlias { line: usize }, // zero-based
    InvalidOption { line: usize }, // zero-based
    MissingName,
    MissingVersion,
    _InvalidData(String),
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
    output_id: Option<String>,
    output_digest: Option<String>,
    runtime_deps: Vec<String>,
}

impl Meta {
    pub(crate) fn parse(reader: impl Read) -> Result<Self, MetaParseError> {
        let mut name = None;
        let mut version = None;
        let mut opts = Vec::new();
        let mut build_cmd = None;
        let mut inputs = Vec::new();
        let mut output_id = None;
        let mut output_digest = None;
        let mut runtime_deps = Vec::new();

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
                if output_id.is_some() {
                    return Err(MetaParseError::DuplicateType { line: i });
                }
                output_id = Some(line["o ".len()..].to_string());
            } else if typ == "d" {
                if output_digest.is_some() {
                    return Err(MetaParseError::DuplicateType { line: i });
                }
                output_digest = Some(line["d ".len()..].to_string());
            } else if typ == "r" {
                runtime_deps.push(line["r ".len()..].to_string());
            } else {
                return Err(MetaParseError::InvalidType { line: i });
            }
        }

        inputs.sort_unstable_by(|x, y| x.alias.cmp(&y.alias));

        let mut obj = Self {
            name: name.ok_or(MetaParseError::MissingName)?,
            version: version.ok_or(MetaParseError::MissingVersion)?,
            opts,
            build_cmd,
            inputs,
            output_id,
            output_digest,
            runtime_deps,
        };

        if obj.output_id.is_none() {
            obj.set_output_id();
        }

        Ok(obj)
    }

    fn _option(&self, opt: &str) -> bool {
        self.opts.iter().find(|&opt2| opt2 == opt).is_some()
    }

    fn set_output_id(&mut self) {
        assert_eq!(self.output_id, None);

        let mut hasher = crate::Hasher::new();
        writeln!(hasher, "n {}", &self.name).unwrap();
        writeln!(hasher, "v {}", &self.version).unwrap();
        for opt in &self.opts {
            writeln!(hasher, "x {}", opt).unwrap();
        }
        for input in &self.inputs {
            writeln!(hasher, "i {} {}", &input.alias, &input.id).unwrap();
        }
        if let Some(ref build_cmd) = self.build_cmd {
            writeln!(hasher, "b {}", build_cmd).unwrap();
        }
        self.output_id = Some(hasher.finalize());
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
        if let Some(ref output_id) = self.output_id {
            writeln!(writer, "o {}", output_id)?;
        }
        if let Some(ref output_digest) = self.output_digest {
            writeln!(writer, "d {}", output_digest)?;
        }
        Ok(())
    }
}

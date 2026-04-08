use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::parse::parse_document;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AaakMeta {
    pub wing: String,
    pub room: String,
    pub date: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AaakHeader {
    pub version: u8,
    pub wing: String,
    pub room: String,
    pub date: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zettel {
    pub id: usize,
    pub entities: Vec<String>,
    pub topics: Vec<String>,
    pub quote: String,
    pub weight: u8,
    pub emotions: Vec<String>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AaakDocument {
    pub header: AaakHeader,
    pub zettels: Vec<Zettel>,
}

impl AaakDocument {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        parse_document(input)
    }
}

impl Display for AaakDocument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "V{}|{}|{}|{}|{}",
            self.header.version,
            self.header.wing,
            self.header.room,
            self.header.date,
            self.header.source
        )?;

        for (index, zettel) in self.zettels.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "{}:{}|{}|\"{}\"|{}|{}|{}",
                zettel.id,
                zettel.entities.join("+"),
                zettel.topics.join("_"),
                zettel.quote.replace('"', "'"),
                "★".repeat(usize::from(zettel.weight.max(1))),
                zettel.emotions.join("+"),
                zettel.flags.join("+"),
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodeReport {
    pub topics_truncated: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodeOutput {
    pub document: AaakDocument,
    pub report: EncodeReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundtripReport {
    pub preserved: Vec<String>,
    pub lost: Vec<String>,
    pub coverage: f32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("missing AAAK header")]
    MissingHeader,
    #[error("invalid AAAK header")]
    InvalidHeader,
    #[error("invalid version marker")]
    InvalidVersion,
    #[error("invalid zettel line: {0}")]
    InvalidZettel(String),
}

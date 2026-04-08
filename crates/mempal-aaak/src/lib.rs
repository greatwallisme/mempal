#![warn(clippy::all)]

mod codec;
mod model;
mod parse;

pub use codec::AaakCodec;
pub use model::{
    AaakDocument, AaakHeader, AaakMeta, EncodeOutput, EncodeReport, ParseError, RoundtripReport,
    Zettel,
};

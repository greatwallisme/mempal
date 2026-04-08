use crate::model::{AaakDocument, AaakHeader, ParseError, Zettel};

pub fn parse_document(input: &str) -> Result<AaakDocument, ParseError> {
    let mut lines = input.lines().filter(|line| !line.trim().is_empty());
    let header = parse_header(lines.next().ok_or(ParseError::MissingHeader)?)?;
    let mut zettels = Vec::new();

    for line in lines {
        if line.starts_with("T:") || line.starts_with("ARC:") {
            continue;
        }
        zettels.push(parse_zettel(line)?);
    }

    Ok(AaakDocument { header, zettels })
}

fn parse_header(line: &str) -> Result<AaakHeader, ParseError> {
    let mut parts = line.split('|');
    let version = parts.next().ok_or(ParseError::InvalidHeader)?;
    let wing = parts.next().ok_or(ParseError::InvalidHeader)?;
    let room = parts.next().ok_or(ParseError::InvalidHeader)?;
    let date = parts.next().ok_or(ParseError::InvalidHeader)?;
    let source = parts.next().ok_or(ParseError::InvalidHeader)?;
    if parts.next().is_some() {
        return Err(ParseError::InvalidHeader);
    }

    let version = version
        .strip_prefix('V')
        .ok_or(ParseError::InvalidVersion)?
        .parse::<u8>()
        .map_err(|_| ParseError::InvalidVersion)?;

    Ok(AaakHeader {
        version,
        wing: wing.to_string(),
        room: room.to_string(),
        date: date.to_string(),
        source: source.to_string(),
    })
}

fn parse_zettel(line: &str) -> Result<Zettel, ParseError> {
    let (id, rest) = line
        .split_once(':')
        .ok_or_else(|| ParseError::InvalidZettel(line.to_string()))?;
    let id = id
        .parse::<usize>()
        .map_err(|_| ParseError::InvalidZettel(line.to_string()))?;
    let parts = rest.split('|').collect::<Vec<_>>();
    if parts.len() != 6 {
        return Err(ParseError::InvalidZettel(line.to_string()));
    }

    let entities = split_field(parts[0], '+');
    let topics = split_field(parts[1], '_');
    let quote = parts[2].trim_matches('"').to_string();
    let weight = u8::try_from(parts[3].chars().filter(|ch| *ch == '★').count())
        .map_err(|_| ParseError::InvalidZettel(line.to_string()))?;
    let emotions = split_field(parts[4], '+');
    let flags = split_field(parts[5], '+');

    Ok(Zettel {
        id,
        entities,
        topics,
        quote,
        weight,
        emotions,
        flags,
    })
}

fn split_field(raw: &str, separator: char) -> Vec<String> {
    raw.split(separator)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

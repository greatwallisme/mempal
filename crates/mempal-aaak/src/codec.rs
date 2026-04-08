use std::collections::{BTreeMap, BTreeSet};

use bimap::BiHashMap;

use crate::model::{
    AaakDocument, AaakHeader, AaakMeta, EncodeOutput, EncodeReport, RoundtripReport, Zettel,
};

const DEFAULT_MAX_TOPICS: usize = 3;
const DEFAULT_EMOTION: &str = "determ";

#[derive(Debug, Clone)]
pub struct AaakCodec {
    entity_map: BiHashMap<String, String>,
    max_topics: usize,
}

impl Default for AaakCodec {
    fn default() -> Self {
        Self {
            entity_map: BiHashMap::new(),
            max_topics: DEFAULT_MAX_TOPICS,
        }
    }
}

impl AaakCodec {
    pub fn with_entity_aliases(aliases: BTreeMap<String, String>) -> Self {
        let entity_map = aliases.into_iter().collect::<BiHashMap<_, _>>();
        Self {
            entity_map,
            max_topics: DEFAULT_MAX_TOPICS,
        }
    }

    pub fn encode(&self, text: &str, meta: &AaakMeta) -> EncodeOutput {
        let normalized = normalize_whitespace(text);
        let all_topics = extract_topics(&normalized);
        let topics = all_topics
            .iter()
            .take(self.max_topics)
            .cloned()
            .collect::<Vec<_>>();
        let topics_truncated = all_topics.len().saturating_sub(self.max_topics);
        let entities = extract_entities(&normalized)
            .into_iter()
            .map(|entity| self.entity_code(&entity))
            .collect::<Vec<_>>();
        let flags = detect_flags(&normalized);
        let emotions = detect_emotions(&normalized);

        let document = AaakDocument {
            header: AaakHeader {
                version: 1,
                wing: meta.wing.clone(),
                room: meta.room.clone(),
                date: meta.date.clone(),
                source: meta.source.clone(),
            },
            zettels: vec![Zettel {
                id: 0,
                entities,
                topics: if topics.is_empty() {
                    vec!["note".to_string()]
                } else {
                    topics
                },
                quote: normalized,
                weight: infer_weight(&flags),
                emotions,
                flags,
            }],
        };

        EncodeOutput {
            document,
            report: EncodeReport { topics_truncated },
        }
    }

    pub fn decode(&self, document: &AaakDocument) -> String {
        document
            .zettels
            .iter()
            .map(|zettel| self.decode_zettel(zettel))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn verify_roundtrip(&self, original: &str, document: &AaakDocument) -> RoundtripReport {
        let decoded = normalize_whitespace(&self.decode(document)).to_lowercase();
        let assertions = split_assertions(original);
        if assertions.is_empty() {
            return RoundtripReport {
                preserved: Vec::new(),
                lost: Vec::new(),
                coverage: 1.0,
            };
        }

        let (preserved, lost): (Vec<_>, Vec<_>) = assertions
            .into_iter()
            .partition(|assertion| decoded.contains(&assertion.to_lowercase()));
        let coverage = preserved.len() as f32 / (preserved.len() + lost.len()) as f32;

        RoundtripReport {
            preserved,
            lost,
            coverage,
        }
    }

    fn decode_zettel(&self, zettel: &Zettel) -> String {
        let mut quote = zettel.quote.clone();
        for entity in &zettel.entities {
            if let Some(name) = self.entity_map.get_by_right(entity) {
                quote = replace_code(&quote, entity, name);
            }
        }

        if quote.is_empty() {
            return zettel
                .entities
                .iter()
                .map(|entity| {
                    self.entity_map
                        .get_by_right(entity)
                        .cloned()
                        .unwrap_or_else(|| entity.clone())
                })
                .collect::<Vec<_>>()
                .join(" ");
        }

        quote
    }

    fn entity_code(&self, entity: &str) -> String {
        self.entity_map
            .get_by_left(entity)
            .cloned()
            .unwrap_or_else(|| default_entity_code(entity))
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "'")
        .trim()
        .to_string()
}

fn extract_entities(text: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut entities = Vec::new();

    for token in text.split(|ch: char| !ch.is_alphanumeric()) {
        if token.len() < 2 {
            continue;
        }

        let first = token.chars().next().unwrap_or_default();
        if !first.is_uppercase() {
            continue;
        }

        if seen.insert(token.to_string()) {
            entities.push(token.to_string());
        }
    }

    entities
}

fn extract_topics(text: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "with", "over", "based", "this", "that", "was", "use", "why",
    ];

    let mut seen = BTreeSet::new();
    let mut topics = Vec::new();

    for token in text.split(|ch: char| !ch.is_alphanumeric()) {
        let token = token.trim().to_lowercase();
        if token.is_empty() || STOP_WORDS.contains(&token.as_str()) {
            continue;
        }

        if seen.insert(token.clone()) {
            topics.push(token);
        }
    }

    topics
}

fn detect_flags(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut flags = Vec::new();

    if contains_any(
        &lower,
        &[
            "decid",
            "chose",
            "switch",
            "migrat",
            "replace",
            "recommend",
            "because",
        ],
    ) {
        flags.push("DECISION".to_string());
    }
    if contains_any(
        &lower,
        &[
            "api",
            "database",
            "architecture",
            "deploy",
            "infrastructure",
            "framework",
            "server",
            "config",
            "auth",
            "clerk",
            "auth0",
        ],
    ) {
        flags.push("TECHNICAL".to_string());
    }
    if flags.is_empty() {
        flags.push("CORE".to_string());
    }

    flags
}

fn detect_emotions(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut emotions = Vec::new();

    if contains_any(&lower, &["decid", "switch", "migrat"]) {
        emotions.push("determ".to_string());
    }
    if contains_any(&lower, &["worry", "anx", "concern"]) {
        emotions.push("anx".to_string());
    }
    if contains_any(&lower, &["excited", "excite"]) {
        emotions.push("excite".to_string());
    }

    if emotions.is_empty() {
        emotions.push(DEFAULT_EMOTION.to_string());
    }

    emotions
}

fn infer_weight(flags: &[String]) -> u8 {
    if flags.iter().any(|flag| flag == "DECISION" || flag == "PIVOT") {
        4
    } else if flags.iter().any(|flag| flag == "TECHNICAL") {
        3
    } else {
        2
    }
}

fn split_assertions(text: &str) -> Vec<String> {
    text.split(['.', '!', '?', ';'])
        .map(normalize_whitespace)
        .filter(|item| !item.is_empty())
        .collect()
}

fn replace_code(quote: &str, code: &str, name: &str) -> String {
    quote.split_whitespace()
        .map(|token| {
            if token == code {
                name.to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_entity_code(entity: &str) -> String {
    let mut code = entity
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(3)
        .collect::<String>()
        .to_uppercase();
    while code.len() < 3 {
        code.push('X');
    }
    code
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

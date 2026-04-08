use std::collections::BTreeMap;

use mempal_aaak::{AaakCodec, AaakDocument, AaakMeta};

fn meta() -> AaakMeta {
    AaakMeta {
        wing: "myapp".to_string(),
        room: "auth".to_string(),
        date: "2026-04-08".to_string(),
        source: "readme".to_string(),
    }
}

#[test]
fn test_aaak_encode() {
    let codec = AaakCodec::default();
    let output = codec.encode(
        "Kai recommended Clerk over Auth0 based on pricing and DX",
        &meta(),
    );

    let encoded = output.document.to_string();
    assert!(encoded.contains("KAI"));
    assert!(encoded.contains("DECISION"));
}

#[test]
fn test_aaak_decode() {
    let codec = AaakCodec::with_entity_aliases(BTreeMap::from([
        ("Kai".to_string(), "KAI".to_string()),
        ("Clerk".to_string(), "CLK".to_string()),
    ]));
    let doc = AaakDocument::parse(
        "V1|myapp|auth|2026-04-08|readme\n0:KAI+CLK|clerk_auth|\"Kai chose Clerk\"|★★★★|determ|DECISION",
    )
    .expect("document should parse");

    let decoded = codec.decode(&doc);
    assert!(decoded.contains("Kai"));
    assert!(decoded.contains("Clerk"));
}

#[test]
fn test_aaak_roundtrip() {
    let codec = AaakCodec::default();
    let original = "Kai recommended Clerk over Auth0. Pricing was better. Developer experience improved. The migration reduced auth bugs. The team documented the decision.";
    let output = codec.encode(original, &meta());
    let report = codec.verify_roundtrip(original, &output.document);

    assert!(report.coverage >= 0.8);
    assert!(report.lost.len() <= 1);
}

#[test]
fn test_entity_bimap() {
    let codec = AaakCodec::with_entity_aliases(BTreeMap::from([(
        "Alice".to_string(),
        "ALC".to_string(),
    )]));
    let output = codec.encode("Alice finalized the rollout plan", &meta());
    let encoded = output.document.to_string();
    assert!(encoded.contains("ALC"));

    let decoded = codec.decode(&output.document);
    assert!(decoded.contains("Alice"));
}

#[test]
fn test_aaak_parse() {
    let document = AaakDocument::parse(
        "V1|myapp|auth|2026-04-08|readme\n0:KAI|clerk_auth|\"use Clerk\"|★★★★|determ|DECISION",
    )
    .expect("document should parse");

    assert_eq!(document.header.version, 1);
    assert_eq!(document.zettels.len(), 1);
    assert_eq!(document.zettels[0].entities, vec!["KAI"]);
}

#[test]
fn test_aaak_parse_invalid() {
    assert!(AaakDocument::parse("this is not aaak").is_err());
}

#[test]
fn test_aaak_version() {
    let codec = AaakCodec::default();
    let output = codec.encode("Decision: use Clerk", &meta());

    assert!(output.document.to_string().starts_with("V1|"));
}

#[test]
fn test_aaak_truncation_report() {
    let codec = AaakCodec::default();
    let output = codec.encode(
        "alpha beta gamma delta epsilon zeta eta theta iota kappa",
        &meta(),
    );

    assert_eq!(output.report.topics_truncated, 7);
}

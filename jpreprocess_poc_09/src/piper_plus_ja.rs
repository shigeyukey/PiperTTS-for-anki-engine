//! Port of piper-plus's japanese phonemizer (`piper_plus_g2p/japanese.py`, MIT).
//!
//! Reference implementation:
//!   https://github.com/ayutaz/piper-plus (src/python/g2p/piper_plus_g2p/japanese.py)
//!
//! Input : OpenJTalk full-context labels (produced by jpreprocess)
//! Output: raw OpenJTalk phone tokens with piper-plus markers
//!         ("[", "]", "#", "_", "?", "?!", "?.", "?~") and prosody A1/A2/A3.
//!         The caller maps the tokens to phoneme ids (PUA table + phoneme_id_map).
//!
//! This module targets the jpreprocess version used by the official
//! piper-plus wasm build (0.9.1) and phonemizes the whole text in one pass
//! (no sentence splitting), matching the official `piper-plus-wasm`
//! `phonemize(text)`.
//!
//! Keep this file in sync with the Python reference; the golden test is
//! dev_tools/tests/piper_plus/expected_ja.json (see check_phonemes.py).

use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Phoneme {
    pub token: String,
    pub prosody: Option<(i32, i32, i32)>,
}

fn phone_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"\-(.*?)\+").unwrap())
}

fn prosody_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"/A:([\d-]+)\+(\d+)\+(\d+)/").unwrap())
}

fn a2_of(label: &str) -> Option<i32> {
    prosody_pattern()
        .captures(label)
        .and_then(|captures| captures.get(2))
        .and_then(|value| value.as_str().parse::<i32>().ok())
}

fn is_skip_token(token: &str) -> bool {
    matches!(
        token,
        "_" | "#" | "[" | "]" | "^" | "$" | "?" | "?!" | "?." | "?~"
    )
}

pub fn question_marker(text: &str) -> &'static str {
    let stripped = text.trim();
    if stripped.ends_with("?!") || stripped.ends_with("！？") || stripped.ends_with("？！") {
        return "?!";
    }
    if stripped.ends_with("?.") || stripped.ends_with("。？") || stripped.ends_with("？。") {
        return "?.";
    }
    if stripped.ends_with("?~") || stripped.ends_with("～？") || stripped.ends_with("？～") {
        return "?~";
    }
    if stripped.ends_with('?') || stripped.ends_with('？') {
        return "?";
    }
    "$"
}

fn nasal_variant(next_phoneme: Option<&str>) -> &'static str {
    match next_phoneme {
        None => "N_uvular",
        Some(phone) => {
            if matches!(phone, "m" | "my" | "b" | "by" | "p" | "py") {
                "N_m"
            } else if matches!(phone, "n" | "ny" | "t" | "ty" | "d" | "dy" | "ts" | "ch") {
                "N_n"
            } else if matches!(phone, "k" | "ky" | "kw" | "g" | "gy" | "gw") {
                "N_ng"
            } else {
                "N_uvular"
            }
        }
    }
}

fn apply_n_phoneme_rules(phonemes: &mut [Phoneme]) {
    let mut next_phoneme: Option<String> = None;
    for phoneme in phonemes.iter_mut().rev() {
        let token = phoneme.token.as_str();
        if is_skip_token(token) {
            continue;
        }
        if token == "N" {
            let variant = nasal_variant(next_phoneme.as_deref());
            phoneme.token = variant.to_string();
            next_phoneme = Some(variant.to_string());
        } else {
            next_phoneme = Some(token.to_string());
        }
    }
}

pub fn phonemize_labels(question: &str, labels: &[String]) -> Vec<Phoneme> {
    let label_count = labels.len();
    let mut phonemes: Vec<Phoneme> = Vec::new();

    for (label_index, label) in labels.iter().enumerate() {
        let phone = match phone_pattern().captures(label) {
            Some(captures) => captures
                .get(1)
                .map(|value| value.as_str().to_string())
                .unwrap_or_default(),
            None => continue,
        };
        if phone.is_empty() {
            continue;
        }

        if phone == "sil" {
            if (label_index == label_count - 1) && (question != "$") {
                phonemes.push(Phoneme {
                    token: question.to_string(),
                    prosody: None,
                });
            }
            continue;
        }

        if phone == "pau" {
            phonemes.push(Phoneme {
                token: "_".to_string(),
                prosody: None,
            });
            continue;
        }

        let parsed = match prosody_pattern().captures(label) {
            Some(captures) => {
                let a1 = captures
                    .get(1)
                    .and_then(|value| value.as_str().parse::<i32>().ok());
                let a2 = captures
                    .get(2)
                    .and_then(|value| value.as_str().parse::<i32>().ok());
                let a3 = captures
                    .get(3)
                    .and_then(|value| value.as_str().parse::<i32>().ok());
                match (a1, a2, a3) {
                    (Some(a1), Some(a2), Some(a3)) => Some((a1, a2, a3)),
                    _ => None,
                }
            }
            None => None,
        };

        phonemes.push(Phoneme {
            token: phone,
            prosody: parsed,
        });

        if let Some((a1, a2, a3)) = parsed {
            let a2_next = if label_index + 1 < label_count {
                a2_of(&labels[label_index + 1]).unwrap_or(-1)
            } else {
                -1
            };

            if (a1 == 0) && (a2_next == a2 + 1) {
                phonemes.push(Phoneme {
                    token: "]".to_string(),
                    prosody: None,
                });
            }
            if (a2 == a3) && (a2_next == 1) {
                phonemes.push(Phoneme {
                    token: "#".to_string(),
                    prosody: None,
                });
            }
            if (a2 == 1) && (a2_next == 2) {
                phonemes.push(Phoneme {
                    token: "[".to_string(),
                    prosody: None,
                });
            }
        }
    }

    apply_n_phoneme_rules(&mut phonemes);
    phonemes
}

pub fn phonemize_sentence(sentence: &str) -> Vec<Phoneme> {
    let labels = crate::labels(sentence);
    if labels.is_empty() {
        return Vec::new();
    }
    phonemize_labels(question_marker(sentence), &labels)
}

/// Phonemize the whole text in one pass (official wasm behaviour).
pub fn phonemize(text: &str) -> Vec<Vec<Phoneme>> {
    let phonemes = phonemize_sentence(text);
    if phonemes.is_empty() {
        return Vec::new();
    }
    vec![phonemes]
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn sentences_to_json(sentences: &[Vec<Phoneme>]) -> String {
    let mut output = String::from("{\"sentences\":[");
    for (sentence_index, sentence) in sentences.iter().enumerate() {
        if sentence_index > 0 {
            output.push(',');
        }
        output.push_str("{\"tokens\":[");
        for (phoneme_index, phoneme) in sentence.iter().enumerate() {
            if phoneme_index > 0 {
                output.push(',');
            }
            output.push('"');
            output.push_str(&escape_json(&phoneme.token));
            output.push('"');
        }
        output.push_str("],\"prosody\":[");
        for (phoneme_index, phoneme) in sentence.iter().enumerate() {
            if phoneme_index > 0 {
                output.push(',');
            }
            match phoneme.prosody {
                Some((a1, a2, a3)) => {
                    output.push_str(&format!("[{a1},{a2},{a3}]"));
                }
                None => output.push_str("null"),
            }
        }
        output.push_str("]}");
    }
    output.push_str("]}");
    output
}

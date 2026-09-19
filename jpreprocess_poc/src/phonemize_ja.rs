//! Port of piper1-gpl's `phonemize_japanese.py` label parser.
//!
//! Reference implementation (MIT/AGPL piper1-gpl):
//!   C:\_github\piper1-gpl-main\src\piper\phonemize_japanese.py
//!
//! Input : OpenJTalk full-context labels (produced by jpreprocess)
//! Output: IPA phonemes with prosody symbols (↑ ↓ # , . ?), one Vec per sentence.
//!
//! Keep this file in sync with the Python implementation; the golden test is
//! dev_tools/tests/jpreprocess/downloaded/expected_ipa.txt.

use regex::Regex;
use std::sync::OnceLock;

// Prosody symbols (all of them have ids in the default phoneme id map).
pub const ACCENT_RISE: char = '↑';
pub const ACCENT_FALL: char = '↓';
pub const ACCENT_PHRASE_BOUNDARY: char = '#';
pub const PAUSE: char = ',';
pub const DECLARATIVE_END: char = '.';
pub const INTERROGATIVE_END: char = '?';

const NO_FEATURE: i32 = -50;

fn is_prosody_symbol(phone: &str) -> bool {
    matches!(phone, "↑" | "↓" | "#" | "," | "." | "?")
}

/// OpenJTalk phone -> IPA (same table as the Python implementation).
fn openjtalk_to_ipa(phone: &str) -> Option<&'static str> {
    let ipa = match phone {
        // Vowels
        "a" => "a",
        "i" => "i",
        "u" => "ɯ", // compressed close back unrounded, not [u]
        "e" => "e",
        "o" => "o",
        // Consonants
        "k" => "k",
        "ky" => "kʲ",
        "kw" => "kʷ",
        "g" => "ɡ",
        "gy" => "ɡʲ",
        "gw" => "ɡʷ",
        "s" => "s",
        "sh" => "ɕ",
        "z" => "z",
        "j" => "dʑ",
        "t" => "t",
        "ts" => "ts",
        "ty" => "tʲ",
        "ch" => "tɕ",
        "d" => "d",
        "dy" => "dʲ",
        "n" => "n",
        "ny" => "nʲ",
        "h" => "h",
        "hy" => "hʲ",
        "f" => "ɸ",
        "b" => "b",
        "by" => "bʲ",
        "p" => "p",
        "py" => "pʲ",
        "m" => "m",
        "my" => "mʲ",
        "y" => "j",
        "r" => "ɾ",
        "ry" => "ɾʲ",
        "w" => "w",
        "v" => "v",
        // Specials
        "N" => "ɴ",  // moraic nasal
        "cl" => "ʔ", // geminate closure
        _ => return None,
    };
    Some(ipa)
}

fn is_devoiced_vowel(phone: &str) -> bool {
    matches!(phone, "A" | "I" | "U" | "E" | "O")
}

/// Vowels and moraic phones that can carry an accent phrase boundary.
fn is_mora_final_phone(phone: &str) -> bool {
    matches!(
        phone,
        "a" | "i" | "u" | "e" | "o" | "A" | "I" | "U" | "E" | "O" | "N" | "cl"
    )
}

// ---------------------------------------------------------------------------
// sentence splitting
// ---------------------------------------------------------------------------
fn sentence_end_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"[。．！？!?]+[」』）】〉》”’"')\]\s]*"#).unwrap())
}

/// Split text into sentences, keeping the final punctuation (piper's `_split_sentences`).
pub fn split_sentences(text: &str) -> Vec<String> {
    // `_is_decimal_point` inspects neighbours, so work on codepoints.
    let chars: Vec<char> = text.chars().collect();
    let mut sentences = Vec::new();
    let mut start = 0usize;

    for m in sentence_end_pattern().find_iter(text) {
        // char index of the match start
        let char_start = text[..m.start()].chars().count();
        let char_end = text[..m.end()].chars().count();

        // "1.5" is not a sentence boundary
        if m.as_str() == "." {
            let before = if char_start > 0 { chars[char_start - 1] } else { ' ' };
            let after = if char_end < chars.len() { chars[char_end] } else { ' ' };
            if before.is_ascii_digit() && after.is_ascii_digit() {
                continue;
            }
        }

        let sentence: String = chars[start..char_end].iter().collect();
        let sentence = sentence.trim();
        if !sentence.is_empty() {
            sentences.push(sentence.to_string());
        }
        start = char_end;
    }

    if start < chars.len() {
        let tail: String = chars[start..].iter().collect();
        let tail = tail.trim();
        if !tail.is_empty() {
            sentences.push(tail.to_string());
        }
    }

    sentences
}

// ---------------------------------------------------------------------------
// label parsing
// ---------------------------------------------------------------------------
fn current_phone_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\-(.*?)\+").unwrap())
}

/// Extract an integer field from a full-context label (piper's `_numeric_feature`).
fn numeric_feature(pattern: &Regex, label: &str) -> i32 {
    match pattern.captures(label) {
        Some(caps) => match caps.get(1).and_then(|m| m.as_str().parse::<i32>().ok()) {
            Some(value) => value,
            None => NO_FEATURE, // unset fields are written "xx"
        },
        None => NO_FEATURE,
    }
}

fn feature_pattern(name: &str) -> &'static Regex {
    match name {
        "a1" => {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new(r"/A:([0-9\-]+)\+").unwrap())
        }
        "a2" => {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new(r"\+(\d+)\+").unwrap())
        }
        "a3" => {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new(r"\+(\d+)/").unwrap())
        }
        "f1" => {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new(r"/F:(\d+)_").unwrap())
        }
        "e3" => {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new(r"!(\d+)_").unwrap())
        }
        _ => unreachable!(),
    }
}

/// Parse one sentence's full-context labels into phones + prosody symbols
/// (piper's `JapanesePhonemizer._phonemize_sentence`).
pub fn labels_to_phones(labels: &[String], drop_devoiced_vowels: bool) -> Vec<String> {
    let num_labels = labels.len();
    let mut phones: Vec<String> = Vec::new();

    for (label_idx, label) in labels.iter().enumerate() {
        let phone = match current_phone_pattern().captures(label) {
            Some(caps) => caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
            None => continue,
        };
        if phone.is_empty() {
            continue;
        }

        let phone = if drop_devoiced_vowels && is_devoiced_vowel(&phone) {
            phone.to_lowercase()
        } else {
            phone
        };

        if phone == "sil" {
            // Utterance boundary: only the trailing one carries the intonation.
            if label_idx == (num_labels - 1) {
                let is_question = numeric_feature(feature_pattern("e3"), label) == 1;
                phones.push(
                    if is_question { INTERROGATIVE_END } else { DECLARATIVE_END }.to_string(),
                );
            }
            continue;
        }

        if phone == "pau" {
            phones.push(PAUSE.to_string());
            continue;
        }

        phones.push(phone.clone());

        if label_idx >= (num_labels - 1) {
            // No following label to compare the accent position against.
            continue;
        }

        let a1 = numeric_feature(feature_pattern("a1"), label);
        let a2 = numeric_feature(feature_pattern("a2"), label);
        let a3 = numeric_feature(feature_pattern("a3"), label);
        let f1 = numeric_feature(feature_pattern("f1"), label);
        let a2_next = numeric_feature(feature_pattern("a2"), &labels[label_idx + 1]);

        if (a3 == 1) && (a2_next == 1) && is_mora_final_phone(&phone) {
            // Last mora of this accent phrase, first mora of the next.
            phones.push(ACCENT_PHRASE_BOUNDARY.to_string());
        } else if (a1 == 0) && (a2_next == (a2 + 1)) && (a2 != f1) {
            // Accent nucleus and the phrase continues: pitch falls after.
            phones.push(ACCENT_FALL.to_string());
        } else if (a2 == 1) && (a2_next == 2) {
            // Mora 1 -> 2 of an accent phrase: pitch rises.
            phones.push(ACCENT_RISE.to_string());
        }
    }

    phones
}

/// OpenJTalk phones -> IPA phonemes (piper's `JapanesePhonemizer.phonemize`).
/// Multi-symbol IPA (kʲ, tɕ, dʑ, ...) becomes separate phonemes.
pub fn phones_to_ipa(phones: &[String]) -> Vec<String> {
    let mut ipa_phonemes: Vec<String> = Vec::new();

    for phone in phones {
        if is_prosody_symbol(phone) {
            ipa_phonemes.push(phone.clone());
            continue;
        }

        let ipa = match openjtalk_to_ipa(phone) {
            Some(ipa) => ipa,
            None => {
                eprintln!("[ja] no IPA mapping for OpenJTalk phone: {phone}");
                continue;
            }
        };

        for c in ipa.chars() {
            ipa_phonemes.push(c.to_string());
        }
    }

    ipa_phonemes
}

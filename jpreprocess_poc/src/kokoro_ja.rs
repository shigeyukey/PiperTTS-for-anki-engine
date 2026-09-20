//! Kokoro phonemes for japanese text.
//!
//! The japanese Kokoro voices were trained with misaki's cutlet tokenizer
//! (https://github.com/hexgrad/misaki, Apache-2.0, based on polm/cutlet, MIT),
//! so their phoneme alphabet is not piper's IPA. Both engines share the
//! OpenJTalk front end (see [`crate::phonemize_ja`]), so this module only has to
//! rewrite the phone sequence:
//!
//!   - piper's prosody symbols are dropped (the model gets the prosody from the
//!     phoneme sequence itself), a long vowel (":") becomes "ː"
//!   - わ is "β", the vowel after s / z / ts is "ɨ" instead of "ɯ"
//!   - ん becomes m / ŋ / ɲ / n / ɴ depending on the following phone
//!     (cutlet's context rule)
//!   - an accent phrase boundary becomes a space, a pause becomes ", "
//!     (misaki separates words and writes a space after sentence stops)
//!
//! Every codepoint the module emits exists in the Kokoro vocab.

use crate::phonemize_ja::{
    labels_to_phones, split_sentences, ACCENT_FALL, ACCENT_PHRASE_BOUNDARY, ACCENT_RISE,
    DECLARATIVE_END, INTERROGATIVE_END, PAUSE,
};

/// OpenJTalk phone -> Kokoro phoneme (cutlet spelling).
fn phone_to_phoneme(phone: &str) -> Option<&'static str> {
    let phoneme = match phone {
        // vowels
        "a" => "a",
        "i" => "i",
        "u" => "ɯ",
        "e" => "e",
        "o" => "o",
        // consonants
        "k" => "k",
        "ky" => "kʲ",
        "kw" => "kᵝ",
        "g" => "ɡ",
        "gy" => "ɡʲ",
        "gw" => "ɡᵝ",
        "s" => "s",
        "sh" => "ɕ",
        "z" => "z",
        "j" => "ʥ",
        "t" => "t",
        "ts" => "ʦ",
        "ty" => "tʲ",
        "ch" => "ʨ",
        "d" => "d",
        "dy" => "dʲ",
        "n" => "n",
        "ny" => "ɲ",
        "h" => "h",
        "hy" => "ç",
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
        "w" => "β",
        "v" => "v",
        // specials
        "cl" => "ʔ",
        ":" => "ː",
        _ => return None,
    };
    Some(phoneme)
}

/// True for the consonants that turn the following "u" into "ɨ" (す ず つ).
fn makes_central_vowel(phone: &str) -> bool {
    matches!(phone, "s" | "z" | "ts")
}

/// True for the prosody symbols that are not part of the phoneme sequence.
fn is_skipped_symbol(phone: &str) -> bool {
    matches!(phone, ACCENT_RISE | ACCENT_FALL | ACCENT_PHRASE_BOUNDARY)
}

/// True for symbols that end the phoneme sequence (nothing follows them).
fn is_boundary_symbol(phone: &str) -> bool {
    matches!(
        phone,
        PAUSE | DECLARATIVE_END | INTERROGATIVE_END | "pau" | "sil"
    )
}

/// The vowel cutlet uses for ん before `next_phone`.
///
/// cutlet checks the *phoneme* that follows: m before m/p/b, ŋ before k/ɡ,
/// ɲ before ɲ/ʨ/ʥ, n before n/t/d/ɾ/z, ɴ in every other case.
fn nasal_vowel(next_phone: &str) -> &'static str {
    match next_phone {
        "m" | "my" | "p" | "py" | "b" | "by" => "m",
        "k" | "ky" | "kw" | "g" | "gy" | "gw" => "ŋ",
        "ny" | "ch" | "j" => "ɲ",
        "n" | "t" | "ty" | "d" | "dy" | "r" | "ry" | "z" => "n",
        _ => "ɴ",
    }
}

/// The first phone after `index` that is part of the phoneme sequence
/// (prosody symbols are skipped, a pause or a sentence end counts as "nothing").
fn following_phone<'a>(phones: &'a [String], index: usize) -> &'a str {
    for phone in phones.iter().skip(index + 1) {
        let phone = phone.as_str();
        if is_skipped_symbol(phone) {
            continue;
        }
        if is_boundary_symbol(phone) {
            return "";
        }
        return phone;
    }
    ""
}

fn push_space(text: &mut String) {
    if !text.is_empty() && !text.ends_with(' ') {
        text.push(' ');
    }
}

/// Rewrite one sentence's OpenJTalk phones into Kokoro phonemes.
pub fn phonemize_sentence(phones: &[String]) -> String {
    let mut text = String::new();
    let mut previous: Option<&str> = None;

    for (index, phone) in phones.iter().enumerate() {
        let current = phone.as_str();

        if current == ACCENT_RISE || current == ACCENT_FALL {
            continue;
        }
        if current == ACCENT_PHRASE_BOUNDARY {
            push_space(&mut text);
            previous = None;
            continue;
        }
        if current == PAUSE {
            text.push(',');
            text.push(' ');
            previous = None;
            continue;
        }
        if current == DECLARATIVE_END || current == INTERROGATIVE_END {
            text.push_str(current);
            text.push(' ');
            previous = None;
            continue;
        }

        if current == "N" {
            text.push_str(nasal_vowel(following_phone(phones, index)));
            previous = Some(current);
            continue;
        }

        let phoneme = match phone_to_phoneme(current) {
            Some(phoneme) => phoneme,
            None => {
                // unknown phone (the same table decides what piper sees)
                continue;
            }
        };

        if phoneme == "ɯ" {
            if let Some(previous_phone) = previous {
                if makes_central_vowel(previous_phone) {
                    text.push('ɨ');
                    previous = Some(current);
                    continue;
                }
            }
        }

        text.push_str(phoneme);
        previous = Some(current);
    }

    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Phonemize japanese text for the Kokoro engine: one string per sentence.
pub fn phonemize(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();

    for sentence in split_sentences(text) {
        let labels = crate::labels(&sentence);
        if labels.is_empty() {
            continue;
        }
        let phones = labels_to_phones(&labels, true);
        let converted = phonemize_sentence(&phones);
        if !converted.is_empty() {
            sentences.push(converted);
        }
    }

    sentences
}

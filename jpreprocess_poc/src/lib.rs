//! jpreprocess PoC for the PiperTTS V2 Anki add-on.
//!
//! - `phonemize_ja`: port of piper1-gpl's label parser (labels -> IPA + prosody)
//! - `phonemize`    : full pipeline (text -> OpenJTalk labels -> IPA), one
//!                    string per sentence, ready to be mapped to phoneme ids
//!                    with the voice's `phoneme_id_map`.
//! - `kokoro_ja`    : the same front end rewritten into the phoneme alphabet of
//!                    the Kokoro japanese voices (misaki/cutlet spelling)
//!
//! The Japanese piper voice model was trained on exactly this IPA
//! representation, so the result matches piper1-gpl's `phoneme_type:
//! "japanese"` output.

pub mod kokoro_ja;
pub mod phonemize_ja;
pub mod piper_plus_ja;

use jpreprocess::kind::JPreprocessDictionaryKind;
use jpreprocess::{DefaultTokenizer, JPreprocess, SystemDictionaryConfig};
use std::cell::RefCell;

thread_local! {
    static J_PREPROCESS: RefCell<Option<JPreprocess<DefaultTokenizer>>> =
        const { RefCell::new(None) };
}

/// Load the bundled naist-jdic dictionary into this thread (idempotent).
pub fn init() -> bool {
    J_PREPROCESS.with(|cell| {
        if cell.borrow().is_some() {
            return true;
        }

        let dictionary =
            match SystemDictionaryConfig::Bundled(JPreprocessDictionaryKind::NaistJdic).load() {
                Ok(dictionary) => dictionary,
                Err(e) => {
                    eprintln!("[ja] dictionary load failed: {e:?}");
                    return false;
                }
            };

        *cell.borrow_mut() = Some(JPreprocess::with_dictionaries(dictionary, None));
        true
    })
}

/// Full-context labels of one sentence.
pub fn labels(text: &str) -> Vec<String> {
    if !init() {
        return Vec::new();
    }

    J_PREPROCESS.with(|cell| {
        let borrowed = cell.borrow();
        let jp = match borrowed.as_ref() {
            Some(jp) => jp,
            None => return Vec::new(),
        };

        match jp.extract_fullcontext(text) {
            Ok(labels) => labels.iter().map(|label| label.to_string()).collect(),
            Err(e) => {
                eprintln!("[ja] extract_fullcontext failed: {e:?}");
                Vec::new()
            }
        }
    })
}

/// Phonemize Japanese text: one IPA string (with prosody symbols) per sentence.
pub fn phonemize(text: &str) -> Vec<String> {
    if !init() {
        return Vec::new();
    }

    let mut sentences = Vec::new();
    for sentence in phonemize_ja::split_sentences(text) {
        let labels = labels(&sentence);
        if labels.is_empty() {
            continue;
        }

        let phones = phonemize_ja::labels_to_phones(&labels, true);
        let ipa = phonemize_ja::phones_to_ipa(&phones);
        if !ipa.is_empty() {
            sentences.push(ipa.join(""));
        }
    }

    sentences
}

/// Phonemize japanese text for the Kokoro engine: one phoneme string per
/// sentence, written in the alphabet the japanese Kokoro voices were trained
/// on (misaki/cutlet). See [`kokoro_ja`].
pub fn kokoro_phonemes(text: &str) -> Vec<String> {
    if !init() {
        return Vec::new();
    }
    kokoro_ja::phonemize(text)
}

/// Phonemize japanese text for the piper-plus engine: one token list with
/// prosody per sentence (see [`piper_plus_ja`]).
pub fn piper_plus_phonemes(text: &str) -> Vec<Vec<piper_plus_ja::Phoneme>> {
    if !init() {
        return Vec::new();
    }
    piper_plus_ja::phonemize(text)
}


// ---------------------------------------------------------------------------
// C API (used by the QWebEngine side through Emscripten)
// ---------------------------------------------------------------------------
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Load the dictionary now (0 = success). Optional: `phonemize` loads lazily.
#[no_mangle]
pub extern "C" fn ja_init() -> i32 {
    if init() {
        0
    } else {
        1
    }
}

/// Phonemize Japanese `text` and return a NUL terminated UTF-8 string with one
/// IPA sentence per line ("" when nothing could be phonemized).
/// The returned pointer must be released with [`ja_free`].
#[no_mangle]
pub extern "C" fn ja_phonemize(text: *const c_char) -> *mut c_char {
    if text.is_null() {
        return CString::new("").unwrap().into_raw();
    }

    let text = unsafe { CStr::from_ptr(text) };
    let text = text.to_string_lossy();

    let joined = phonemize(&text).join("\n");
    match CString::new(joined) {
        Ok(s) => s.into_raw(),
        Err(_) => CString::new("").unwrap().into_raw(),
    }
}

/// Phonemize japanese `text` for the Kokoro engine and return a NUL terminated
/// UTF-8 string with one phoneme sentence per line ("" when nothing could be
/// phonemized). The returned pointer must be released with [`ja_free`].
#[no_mangle]
pub extern "C" fn ja_kokoro_phonemes(text: *const c_char) -> *mut c_char {
    if text.is_null() {
        return CString::new("").unwrap().into_raw();
    }

    let text = unsafe { CStr::from_ptr(text) };
    let text = text.to_string_lossy();

    let joined = kokoro_phonemes(&text).join("\n");
    match CString::new(joined) {
        Ok(s) => s.into_raw(),
        Err(_) => CString::new("").unwrap().into_raw(),
    }
}

/// Phonemize japanese `text` for the piper-plus engine and return the tokens
/// and prosody as a JSON string ("" when nothing could be phonemized).
/// The returned pointer must be released with [`ja_free`].
#[no_mangle]
pub extern "C" fn ja_piper_plus_phonemes(text: *const c_char) -> *mut c_char {
    if text.is_null() {
        return CString::new("").unwrap().into_raw();
    }

    let text = unsafe { CStr::from_ptr(text) };
    let text = text.to_string_lossy();

    let json = piper_plus_ja::sentences_to_json(&piper_plus_phonemes(&text));
    match CString::new(json) {
        Ok(value) => value.into_raw(),
        Err(_) => CString::new("").unwrap().into_raw(),
    }
}

/// Release a string returned by [`ja_phonemize`], [`ja_kokoro_phonemes`] or
/// [`ja_piper_plus_phonemes`].
#[no_mangle]
pub extern "C" fn ja_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

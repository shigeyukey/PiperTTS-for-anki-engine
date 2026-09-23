//! piper-plus Japanese phonemizer module (jpreprocess 0.9.1, bundled naist-jdic).
//!
//! Dedicated to the piper-plus engine: the official piper-plus wasm build
//! (`piper-plus-wasm` 0.5.0) uses jpreprocess 0.9.1. The shared module built
//! from dev_tools/jpreprocess_poc (jpreprocess 0.15) is used by piper / kokoro
//! and must never be changed.
//!
//! C API (used by the QWebEngine side through Emscripten):
//!   `ja_init` / `ja_piper_plus_phonemes` / `ja_free`

pub mod piper_plus_ja;

use jpreprocess::kind::JPreprocessDictionaryKind;
use jpreprocess::{JPreprocess, SystemDictionaryConfig};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub type JPreprocessInstance = JPreprocess<jpreprocess::DefaultFetcher>;

thread_local! {
    static J_PREPROCESS: RefCell<Option<JPreprocessInstance>> = const { RefCell::new(None) };
}

fn create_instance() -> Result<JPreprocessInstance, String> {
    let config = jpreprocess::JPreprocessConfig {
        dictionary: SystemDictionaryConfig::Bundled(JPreprocessDictionaryKind::NaistJdic),
        user_dictionary: None,
    };
    JPreprocess::from_config(config).map_err(|error| error.to_string())
}

/// Load the bundled naist-jdic dictionary into this thread (idempotent).
pub fn init() -> bool {
    J_PREPROCESS.with(|cell| {
        if cell.borrow().is_some() {
            return true;
        }

        let instance = match create_instance() {
            Ok(value) => value,
            Err(error) => {
                eprintln!("[piper_plus_ja] dictionary load failed: {error}");
                return false;
            }
        };

        *cell.borrow_mut() = Some(instance);
        true
    })
}

/// Full-context labels of one text.
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
            Err(error) => {
                eprintln!("[piper_plus_ja] extract_fullcontext failed: {error:?}");
                Vec::new()
            }
        }
    })
}

/// Phonemize japanese text for the piper-plus engine: token lists with prosody
/// (one entry for the whole text, matching the official wasm build).
pub fn piper_plus_phonemes(text: &str) -> Vec<Vec<piper_plus_ja::Phoneme>> {
    if !init() {
        return Vec::new();
    }
    piper_plus_ja::phonemize(text)
}

// ---------------------------------------------------------------------------
// C API (Emscripten exports)
// ---------------------------------------------------------------------------

/// Load the dictionary now (0 = success). Optional: phonemize loads lazily.
#[no_mangle]
pub extern "C" fn ja_init() -> i32 {
    if init() {
        0
    } else {
        1
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

/// Release a string returned by [`ja_piper_plus_phonemes`].
#[no_mangle]
pub extern "C" fn ja_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

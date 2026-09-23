// Native CLI for the piper-plus phonemizer (jpreprocess 0.9.1).
//
//   cargo run --release                    -> built-in sample texts
//   cargo run --release -- piper-plus      -> one JSON per stdin line
//   cargo run --release -- labels          -> full-context labels per stdin line
//
// For wasm (emscripten) this binary is only a container for the library
// exports, so main() does nothing.

#[cfg(target_os = "emscripten")]
fn main() {
    // The C API lives in the library; reference it so the linker keeps it even
    // with LTO + dead code elimination.
    let keep_alive: [*const (); 3] = [
        jpreprocess_poc::ja_init as *const (),
        jpreprocess_poc::ja_piper_plus_phonemes as *const (),
        jpreprocess_poc::ja_free as *const (),
    ];
    std::hint::black_box(keep_alive);
}

#[cfg(not(target_os = "emscripten"))]
mod cli {
    use std::io::{BufRead, Write};

    const TEXTS: [&str; 4] = [
        "こんにちは",
        "今日はいい天気です",
        "すもももももももものうち",
        "明日、東京へ行きますか？",
    ];

    pub fn run() {
        let mode = std::env::args().nth(1).unwrap_or_default();

        if !jpreprocess_poc::init() {
            eprintln!("[piper_plus_ja] dictionary load failed");
            std::process::exit(1);
        }

        if mode == "labels" {
            run_labels();
            return;
        }

        if mode == "piper-plus" {
            run_piper_plus();
            return;
        }

        for text in TEXTS {
            let json = jpreprocess_poc::piper_plus_ja::sentences_to_json(
                &jpreprocess_poc::piper_plus_phonemes(text),
            );
            println!("{json}");
        }
    }

    fn run_piper_plus() {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("[piper_plus_ja] stdin error: {error}");
                    std::process::exit(1);
                }
            };
            let text = line.trim_end_matches(['\r', '\n']);
            if text.is_empty() {
                continue;
            }
            let json = jpreprocess_poc::piper_plus_ja::sentences_to_json(
                &jpreprocess_poc::piper_plus_phonemes(text),
            );
            if writeln!(output, "{json}").is_err() {
                std::process::exit(1);
            }
        }
    }

    fn run_labels() {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("[piper_plus_ja] stdin error: {error}");
                    std::process::exit(1);
                }
            };
            let text = line.trim_end_matches(['\r', '\n']);
            if text.is_empty() {
                continue;
            }
            println!("=== {text} ===");
            for label in jpreprocess_poc::labels(text) {
                if writeln!(output, "{label}").is_err() {
                    std::process::exit(1);
                }
            }
        }
    }
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    cli::run();
}

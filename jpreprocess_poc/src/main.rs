// Native CLI for the PoC.
//
//   cargo run --release            -> IPA (golden test against the reference in
//                                    ..\tests\jpreprocess\downloaded\expected_ipa.txt)
//   cargo run --release -- labels  -> OpenJTalk full-context labels
//                                    (capture: ... labels > ..\tests\jpreprocess\output\labels_native.txt)
//   cargo run --release -- piper-plus         -> piper-plus tokens from stdin texts
//   cargo run --release -- piper-plus-labels  -> piper-plus tokens from stdin labels
//                                    (record = one text line, label lines, blank line)
//
// For wasm (emscripten) this binary is only a container for the library
// exports (`ja_init` / `ja_phonemize` / `ja_free`), so main() does nothing.

#[cfg(target_os = "emscripten")]
fn main() {
    // The C API lives in the library; reference it so the linker keeps it even
    // with LTO + dead code elimination.
    let keep_alive: [*const (); 5] = [
        jpreprocess_poc::ja_init as *const (),
        jpreprocess_poc::ja_phonemize as *const (),
        jpreprocess_poc::ja_kokoro_phonemes as *const (),
        jpreprocess_poc::ja_piper_plus_phonemes as *const (),
        jpreprocess_poc::ja_free as *const (),
    ];
    std::hint::black_box(keep_alive);
}

#[cfg(not(target_os = "emscripten"))]
mod cli {
    use jpreprocess_poc::phonemize_ja::split_sentences;
    use jpreprocess_poc::{init, labels, phonemize};

    const TEXTS: [&str; 4] = [
        "こんにちは",
        "今日はいい天気です",
        "すもももももももものうち",
        "明日、東京へ行きますか？",
    ];

    pub fn run() {
        let mode = std::env::args().nth(1).unwrap_or_default();
        let show_labels = mode == "labels";

        if mode == "piper-plus" {
            run_piper_plus();
            return;
        }

        if mode == "labels-stdin" {
            run_labels_stdin();
            return;
        }

        if mode == "piper-plus-labels" {
            run_piper_plus_labels();
            return;
        }

        println!("[poc] loading the bundled naist-jdic dictionary ...");
        if !init() {
            eprintln!("[poc] dictionary load failed");
            std::process::exit(1);
        }
        println!("[poc] dictionary loaded");

        for text in TEXTS {
            if show_labels {
                println!("=== {text} ===");
                for sentence in split_sentences(text) {
                    for label in labels(&sentence) {
                        println!("{label}");
                    }
                }
                continue;
            }

            for ipa in phonemize(text) {
                let phones = ipa
                    .chars()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("=== {text} ===");
                println!("phones: {phones}");
                println!("IPA   : {ipa}");
                println!();
            }
        }

        println!("[poc] done");
    }

    fn run_piper_plus() {
        use std::io::BufRead;
        use std::io::Write;

        if !init() {
            eprintln!("[poc] dictionary load failed");
            std::process::exit(1);
        }

        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("[poc] stdin error: {e}");
                    std::process::exit(1);
                }
            };
            let text = line.trim_end_matches(['\r', '\n']);
            if text.is_empty() {
                continue;
            }
            let sentences = jpreprocess_poc::piper_plus_phonemes(text);
            let json = jpreprocess_poc::piper_plus_ja::sentences_to_json(&sentences);
            if writeln!(output, "{json}").is_err() {
                std::process::exit(1);
            }
        }
    }

    fn run_piper_plus_labels() {
        use std::io::BufRead;

        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        let mut text = String::new();
        let mut labels: Vec<String> = Vec::new();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("[poc] stdin error: {e}");
                    std::process::exit(1);
                }
            };
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                write_piper_plus_record(&mut output, &mut text, &mut labels);
                continue;
            }
            if text.is_empty() {
                text.push_str(trimmed);
                continue;
            }
            labels.push(trimmed.to_string());
        }
        write_piper_plus_record(&mut output, &mut text, &mut labels);
    }

    fn write_piper_plus_record(
        output: &mut impl std::io::Write,
        text: &mut String,
        labels: &mut Vec<String>,
    ) {
        if text.is_empty() && labels.is_empty() {
            return;
        }
        let phonemes = jpreprocess_poc::piper_plus_ja::phonemize_labels(
            jpreprocess_poc::piper_plus_ja::question_marker(text),
            labels,
        );
        let json = jpreprocess_poc::piper_plus_ja::sentences_to_json(&[phonemes]);
        writeln!(output, "{json}").ok();
        text.clear();
        labels.clear();
    }

    fn run_labels_stdin() {
        use std::io::BufRead;
        use std::io::Write;

        if !init() {
            eprintln!("[poc] dictionary load failed");
            std::process::exit(1);
        }

        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("[poc] stdin error: {e}");
                    std::process::exit(1);
                }
            };
            let text = line.trim_end_matches(['\r', '\n']);
            if text.is_empty() {
                continue;
            }
            writeln!(output, "=== {text} ===").ok();
            for label in labels(text) {
                writeln!(output, "{label}").ok();
            }
        }
    }
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    cli::run();
}

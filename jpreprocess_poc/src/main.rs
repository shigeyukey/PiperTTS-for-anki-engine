// Native CLI for the PoC.
//
//   cargo run --release            -> IPA (golden test against the reference in
//                                    ..\tests\jpreprocess\downloaded\expected_ipa.txt)
//   cargo run --release -- labels  -> OpenJTalk full-context labels
//                                    (capture: ... labels > ..\tests\jpreprocess\output\labels_native.txt)
//
// For wasm (emscripten) this binary is only a container for the library
// exports (`ja_init` / `ja_phonemize` / `ja_free`), so main() does nothing.

#[cfg(target_os = "emscripten")]
fn main() {
    // The C API lives in the library; reference it so the linker keeps it even
    // with LTO + dead code elimination.
    let keep_alive: [*const (); 3] = [
        jpreprocess_poc::ja_init as *const (),
        jpreprocess_poc::ja_phonemize as *const (),
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
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    cli::run();
}

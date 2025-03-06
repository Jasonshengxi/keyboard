use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    num::NonZeroU8,
    path::{Component, Path, PathBuf},
};

use walkdir::WalkDir;

use crate::in_alphabet;

#[derive(Default)]
pub struct NGramTracker {
    last: [Option<NonZeroU8>; 2],
}

impl NGramTracker {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn shift(&mut self, x: NonZeroU8) {
        let [_, b] = self.last;
        self.last = [b, Some(x)];
    }

    pub fn apply(&mut self, counter: &mut CountOutcome, c: NonZeroU8) {
        let [a, b] = self.last;
        counter.add_letter([c.into()]);
        if let Some(b) = b {
            counter.add_bigram([b.into(), c.into()]);
            if let Some(a) = a {
                counter.add_trigram([a.into(), b.into(), c.into()]);
            }
        }
        self.shift(c);
    }
}

pub type Letters = HashMap<[u8; 1], u32>;
pub type Bigrams = HashMap<[u8; 2], u32>;
pub type Trigrams = HashMap<[u8; 3], u32>;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CountOutcome {
    pub letter: Letters,
    pub bigrams: Bigrams,
    pub trigrams: Trigrams,
}

impl CountOutcome {
    pub fn add_letter(&mut self, letter: [u8; 1]) {
        let count = self.letter.entry(letter).or_insert(0);
        *count += 1;
    }

    pub fn add_bigram(&mut self, bigram: [u8; 2]) {
        let count = self.bigrams.entry(bigram).or_insert(0);
        *count += 1;
    }

    pub fn add_trigram(&mut self, trigram: [u8; 3]) {
        let count = self.trigrams.entry(trigram).or_insert(0);
        *count += 1;
    }
}

const CACHE_PATH: &str = "cache.bin";

#[derive(Debug)]
#[allow(unused)]
pub enum CacheFailReason {
    FileSystem(std::io::Error),
    Deserialize(bincode::Error),
    BadPath(PathBuf),
}

pub fn count(path: impl AsRef<Path>) -> (CountOutcome, Option<CacheFailReason>) {
    let path = path.as_ref();

    let cache_raw = std::fs::read(CACHE_PATH);
    let cache = cache_raw
        .map_err(CacheFailReason::FileSystem)
        .and_then(|data| {
            bincode::deserialize::<(PathBuf, CountOutcome)>(data.as_slice())
                .map_err(CacheFailReason::Deserialize)
        });

    let fail_reason = match cache {
        Ok((cached_path, cached)) if cached_path == path => return (cached, None),
        Ok((cached_path, _)) => Some(CacheFailReason::BadPath(cached_path)),
        Err(err) => Some(err),
    };

    let outcome = count_uncached(path);
    let data = (path.to_path_buf(), outcome);
    let ser = bincode::serialize(&data);
    let outcome = data.1;

    if let Ok(ser) = ser {
        let _ = std::fs::write(CACHE_PATH, ser);
    }
    (outcome, fail_reason)
}

fn count_uncached(path: impl AsRef<Path>) -> CountOutcome {
    let mut result = CountOutcome::default();

    for item in WalkDir::new(path) {
        let Ok(entry) = item else {
            continue;
        };

        let file_type = entry.file_type();
        if file_type.is_file() {
            let path = entry.path();
            let ext = path.extension();

            const IGNORE_COMPONENTS: [&str; 3] = ["target", "uiua", "uiua-main"];

            if path.components().any(|part| {
                IGNORE_COMPONENTS.iter().any(|&component| match part {
                    Component::Normal(part) => component == part,
                    _ => false,
                })
            }) {
                continue;
            }

            const INCLUDE_EXTENSIONS: [&str; 7] =
                ["rs", "wgsl", "glsl", "vert", "comp", "frag", "py"];

            if ext.is_some_and(|ext| INCLUDE_EXTENSIONS.iter().any(|&e| ext == e)) {
                let Ok(mut file) = File::open(entry.path()) else {
                    continue;
                };

                println!("counting {}...", path.display());

                let mut string = String::new();
                let Ok(_) = file.read_to_string(&mut string) else {
                    continue;
                };
                let string = string;
                let mut chars = string.chars();

                let mut tracker = NGramTracker::default();
                while let Some(ch) = chars.next() {
                    if ch == '\r' {
                        continue;
                    }
                    if ch == '\n' {
                        let mut spaces = 0;
                        while chars.next() == Some(' ') {
                            spaces += 1;
                        }
                        while spaces > 0 {
                            spaces -= 4;
                        }
                        tracker.apply(&mut result, NonZeroU8::new(b'\t').unwrap());
                        for _ in 0..spaces {
                            tracker.apply(&mut result, NonZeroU8::new(b' ').unwrap());
                        }
                    }

                    match u8::try_from(ch)
                        .ok()
                        .and_then(NonZeroU8::new)
                        .and_then(|x| in_alphabet(x.into()).then_some(x))
                    {
                        Some(ch) => tracker.apply(&mut result, ch),
                        None => tracker.clear(),
                    }
                }
            }
        }
    }

    result
}

use std::num::NonZeroU8;

use array_map::ArrayMap;
use derive_more::{Add, AddAssign, Sum};
use glam::Vec2;
use macro_rules_attribute::macro_rules_derive;
use rustc_hash::FxHashMap;

use crate::{
    counter::{Bigrams, CountOutcome, Letters, Trigrams},
    iter::OneIter,
    keyboard::{Finger, Hand, HandFinger, Keyboard},
    layout::{Behavior, Layout},
};

// terminology:
// - alternates: trigrams with 2 hand changes
// - rolls: trigrams with 1 hand change
// - redirects: trigram with 2 direction change and 0 hand change

// - same finger bigrams
// ? same hand bigrams
// + finger bias
// ? hand bias
// - lateral movement
// - home row jumping bigrams
// - redirects
// ? rolls
// ? alternates
// - redirects
// - staccato tax

#[derive(Debug, Clone, Copy)]
pub struct Evaluation {
    pub letter: LetterEval,
    pub bigram: BigramEval,
    pub trigram: TrigramEval,
}

impl std::ops::Div for Evaluation {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            letter: self.letter / rhs.letter,
            bigram: self.bigram / rhs.bigram,
            trigram: self.trigram / rhs.trigram,
        }
    }
}

impl std::ops::Mul<f32> for Evaluation {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            letter: self.letter * rhs,
            bigram: self.bigram * rhs,
            trigram: self.trigram * rhs,
        }
    }
}

macro_rules! multi_eval {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident {
            $($v_:vis $field:ident : $ty:ty),*
            $(,)?
        }
    ) =>{
        impl std::ops::Mul<f32> for $name {
            type Output = Self;

            fn mul(self, _rhs: f32) -> Self::Output {
                Self { $($field : self.$field * _rhs),* }
            }
        }

        impl std::ops::Div for $name {
            type Output = Self;

            fn div(self, _rhs: Self) -> Self {
                Self { $($field : self.$field / _rhs.$field),* }
            }
        }

        impl $name {
            pub const NAN: Self = Self::splat(f32::NAN);

            pub const fn splat(_x: f32) -> Self {
                Self { $($field: _x),* }
            }

            pub fn min(self, _other: Self) -> Self {
                Self { $($field : self.$field.min(_other.$field)),* }
            }
        }
    };
}

#[macro_rules_derive(multi_eval!)]
#[derive(Debug, Clone, Copy, Add, AddAssign, Sum)]
pub struct LetterEval {
    pub base_dist: f32,
    pub base_x: f32,
    pub base_y: f32,
}

#[macro_rules_derive(multi_eval!)]
#[derive(Debug, Clone, Copy, Add, AddAssign, Sum)]
pub struct BigramEval {
    pub sfb: f32,
    pub shb: f32,
    pub movement: f32,
    pub lateral: f32,
    pub vertical: f32,
    pub staccato: f32,
}

#[macro_rules_derive(multi_eval!)]
#[derive(Debug, Clone, Copy, Add, AddAssign, Sum)]
pub struct TrigramEval {
    pub redirects: f32,
    pub rolls: f32,
    pub alternates: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyCombo {
    shift: Option<usize>,
    layer: Option<usize>,
    key: usize,
}

impl KeyCombo {
    pub fn new(shift: Option<usize>, layer: Option<usize>, key: usize) -> Self {
        Self { shift, layer, key }
    }
}

#[derive(Debug)]
pub struct KeyboardLayout<'a> {
    keys: FxHashMap<u8, Vec<KeyCombo>>,
    base: ArrayMap<HandFinger, Vec2, 10>,
    kb: &'a Keyboard,
}

impl<'a> KeyboardLayout<'a> {
    pub fn generate(layout: &Layout, keyboard: &'a Keyboard) -> Result<Self, u8> {
        let shift_keys = layout
            .find_on_base(|b| b == Behavior::Shift)
            .collect::<Vec<_>>();

        let mut keys = FxHashMap::default();
        for &key in crate::ALPHABET {
            let real_key = match key {
                b'A'..=b'Z' => key.to_ascii_lowercase(),
                b'?' => b'/',
                _ => key,
            };
            let do_shift = key != real_key;

            let shift_keys: OneIter<_> = do_shift.then(|| shift_keys.iter().copied()).into();

            let mut combos = vec![];
            for final_key in layout.find_all_key(|layout_key| layout_key.get() == real_key) {
                let layer = final_key.layer();
                let layer_keys: OneIter<_> = NonZeroU8::new(layer)
                    .map(|l| layout.find_on_base(move |b| b == Behavior::Layer(l)))
                    .into();

                for layer_key in layer_keys {
                    'skip_key: for shift_key in shift_keys.clone() {
                        if let (Some(l), Some(s)) = (layer_key, shift_key) {
                            let overlap = layout.key(l.layer(), s.index());
                            if overlap.is_some() {
                                continue 'skip_key;
                            }
                        }

                        let mut fingers = ArrayMap::<HandFinger, bool, 10>::new([false; 10]);
                        for key in std::iter::once(final_key).chain(layer_key).chain(shift_key) {
                            match &mut fingers[keyboard.key(key.index()).finger()] {
                                true => continue 'skip_key,
                                a @ false => *a = true,
                            }
                        }

                        combos.push(KeyCombo::new(
                            shift_key.map(|x| x.index()),
                            layer_key.map(|x| x.index()),
                            final_key.index(),
                        ));
                    }
                }
            }

            if combos.is_empty() {
                return Err(key);
            }
            keys.insert(key, combos);
        }

        let mut base = ArrayMap::new([Vec2::NAN; 10]);
        for key in keyboard.keys() {
            if key.is_base() {
                base[key.finger()] = key.pos();
            }
        }

        Ok(Self {
            keys,
            base,
            kb: keyboard,
        })
    }

    pub fn key(&self, x: u8) -> &[KeyCombo] {
        &self.keys[&x]
    }
}

pub fn evaluate(info: &KeyboardLayout, count: &CountOutcome) -> Evaluation {
    Evaluation {
        letter: eval_letters(info, &count.letter),
        bigram: eval_bigrams(info, &count.bigrams),
        trigram: eval_trigrams(info, &count.trigrams),
    }
}

pub fn eval_letters(info: &KeyboardLayout, letters: &Letters) -> LetterEval {
    letters
        .iter()
        .map(|(&letter, &freq)| one_letter(info, letter) * freq as f32)
        .sum()
}

pub fn eval_bigrams(info: &KeyboardLayout, bigrams: &Bigrams) -> BigramEval {
    bigrams
        .iter()
        .map(|(&bigram, &freq)| one_bigram(info, bigram) * freq as f32)
        .sum()
}

pub fn eval_trigrams(info: &KeyboardLayout, trigrams: &Trigrams) -> TrigramEval {
    trigrams
        .iter()
        .map(|(&trigram, &freq)| one_trigram(info, trigram) * freq as f32)
        .sum()
}

fn convert_fingers(
    info: &KeyboardLayout,
    combo: &KeyCombo,
) -> ArrayMap<HandFinger, Option<Vec2>, 10> {
    let mut position = ArrayMap::<HandFinger, Option<Vec2>, 10>::new([None; 10]);
    for index in std::iter::once(combo.key)
        .chain(combo.shift)
        .chain(combo.layer)
    {
        let key = info.kb.key(index);
        position[key.finger()] = Some(key.pos());
    }
    position
}

pub fn one_letter(info: &KeyboardLayout, letter: [u8; 1]) -> LetterEval {
    one_letter_any(
        info,
        LetterEval::NAN,
        |info, [c]| {
            let h = convert_fingers(info, c);
            let base = info.base;
            let mut base_dist = 0.0;
            let mut base_x = 0.0;
            let mut base_y = 0.0;

            for (a, b) in h.values().zip(base.values()) {
                if let Some(a) = a {
                    let delta = (a - b).abs();
                    base_dist += delta.length();
                    base_x += delta.x;
                    base_y += delta.y;
                }
            }

            LetterEval {
                base_dist,
                base_x,
                base_y,
            }
        },
        LetterEval::min,
        letter,
    )
}

pub fn one_bigram(info: &KeyboardLayout, bigram: [u8; 2]) -> BigramEval {
    one_bigram_any(
        info,
        BigramEval::NAN,
        |info, [c1, c2]| {
            let h1 = convert_fingers(info, c1);
            let h2 = convert_fingers(info, c2);

            let sfb = h1
                .values()
                .zip(h2.values())
                .map(|(x1, x2)| match (x1, x2) {
                    (Some(x1), Some(x2)) if x1 != x2 => 1.0,
                    _ => 0.0,
                })
                .sum();

            let shb = Hand::ALL
                .into_iter()
                .map(|hand| {
                    let [count1, count2] = [h1, h2].map(|h| {
                        Finger::ALL
                            .into_iter()
                            .map(|finger| h[HandFinger::new(hand, finger)].is_some() as u8)
                            .sum::<u8>()
                    });
                    let total = count1.min(count2);

                    let invalid = Finger::ALL
                        .into_iter()
                        .map(|finger| {
                            let finger = HandFinger::new(hand, finger);
                            let [a1, a2] = [h1, h2].map(|h| h[finger]);
                            (a1.is_some() && a1 == a2) as u8
                        })
                        .sum::<u8>();

                    (total - invalid) as f32
                })
                .sum();

            let mut movement = 0.0;
            let mut lateral = 0.0;
            let mut vertical = 0.0;
            for pair in h1.values().zip(h2.values()) {
                match pair {
                    (&Some(x), &Some(y)) => {
                        let delta = (x - y).abs();
                        movement += delta.length();
                        lateral += delta.x;
                        vertical += delta.y;
                    }
                    _ => {}
                }
            }

            let [s1, s2] =
                [(c1.layer, c2.layer), (c1.shift, c2.shift)].map(|(x, y)| u8::from(x != y));
            let staccato = (s1 + s2) as f32;

            BigramEval {
                sfb,
                shb,
                movement,
                lateral,
                vertical,
                staccato,
            }
        },
        BigramEval::min,
        bigram,
    )
}

pub fn one_letter_any<T>(
    info: &KeyboardLayout,
    init: T,
    mut op: impl FnMut(&KeyboardLayout, [&KeyCombo; 1]) -> T,
    mut reduce: impl FnMut(T, T) -> T,
    letter: [u8; 1],
) -> T {
    let [combo1] = letter.map(|k| info.key(k));
    let mut result = init;
    for c1 in combo1 {
        let this = op(info, [c1]);
        result = reduce(result, this);
    }
    result
}

pub fn one_bigram_any<T>(
    info: &KeyboardLayout,
    init: T,
    mut op: impl FnMut(&KeyboardLayout, [&KeyCombo; 2]) -> T,
    mut reduce: impl FnMut(T, T) -> T,
    bigram: [u8; 2],
) -> T {
    let [combo1, combo2] = bigram.map(|k| info.key(k));
    let mut result = init;
    for c1 in combo1 {
        for c2 in combo2 {
            let this = op(info, [c1, c2]);
            result = reduce(result, this);
        }
    }
    result
}

pub fn one_trigram_any<T>(
    info: &KeyboardLayout,
    init: T,
    mut op: impl FnMut(&KeyboardLayout, [&KeyCombo; 3]) -> T,
    mut reduce: impl FnMut(T, T) -> T,
    trigram: [u8; 3],
) -> T {
    let [combo1, combo2, combo3] = trigram.map(|k| info.key(k));
    let mut result = init;
    for c1 in combo1 {
        for c2 in combo2 {
            for c3 in combo3 {
                let this = op(info, [c1, c2, c3]);
                result = reduce(result, this);
            }
        }
    }
    result
}

pub fn one_trigram(info: &KeyboardLayout, trigram: [u8; 3]) -> TrigramEval {
    one_trigram_any(
        info,
        TrigramEval::NAN,
        |_, _| {
            // fuck what do i do about multifinger shit
            TrigramEval::splat(0.0)
        },
        TrigramEval::min,
        trigram,
    )
}

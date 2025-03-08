use std::{collections::HashSet, num::NonZeroU8};

use array_map::ArrayMap;
use derive_more::{Add, AddAssign, Sum};
use glam::{Vec2, Vec3};
use macro_rules_attribute::macro_rules_derive;
use rustc_hash::FxHashMap;

use crate::{
    counter::{Bigrams, CountOutcome, Letters, Trigrams},
    iter::{OneIter, Range},
    keyboard::{Finger, Hand, HandFinger, Keyboard},
    layout::{Behavior, KeyLoc, Layout},
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

macro_rules! splat_eval {
    ($name:ident : f32) => {
        $name
    };
    ($name:ident : Vec3) => {
        Vec3::splat($name)
    };
    ($name:ident : Vec2) => {
        Vec2::splat($name)
    };
}

macro_rules! multi_eval {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident {
            $($v_:vis $field:ident : $ty:tt),*
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

        impl std::ops::Div<f32> for $name {
            type Output = Self;

            fn div(self, _rhs: f32) -> Self {
                Self { $($field : self.$field / _rhs),* }
            }
        }

        #[allow(unused)]
        impl $name {
            pub const NAN: Self = Self::splat(f32::NAN);
            pub const ZERO: Self = Self::splat(0.0);

            pub const fn splat(_x: f32) -> Self {
                Self { $($field: splat_eval!(_x: $ty)),* }
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
    pub base: Vec3,
    pub stretch: Vec2,
}

#[macro_rules_derive(multi_eval!)]
#[derive(Debug, Clone, Copy, Add, AddAssign, Sum)]
pub struct BigramEval {
    pub sfb: f32,
    pub movement: Vec2,
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
    pub fn generate(layout: &'a Layout, keyboard: &'a Keyboard) -> Result<Self, u8> {
        Self::generate_with_usage(layout, keyboard, None, None)
    }

    pub fn generate_with_usage(
        layout: &'a Layout,
        keyboard: &'a Keyboard,
        mut used_keys: Option<&mut HashSet<KeyLoc>>,
        mut used_holds: Option<&mut HashSet<usize>>,
    ) -> Result<Self, u8> {
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

                        if let Some(ref mut used_keys) = used_keys {
                            used_keys.insert(final_key);
                        }

                        if let Some(ref mut used_holds) = used_holds {
                            used_holds.extend(shift_key.map(|x| x.index()));
                            used_holds.extend(layer_key.map(|x| x.index()))
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
        trigram: TrigramEval::ZERO,
    }
}

pub fn eval_letters(info: &KeyboardLayout, letters: &Letters) -> LetterEval {
    letters
        .iter()
        .map(|(&letter, &freq)| avg_apply(one_letter(info, letter)) * freq as f32)
        .sum()
}

pub fn eval_bigrams(info: &KeyboardLayout, bigrams: &Bigrams) -> BigramEval {
    bigrams
        .iter()
        .map(|(&bigram, &freq)| avg_apply(one_bigram(info, bigram)) * freq as f32)
        .sum()
}

pub fn eval_trigrams(info: &KeyboardLayout, trigrams: &Trigrams) -> TrigramEval {
    trigrams
        .iter()
        .map(|(&trigram, &freq)| one_trigram(info, trigram) * freq as f32)
        .sum()
}

fn avg_reduce<T: std::ops::Add<Output = T>>(x: (T, f32), y: (T, f32)) -> (T, f32) {
    (x.0 + y.0, x.1 + y.1)
}

fn avg_apply<T: std::ops::Div<f32>>(x: (T, f32)) -> T::Output {
    x.0 / x.1
}

fn convert_fingers(
    info: &KeyboardLayout,
    combo: &KeyCombo,
) -> ArrayMap<HandFinger, Option<(Vec2, bool)>, 10> {
    let mut position = ArrayMap::new([None; 10]);
    for (index, hold) in std::iter::once((combo.key, false))
        .chain(combo.shift.map(|x| (x, true)))
        .chain(combo.layer.map(|x| (x, true)))
    {
        let key = info.kb.key(index);
        position[key.finger()] = Some((key.pos(), hold));
    }
    position
}

pub fn finger_strength(finger: Finger) -> f32 {
    match finger {
        Finger::Thumb => 0.8,
        Finger::Index => 1.0,
        Finger::Middle => 1.0,
        Finger::Ring => 0.4,
        Finger::Pinky => 0.3,
    }
}

pub fn gap_costs(a: Finger, b: Finger) -> f32 {
    match (a, b) {
        (Finger::Thumb, _) => 0.0,
        (Finger::Index, Finger::Middle) => 0.6,
        (Finger::Index, Finger::Ring) => 0.5,
        (Finger::Index, Finger::Pinky) => 0.4,
        (Finger::Middle, Finger::Ring) => 1.0,
        (Finger::Middle, Finger::Pinky) => 0.9,
        (Finger::Ring, Finger::Pinky) => 1.5,

        (_, Finger::Thumb)
        | (Finger::Index, Finger::Index)
        | (Finger::Middle, Finger::Index)
        | (Finger::Ring, Finger::Index)
        | (Finger::Pinky, Finger::Index)
        | (Finger::Middle, Finger::Middle)
        | (Finger::Ring, Finger::Middle)
        | (Finger::Pinky, Finger::Middle)
        | (Finger::Ring, Finger::Ring)
        | (Finger::Pinky, Finger::Ring)
        | (Finger::Pinky, Finger::Pinky) => unreachable!(),
    }
}

pub fn hold_multiplier(hold: bool) -> f32 {
    match hold {
        true => 1.8,
        false => 1.0,
    }
}

pub fn finger_axis(finger: HandFinger) -> Vec2 {
    let flip = finger.hand != Hand::Right;
    let vec = match finger.finger {
        Finger::Thumb => Vec2::X,
        Finger::Index => Vec2::new(-1.0, 4.0).normalize(),
        Finger::Middle | Finger::Ring | Finger::Pinky => Vec2::Y,
    };
    match flip {
        true => Vec2::new(vec.x, vec.y),
        false => Vec2::new(-vec.x, vec.y),
    }
}

fn alignment_to_multiplier(hand: HandFinger, delta: Vec2) -> f32 {
    let axis = finger_axis(hand);
    let align = delta.normalize_or_zero().dot(axis);

    1.0 + (-align) * 0.5
}

pub fn one_letter(info: &KeyboardLayout, letter: [u8; 1]) -> (LetterEval, f32) {
    one_letter_any(
        info,
        (LetterEval::ZERO, 0.0),
        |info, [c]| {
            let h = convert_fingers(info, c);
            let bases = info.base;
            let mut base = Vec3::ZERO;

            for ((h, a), b) in h.iter().zip(bases.values()) {
                let hold = a.map(|x| x.1).unwrap_or(false);
                let a = a.map(|x| x.0);
                if let Some(a) = a {
                    let strength = finger_strength(h.finger);
                    let delta = (a - b).abs();
                    let align = alignment_to_multiplier(h, delta);

                    let delta = Vec3::new(delta.x, delta.y, Z)
                        * strength.recip()
                        * hold_multiplier(hold)
                        * align;
                    const Z: f32 = 2.0;
                    base += delta;
                }
            }

            let mut stretch = Vec2::ZERO;
            for hand in Hand::ALL {
                let mut last_finger = None;
                for finger in Finger::ALL {
                    let hf = HandFinger::new(hand, finger);
                    let Some((pos, hold)) = h[hf] else {
                        continue;
                    };

                    if let Some((last_finger, last_pos, last_hold)) = last_finger {
                        let cost = gap_costs(last_finger, finger);
                        let base_pos = bases[hf];
                        let last_base = bases[HandFinger::new(hand, last_finger)];

                        let base_dist = base_pos - last_base;
                        let now_dist: Vec2 = pos - last_pos;

                        let ratio = (now_dist / base_dist).abs()
                            * cost
                            * hold_multiplier(hold)
                            * hold_multiplier(last_hold);

                        stretch += ratio;
                    } else {
                        last_finger = Some((finger, pos, hold));
                    }
                }
            }

            (LetterEval { base, stretch }, 1.0)
        },
        avg_reduce,
        letter,
    )
}

pub fn one_bigram(info: &KeyboardLayout, bigram: [u8; 2]) -> (BigramEval, f32) {
    one_bigram_any(
        info,
        (BigramEval::ZERO, 0.0),
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

            let mut movement = Vec2::ZERO;
            for pair in h1.iter().zip(h2.values()) {
                match pair {
                    ((hand, &Some((x, _))), &Some((y, hy))) => {
                        let delta =
                            (x - y).abs() / finger_strength(hand.finger) * hold_multiplier(hy);
                        movement += delta;
                    }
                    _ => {}
                }
            }

            let [s1, s2] =
                [(c1.layer, c2.layer), (c1.shift, c2.shift)].map(|(x, y)| u8::from(x != y));
            let staccato = (s1 + s2) as f32;

            (
                BigramEval {
                    sfb,
                    movement,
                    staccato,
                },
                1.0,
            )
        },
        avg_reduce,
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

/// sum squared evaluation
pub fn sse<const N: usize>(combos: [(f32, f32); N]) -> f32 {
    combos.into_iter().map(|(w, x)| w * x.powi(2)).sum()
}

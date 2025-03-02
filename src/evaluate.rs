use std::ops::Mul;

use array_map::ArrayMap;
use arrayvec::ArrayVec;
use derive_more::{Add, AddAssign, Sum};
use glam::{Vec2, Vec3};
use rustc_hash::FxHashMap;

use crate::{
    counter::{Bigrams, CountOutcome},
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

#[derive(Debug)]
pub struct Evaluation {
    bigram: BigramEval,

    // trigrams
    redirects: f32,
    rolls: f32,
    alternates: f32,
}

#[derive(Debug, Add, AddAssign, Sum)]
pub struct BigramEval {
    sfb: f32,
    shb: f32,
    movement: f32,
    lateral: f32,
    vertical: f32,
    staccato: f32,
}

impl Mul<f32> for BigramEval {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            sfb: self.sfb * rhs,
            shb: self.shb * rhs,
            movement: self.movement * rhs,
            lateral: self.lateral * rhs,
            vertical: self.vertical * rhs,
            staccato: self.staccato * rhs,
        }
    }
}

impl BigramEval {
    pub const NAN: Self = Self::splat(f32::NAN);
    pub const ZERO: Self = Self::splat(0.0);

    pub const fn splat(x: f32) -> Self {
        Self {
            sfb: x,
            shb: x,
            movement: x,
            lateral: x,
            vertical: x,
            staccato: x,
        }
    }

    pub fn min(self, other: Self) -> Self {
        Self {
            sfb: self.sfb.min(other.sfb),
            shb: self.shb.min(other.shb),
            movement: self.movement.min(other.movement),
            lateral: self.lateral.min(other.lateral),
            vertical: self.vertical.min(other.vertical),
            staccato: self.staccato.min(other.staccato),
        }
    }
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
    kb: &'a Keyboard,
}

impl<'a> KeyboardLayout<'a> {
    pub fn generate(layout: &Layout, keyboard: &'a Keyboard) -> Self {
        let shift_keys = layout
            .find_all_key(|k| k.hold().is_some_and(|b| b == Behavior::Shift))
            .collect::<Vec<_>>();

        let mut keys = FxHashMap::default();
        for &key in crate::KEYS {
            let real_key = match key {
                b'A'..=b'Z' => key.to_ascii_lowercase(),
                b'?' => b'/',
                _ => key,
            };
            let do_shift = key != real_key;

            let shift_keys: OneIter<_> = do_shift.then(|| shift_keys.iter().copied()).into();

            let mut combos = vec![];
            for final_key in layout.find_all_key(|layout_key| {
                layout_key.tap().is_some_and(|k| u8::from(k) == real_key)
            }) {
                let layer = final_key.layer();
                let layer_keys: OneIter<_> = (layer != 0)
                    .then(|| {
                        layout.find_on_base(|layout_key| {
                            layout_key
                                .hold()
                                .is_some_and(|b| b == Behavior::Layer(layer))
                        })
                    })
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
                panic!("No ways to get char: {}({key})", char::from(key));
            }
            keys.insert(key, combos);
        }

        Self { keys, kb: keyboard }
    }

    pub fn key(&self, x: u8) -> &[KeyCombo] {
        &self.keys[&x]
    }
}

pub fn evaluate(info: &KeyboardLayout, count: &CountOutcome) -> Evaluation {
    Evaluation {
        bigram: eval_bigrams(info, &count.bigrams),
        redirects: 0.0,
        rolls: 0.0,
        alternates: 0.0,
    }
}

pub fn eval_bigrams(info: &KeyboardLayout, bigrams: &Bigrams) -> BigramEval {
    bigrams
        .iter()
        .map(|(&bigram, &freq)| one_bigram(info, bigram) * freq as f32)
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

pub fn one_bigram(info: &KeyboardLayout, bigram: [u8; 2]) -> BigramEval {
    one_bigram_any(
        info,
        BigramEval::NAN,
        |info, c1, c2| {
            let h1 = convert_fingers(info, c1);
            let h2 = convert_fingers(info, c2);

            // sfb
            let sfb = h1
                .values()
                .zip(h2.values())
                .map(|(x1, x2)| match (x1, x2) {
                    (Some(x1), Some(x2)) if x1 != x2 => 1.0,
                    _ => 0.0,
                })
                .sum();

            // shb
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

            BigramEval {
                sfb,
                shb,
                movement,
                lateral,
                vertical,
                staccato: 0.0,
            }
        },
        BigramEval::min,
        bigram,
    )
}

pub fn one_bigram_any<T>(
    info: &KeyboardLayout,
    init: T,
    mut op: impl FnMut(&KeyboardLayout, &KeyCombo, &KeyCombo) -> T,
    mut reduce: impl FnMut(T, T) -> T,
    bigram: [u8; 2],
) -> T {
    let [combo1, combo2] = bigram.map(|k| info.key(k));
    let mut result = init;
    for c1 in combo1 {
        for c2 in combo2 {
            let this = op(info, c1, c2);
            result = reduce(result, this);
        }
    }
    result
}

use evaluate::{Evaluation, KeyboardLayout};
use keyboard::Keyboard;
use layout::{Behavior, KeyLoc, Layout};
use rand::Rng as _;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    num::NonZeroU8,
    sync::LazyLock,
};

mod counter;
mod evaluate;
mod ferris;
mod iter;
mod keyboard;
mod layout;
mod optimization;
mod output;

pub const ALPHABET: &[u8; 97] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\\\"<>(){}[]:!;.,/?=+&*^%@#_|'`$-~";
pub fn in_alphabet(x: u8) -> bool {
    static LUT: LazyLock<[bool; 256]> =
        LazyLock::new(|| std::array::from_fn(|x| ALPHABET.iter().any(|&k| usize::from(k) == x)));
    LUT[usize::from(x)]
}

fn main() {
    let (count, err) = counter::count("..");
    if let Some(err) = err {
        println!("Cache failed: {err:?}");
    }

    let keyboard = Keyboard::ferris_sweep();
    // let start_layout = ferris::qwerty();
    let reference_layout = ferris::qwerty();
    let start_layout: Layout =
        serde_json::from_str(&std::fs::read_to_string("kb/final.json").unwrap()).unwrap();
    const THIS_PATH: &str = "kb/final2.json";

    // let layout2: Layout =
    //     serde_json::from_str(&std::fs::read_to_string(THIS_PATH).unwrap()).unwrap();
    // output::print_ferris_layout(&layout);
    // output::print_ferris_layout(&layout2);
    // let l1 = KeyboardLayout::generate(&layout, &keyboard)
    //     .map_err(char::from)
    //     .unwrap();
    // let l2 = KeyboardLayout::generate(&layout2, &keyboard).unwrap();
    // let eval = evaluate::evaluate(&l1, &count);
    // println!("qwerty: {eval:#?}");
    // let eval = evaluate::evaluate(&l2, &count);
    // println!("??????: {eval:#?}");

    // for &k in b1.keys() {
    //     if b1[&k] != b2[&k] {
    //         println!("{} {} {}", char::from(k), b1[&k], b2[&k]);
    //     }
    // }

    fn to_evaluation(scaled: &Evaluation) -> f32 {
        evaluate::sse([
            (2.0, scaled.letter.base.x),
            (1.0, scaled.letter.base.y),
            (5.0, scaled.letter.base.z),
            (2.0, scaled.bigram.movement),
            (15.0, scaled.bigram.staccato),
        ])
    }

    let kl = KeyboardLayout::generate(&reference_layout, &keyboard).unwrap();
    let reference_eval = evaluate::evaluate(&kl, &count);
    let scale_evaluation = |eval: Evaluation| eval / reference_eval.clone() * 100.0;

    let start_kl = KeyboardLayout::generate(&start_layout, &keyboard).unwrap();
    let start_eval = scale_evaluation(evaluate::evaluate(&start_kl, &count));
    let start_evaluation = to_evaluation(&start_eval);
    let eval_scaler = 1_000_000.0 / start_evaluation;

    let result = optimization::anneal(
        start_layout,
        200000,
        |x| 5.0 * (1.0 - x),
        |_, layout| {
            let any_other_alphabetic = layout.layers().iter().skip(1).any(|layer| {
                layer
                    .keys()
                    .iter()
                    .any(|key| key.is_some_and(|k| matches!(k.get(), b'a'..=b'z')))
            });
            let layers_with_numbers = layout
                .layers()
                .iter()
                .map(|layer| {
                    layer
                        .keys()
                        .iter()
                        .any(|k| k.is_some_and(|k| matches!(k.get(), b'0'..=b'9')))
                        as u8
                })
                .sum::<u8>();
            if any_other_alphabetic || layers_with_numbers > 1 {
                return None;
            }

            let mut keys = HashSet::new();
            let mut holds = HashSet::new();
            let info = KeyboardLayout::generate_with_usage(
                layout,
                &keyboard,
                Some(&mut keys),
                Some(&mut holds),
            )
            .ok()?;

            let scaled = scale_evaluation(evaluate::evaluate(&info, &count));
            Some((to_evaluation(&scaled) * eval_scaler, (keys, holds)))
        },
        |rng, layout, (keys, holds)| {
            let size = layout.layer_size();

            for i in 0..size {
                let at = &mut layout.base_hold_mut()[i];
                if !holds.contains(&i) && rng.random_bool(0.5) {
                    *at = None;
                }
            }

            for (li, layer) in layout.layers_mut().iter_mut().enumerate() {
                for i in 0..size {
                    let loc = KeyLoc::new(li as u8, i);
                    if !keys.contains(&loc) && rng.random_bool(0.7) {
                        *layer.key_mut(i) = None;
                    }
                }
            }
        },
    );
    let json = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(THIS_PATH, json).unwrap();
    output::print_ferris_layout(&result);
}

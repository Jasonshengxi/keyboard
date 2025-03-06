use evaluate::KeyboardLayout;
use keyboard::Keyboard;
use layout::{Behavior, Layout, LayoutLayer};
use std::{num::NonZeroU8, sync::LazyLock};

mod counter;
mod evaluate;
mod iter;
mod keyboard;
mod layout;
mod optimization;
mod output;

pub const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\\\"<>(){}[]:!;.,/?=+&*^%@#_|'`$-~";
pub fn in_alphabet(x: u8) -> bool {
    static LUT: LazyLock<[bool; 256]> =
        LazyLock::new(|| std::array::from_fn(|x| ALPHABET.iter().any(|&k| usize::from(k) == x)));
    LUT[usize::from(x)]
}

const fn u(x: u8) -> NonZeroU8 {
    match NonZeroU8::new(x) {
        Some(x) => x,
        None => unreachable!(),
    }
}

pub fn make_hold(mods: &[u8; 34]) -> Vec<Option<Behavior>> {
    let mut base_hold = Vec::with_capacity(34);
    for &m in mods {
        let hold = match m {
            b'S' => Some(Behavior::Shift),
            c if c.is_ascii_digit() => Some(Behavior::Layer(u(c - b'0'))),
            _ => None,
        };
        base_hold.push(hold);
    }

    base_hold
}

fn layer_simple(lay: &[u8]) -> LayoutLayer {
    let mut layer1 = Vec::with_capacity(34);
    for &key in lay {
        let tap = match key {
            b' ' => None,
            ch => Some(NonZeroU8::new(ch)),
        };
        layer1.push(tap.flatten());
    }
    for _ in layer1.len()..34 {
        layer1.push(None);
    }
    LayoutLayer::new(layer1)
}

pub fn ferris_any(base: &[u8; 30]) -> Layout {
    const MODS: &[u8; 34] = b" S        1        2        S 3443";
    const LAY1: &[u8; 30] = b" ^  *  &        # _~-|/\\'\"` $ ";
    const LAY2: &[u8; 30] = b" { :}!<([>)];@        =, +. % ";
    const LAY3: &[u8; 30] = b"    % :/  \n!                  ";
    const LAY4: &[u8; 30] = b"1  2  3  4  5  6  7  8  9  0  ";
    let base_hold = make_hold(MODS);
    let mut layer0 = layer_simple(base);
    layer0.set_key(31, Some(u(b' ')));
    layer0.set_key(32, Some(u(b' ')));
    let layer1 = layer_simple(LAY1);
    let layer2 = layer_simple(LAY2);
    let mut layer3 = layer_simple(LAY3);
    layer3.set_key(31, Some(u(b'\t')));
    let layer4 = layer_simple(LAY4);

    Layout::new(base_hold, vec![layer0, layer1, layer2, layer3, layer4])
}

pub fn ferris_qwerty() -> Layout {
    const KEYS: &[u8; 30] = b"qazwsxedcrfvtgbyhnujmik,ol.p;/";
    ferris_any(KEYS)
}

pub fn ferris_colemak_dh() -> Layout {
    const KEYS: &[u8; 30] = b"qazwrxfscptdbgvjmklnhue,yi.;o/";
    ferris_any(KEYS)
}

pub fn ferris_canary() -> Layout {
    const KEYS: &[u8; 30] = b"wcqlrjysvptdbgkzmxfnhoe/ui,;a.";
    ferris_any(KEYS)
}

fn main() {
    let (count, err) = counter::count("..");
    if let Some(err) = err {
        println!("Cache failed: {err:?}");
    }

    let keyboard = Keyboard::ferris_sweep();
    let layout = ferris_qwerty();
    // let layout: Layout =
    //     serde_json::from_str(&std::fs::read_to_string("kb/base_dist.json").unwrap()).unwrap();
    const THIS_PATH: &str = "kb/final.json";
    // let layout2: Layout =
    //     serde_json::from_str(&std::fs::read_to_string(THIS_PATH).unwrap()).unwrap();
    // output::print_ferris_layout(&layout2);
    // let l1 = KeyboardLayout::generate(&layout, &keyboard)
    //     .map_err(char::from)
    //     .unwrap();
    // let l2 = KeyboardLayout::generate(&layout2, &keyboard).unwrap();
    // let eval = evaluate::evaluate(&l1, &count);
    // println!("qwerty: {eval:#?}");
    // let eval = evaluate::evaluate(&l2, &count);
    // println!("???: {eval:#?}");

    let kl = KeyboardLayout::generate(&layout, &keyboard).unwrap();
    let start_eval = evaluate::evaluate(&kl, &count);
    let result = optimization::anneal(layout, |layout| {
        let letters_on_base = layout.layer(0).keys().iter().fold(0, |acc, ch| {
            let c = ch.map_or(0, u8::from);
            acc + u32::from(matches!(c, b'a'..=b'z'))
        });
        if letters_on_base != 26 {
            return None;
        }
        let info = KeyboardLayout::generate(layout, &keyboard).ok()?;
        let eval = evaluate::evaluate(&info, &count);
        let scaled = eval / start_eval.clone() * 100.0;
        // Some(eval.bigram.movement + 8.0 * eval.bigram.staccato)
        // Some(eval.letter.base_dist)
        // Some(scaled.bigram.movement)
        Some(
            scaled.letter.base_dist.powi(2)
                + scaled.bigram.movement.powi(2)
                + scaled.bigram.staccato.powi(2),
        )
    });
    let json = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(THIS_PATH, json).unwrap();
    output::print_ferris_layout(&result);
}

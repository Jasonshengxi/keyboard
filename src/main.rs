use evaluate::KeyboardLayout;
use keyboard::Keyboard;
use layout::{Behavior, Layout, LayoutKey, LayoutLayer};
use std::{
    collections::{hash_map, HashMap},
    fmt::Write as _,
    num::NonZeroU8,
    sync::LazyLock,
};

mod counter;
mod evaluate;
mod iter;
mod keyboard;
mod layout;

pub const KEYS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\\\"<>(){}[]:!;.,/?=+&*^%@#_|'`$-~";
pub fn in_alphabet(x: u8) -> bool {
    static LUT: LazyLock<[bool; 256]> =
        LazyLock::new(|| std::array::from_fn(|x| KEYS.iter().any(|&k| usize::from(k) == x)));
    LUT[usize::from(x)]
}

fn render_frequency_table<I, F, const NGRAM: usize>(
    data: HashMap<[u8; NGRAM], u32>,
    top_n: usize,
    func: F,
) where
    I: IntoIterator<Item = ([u8; NGRAM], u32)>,
    F: FnOnce(hash_map::IntoIter<[u8; NGRAM], u32>) -> I,
{
    let mut pairs = func(data.into_iter()).into_iter().collect::<Vec<_>>();
    let len = pairs.len();
    pairs.sort_unstable_by_key(|i| i.1);
    let max_len = pairs
        .iter()
        .skip(len.saturating_sub(top_n))
        .map(|(chars, _)| {
            let mut total_len = 0;
            for c in chars.map(char::from) {
                if c.is_ascii_graphic() || c == ' ' {
                    total_len += 1;
                } else {
                    let debug_len = format!("{c:?}").len();
                    total_len += debug_len;
                }
            }
            total_len
        })
        .max()
        .unwrap();

    println!("top {top_n}");
    for (chars, occur) in pairs.into_iter().skip(len.saturating_sub(top_n)) {
        let mut printed = String::new();
        for c in chars.map(char::from) {
            if c.is_ascii_graphic() || c == ' ' {
                write!(printed, "{c}").unwrap();
            } else {
                write!(printed, "{c:?}").unwrap();
            }
        }

        let count = max_len - printed.len();
        print!("{printed}");
        for _ in 0..count {
            print!(" ");
        }

        println!(" | {occur}");
    }
}

pub fn ferris_any(base: &[u8; 30]) -> Layout {
    const MODS: &[u8; 30] = b" S        1        2        S ";
    const LAY1: &[u8; 30] = b" ^  *  &        # _~-|/\\'\"` $ ";
    const LAY2: &[u8; 30] = b" { :}!<([>)];@        =, +. % ";
    const LAY3: &[u8; 30] = b"    % :/  \n!                  ";
    const LAY4: &[u8; 30] = b"1  2  3  4  5  6  7  8  9  0  ";

    let mut base_keys = Vec::with_capacity(30);
    for (&key, &m) in base.iter().zip(MODS) {
        let hold = match m {
            b'S' => Some(Behavior::Shift),
            b'1' => Some(Behavior::Layer(1)),
            b'2' => Some(Behavior::Layer(2)),
            _ => None,
        };

        base_keys.push(Some(LayoutKey::new(key, hold)));
    }
    base_keys.push(Some(LayoutKey::new_raw(None, Some(Behavior::Layer(4)))));
    base_keys.push(Some(LayoutKey::new(b' ', Some(Behavior::Layer(3)))));
    base_keys.push(Some(LayoutKey::new(b' ', Some(Behavior::Layer(3)))));
    base_keys.push(Some(LayoutKey::new_raw(None, Some(Behavior::Layer(4)))));
    let base_layer = LayoutLayer::new(base_keys);

    fn layer_simple(lay: &[u8]) -> LayoutLayer {
        let mut layer1 = Vec::with_capacity(30);
        for &key in lay {
            let tap = match key {
                b' ' => None,
                ch => Some(NonZeroU8::new(ch)),
            };
            layer1.push(tap.map(|tap| LayoutKey::new_raw(tap, None)));
        }
        layer1.push(None);
        layer1.push(None);
        layer1.push(None);
        layer1.push(None);
        LayoutLayer::new(layer1)
    }
    let layer1 = layer_simple(LAY1);
    let layer2 = layer_simple(LAY2);
    let mut layer3 = layer_simple(LAY3);
    layer3.set_key(31, Some(LayoutKey::new(b'\t', None)));
    let layer4 = layer_simple(LAY4);

    Layout::new(vec![base_layer, layer1, layer2, layer3, layer4])
}

pub fn ferris_qwerty() -> Layout {
    const KEYS: &[u8; 30] = b"qazwsxedcrfvtgbyhnujmik,ol.p;/";
    ferris_any(KEYS)
}

pub fn ferris_colemak_dh() -> Layout {
    const KEYS: &[u8; 30] = b"qazwrxfscptdbgvjmklnhue,yi.;o/";
    ferris_any(KEYS)
}

fn main() {
    let (count, err) = counter::count("..");
    if let Some(err) = err {
        println!("Cache failed: {err:?}");
    }

    let keyboard = Keyboard::ferris_sweep();
    let [l1, l2] = [ferris_qwerty(), ferris_colemak_dh()]
        .map(|layout| KeyboardLayout::generate(&layout, &keyboard));

    // println!("QWERTY: ");
    // render_frequency_table(count.bigrams.clone(), 50, |i| {
    //     i.map(|(bigram, freq)| (bigram, freq * evaluate::one_sfb(&l1, bigram)))
    // });
    // println!("total: {}", evaluate::eval_sfb(&l1, &count.bigrams));
    // println!();
    //
    // println!("Colemak DH: ");
    // render_frequency_table(count.bigrams.clone(), 50, |i| {
    //     i.map(|(bigram, freq)| (bigram, freq * evaluate::one_sfb(&l2, bigram)))
    // });
    // println!("total: {}", evaluate::eval_sfb(&l2, &count.bigrams));

    let eval = evaluate::evaluate(&l1, &count);
    println!("qwerty: {eval:#?}");
    let eval = evaluate::evaluate(&l2, &count);
    println!("colemak dh: {eval:#?}");
}

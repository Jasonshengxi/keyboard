use colored::Colorize;
use evaluate::KeyboardLayout;
use keyboard::Keyboard;
use layout::{Behavior, Layout, LayoutLayer};
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
mod optimization;

pub const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\\\"<>(){}[]:!;.,/?=+&*^%@#_|'`$-~";
pub fn in_alphabet(x: u8) -> bool {
    static LUT: LazyLock<[bool; 256]> =
        LazyLock::new(|| std::array::from_fn(|x| ALPHABET.iter().any(|&k| usize::from(k) == x)));
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

const fn u(x: u8) -> NonZeroU8 {
    match NonZeroU8::new(x) {
        Some(x) => x,
        None => unreachable!(),
    }
}

pub fn ferris_any(base: &[u8; 30]) -> Layout {
    const MODS: &[u8; 30] = b" S        1        2        S ";
    const LAY1: &[u8; 30] = b" ^  *  &        # _~-|/\\'\"` $ ";
    const LAY2: &[u8; 30] = b" { :}!<([>)];@        =, +. % ";
    const LAY3: &[u8; 30] = b"    % :/  \n!                  ";
    const LAY4: &[u8; 30] = b"1  2  3  4  5  6  7  8  9  0  ";

    let mut base_hold = Vec::with_capacity(34);
    for &m in MODS {
        let hold = match m {
            b'S' => Some(Behavior::Shift),
            b'1' => Some(Behavior::Layer(u(1))),
            b'2' => Some(Behavior::Layer(u(2))),
            _ => None,
        };
        base_hold.push(hold);
    }

    base_hold.push(Some(Behavior::Layer(u(4))));
    base_hold.push(Some(Behavior::Layer(u(3))));
    base_hold.push(Some(Behavior::Layer(u(3))));
    base_hold.push(Some(Behavior::Layer(u(4))));

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

pub fn print_ferris_layout(layout: &Layout) {
    fn row1(key: Option<NonZeroU8>) {
        let key = key.map_or(0, u8::from);

        match key {
            b'\n' => print!("{}│", "RET".green().bold()),
            b'\t' => print!("{}│", "TAB".green().bold()),
            b' ' => print!("{}│", "SPC".green().bold()),
            0 => print!("   │"),
            _ => print!(" {} │", char::from(key).to_string().green().bold()),
        }
    }

    fn row2(key: Option<Behavior>) {
        match key {
            Some(Behavior::Shift) => print!(" {} │", "S".blue()),
            Some(Behavior::Layer(layer)) => print!(" {} │", layer.to_string().blue()),
            None => print!("   │"),
        }
    }

    for (li, layer) in layout.layers().iter().enumerate() {
        let keys = layer.keys();

        print!("┌");
        for _ in 1..10 {
            print!("───┬");
        }
        println!("───┐");
        for row in 0..3 {
            print!("│");
            for column in 0..10 {
                let index = column * 3 + row;
                let key = keys[index];
                row1(key);
            }
            println!();

            print!("│");
            for column in 0..10 {
                let index = column * 3 + row;
                row2(
                    (li == 0)
                        .then_some(())
                        .and_then(|_| layout.base_hold()[index]),
                );
            }
            println!();

            if row == 2 {
                print!("└");
            } else {
                print!("├");
            }
            for x in 0..9 {
                print!("───");
                if (x <= 1 || x >= 7) && row == 2 {
                    print!("┴")
                } else {
                    print!("┼");
                }
            }
            print!("───");
            if row == 2 {
                println!("┘");
            } else {
                println!("┤");
            }
        }

        fn tab() {
            for _ in 0..12 {
                print!(" ")
            }
        }

        tab();
        print!("│");
        for i in 30..34 {
            row1(keys[i]);
        }
        println!();
        tab();
        print!("│");
        for i in 30..34 {
            row2((li == 0).then_some(()).and_then(|_| layout.base_hold()[i]));
        }
        println!();
        tab();
        print!("└");
        for _ in 0..3 {
            print!("───");
            print!("┴")
        }
        print!("───");
        println!("┘");

        println!()
    }
}

fn main() {
    let (count, err) = counter::count("..");
    if let Some(err) = err {
        println!("Cache failed: {err:?}");
    }

    let keyboard = Keyboard::ferris_sweep();
    // let layout = ferris_qwerty();
    let layout: Layout =
        serde_json::from_str(&std::fs::read_to_string("kb/base_dist.json").unwrap()).unwrap();
    const THIS_PATH: &str = "kb/base_dist2.json";
    // let layout2: Layout =
    //     serde_json::from_str(&std::fs::read_to_string(THIS_PATH).unwrap()).unwrap();

    // print_ferris_layout(&layout);
    // let l1 = KeyboardLayout::generate(&layout, &keyboard)
    //     .map_err(char::from)
    //     .unwrap();
    // let l2 = KeyboardLayout::generate(&layout2, &keyboard).unwrap();
    // let eval = evaluate::evaluate(&l1, &count);
    // println!("qwerty: {eval:#?}");
    // let eval = evaluate::evaluate(&l2, &count);
    // println!("???: {eval:#?}");

    let result = optimization::anneal(layout, |layout| {
        let info = KeyboardLayout::generate(layout, &keyboard).ok()?;
        let eval = evaluate::evaluate(&info, &count);
        // Some(eval.bigram.movement + 8.0 * eval.bigram.staccato)
        Some(eval.letter.base_dist)
    });
    let json = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(THIS_PATH, json).unwrap();
    print_ferris_layout(&result);
}

use colored::Colorize as _;
use std::{
    collections::{hash_map, HashMap},
    fmt::{Display, Write as _}, num::NonZeroU8,
};

use crate::layout::{Behavior, Layout};

pub fn render_frequency_table<I, F, E, const NGRAM: usize>(
    data: HashMap<[u8; NGRAM], E>,
    top_n: usize,
    func: F,
) where
    I: IntoIterator<Item = ([u8; NGRAM], E)>,
    F: FnOnce(hash_map::IntoIter<[u8; NGRAM], E>) -> I,
    E: Ord + Display + Copy
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
            Some(Behavior::Shift) => print!(" {} │", "S".blue().bold()),
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

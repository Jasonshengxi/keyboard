use std::num::NonZeroU8;

use crate::layout::{Behavior, Layout, LayoutLayer};

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

pub fn layout_any(base: &[u8; 30]) -> Layout {
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

pub fn qwerty() -> Layout {
    const KEYS: &[u8; 30] = b"qazwsxedcrfvtgbyhnujmik,ol.p;/";
    layout_any(KEYS)
}

pub fn colemak_dh() -> Layout {
    const KEYS: &[u8; 30] = b"qazwrxfscptdbgvjmklnhue,yi.;o/";
    layout_any(KEYS)
}

pub fn canary() -> Layout {
    const KEYS: &[u8; 30] = b"wcqlrjysvptdbgkzmxfnhoe/ui,;a.";
    layout_any(KEYS)
}

fn flip_internal<T: Copy>(buffer: &[T]) -> Vec<T> {
    const NEW_SHAPE: [usize; 34] = [
        27, 28, 29, 24, 25, 26, 21, 22, 23, 18, 19, 20, 15, 16, 17, 12, 13, 14, 9, 10, 11, 6, 7, 8,
        3, 4, 5, 0, 1, 2, 33, 32, 31, 30,
    ];
    NEW_SHAPE.into_iter().map(|i| buffer[i]).collect()
}

pub fn flip_layout(layout: &Layout) -> Layout {
    Layout::new(
        flip_internal(layout.base_hold()),
        layout
            .layers()
            .iter()
            .map(|layer| LayoutLayer::new(flip_internal(layer.keys())))
            .collect(),
    )
}

use std::num::NonZeroU8;

use rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::{
    layout::{Behavior, Layout},
    ALPHABET,
};

pub fn mutate(rng: &mut impl Rng, layout: &mut Layout) {
    const HOLD_SWAP: f64 = 1.0;
    const KEY_SWAP: f64 = 1.0;
    const VERTICAL_SWAP: f64 = 0.8;
    const NEW_KEY: f64 = 0.01;
    const NEW_HOLD: f64 = 0.00;

    let layer_count = layout.layer_count();
    let size = layout.layer_size();

    if rng.random_bool(NEW_HOLD) {
        let i = rng.random_range(0..size);
        let layer = rng.random_range(0..layer_count);
        let behavior = match NonZeroU8::new(layer) {
            None => Behavior::Shift,
            Some(layer) => Behavior::Layer(layer),
        };
        layout.base_hold_mut()[i] = Some(behavior);
    }

    if rng.random_bool(NEW_KEY) {
        let layer = rng.random_range(0..layer_count);
        let i = rng.random_range(0..size);
        let ch = ALPHABET[rng.random_range(0..ALPHABET.len())];
        *layout.layer_mut(layer).key_mut(i) = NonZeroU8::new(ch);
    }

    if rng.random_bool(HOLD_SWAP) {
        let [i1, i2] = [(); 2].map(|_| rng.random_range(0..size));
        layout.base_hold_mut().swap(i1, i2);
    }

    if rng.random_bool(KEY_SWAP) {
        let target_layer = rng.random_range(0..layer_count);
        let layer = layout.layer_mut(target_layer);
        let [i1, i2] = [(); 2].map(|_| rng.random_range(0..size));
        layer.keys_mut().swap(i1, i2);
    }

    if rng.random_bool(VERTICAL_SWAP) {
        let layer1 = rng.random_range(0..layer_count);
        let mut layer2 = rng.random_range(1..layer_count);
        if layer2 <= layer1 {
            layer2 -= 1
        }
        let index = rng.random_range(0..size);

        let [layer1, .., layer2] =
            &mut layout.layers_mut()[layer1.min(layer2) as usize..=layer1.max(layer2) as usize]
        else {
            unreachable!()
        };
        std::mem::swap(layer1.key_mut(index), layer2.key_mut(index));
    }
}

pub fn anneal<E>(
    layout: Layout,
    iters: u32,
    profile: impl Fn(f32) -> f32,
    eval: impl Fn(u32, &Layout) -> Option<(f32, E)>,
    modifier: impl Fn(&mut SmallRng, &mut Layout, E),
) -> Layout {
    let mut current = layout;
    let (mut current_score, _) = eval(0, &current).unwrap();
    let mut rng = SmallRng::from_os_rng();

    for i in 0..iters {
        let temperature = profile(i as f32 / iters as f32);

        let mut new_layout = current.clone();
        let (new_layout, extra, new_score) = loop {
            mutate(&mut rng, &mut new_layout);
            let new_score = eval(i, &new_layout);
            if let Some((score, extra)) = new_score {
                break (new_layout, extra, score);
            }
            current.clone_into(&mut new_layout);
        };
        if i % 1000 == 0 {
            println!("({i},{current_score}),");
        }

        let accept_prob = if new_score < current_score {
            1.0
        } else {
            ((current_score - new_score) / temperature).exp()
        };

        if rng.random_bool(accept_prob.into()) {
            current = new_layout;
            modifier(&mut rng, &mut current, extra);
            current_score = new_score;
        }
    }

    current
}

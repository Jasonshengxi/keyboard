use std::num::NonZeroU8;

use rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::layout::{Behavior, Layout};

pub fn mutate(rng: &mut impl Rng, mut layout: Layout) -> Layout {
    const BASE_SWAP: f64 = 1.0;
    const KEY_SWAP: f64 = 1.0;
    const VERTICAL_SWAP: f64 = 0.8;

    let layer_count = layout.layer_count();
    let size = layout.layer_size();
    if rng.random_bool(BASE_SWAP) {
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
            layer2 -= 1;
        }
        assert_ne!(layer1, layer2);
        let size = layout.layer_size();
        let index = rng.random_range(0..size);
        let layers = layout.layers_mut();

        let layer1 = layers[usize::from(layer1)].key_mut(index) as *mut _;
        let layer2 = layers[usize::from(layer2)].key_mut(index) as *mut _;
        unsafe { std::ptr::swap(layer1, layer2) };
    }

    layout
}

pub fn anneal(layout: Layout, eval: impl Fn(&Layout) -> Option<f32>) -> Layout {
    let mut current = layout;
    let mut current_score = eval(&current).unwrap();
    let mut rng = SmallRng::from_os_rng();

    const ITERS: i32 = 10000;
    for i in 0..ITERS {
        let temperature = 500.0 * (1.0 - (i as f32 / ITERS as f32));

        let (new_layout, new_score) = loop {
            let new_layout = mutate(&mut rng, current.clone());
            let new_score = eval(&new_layout);
            if let Some(score) = new_score {
                break (new_layout, score);
            }
        };
        if i % 100 == 0 {
            println!("({i},{current_score}),");
        }

        let accept_prob = if new_score < current_score {
            1.0
        } else {
            ((current_score - new_score) / temperature).exp()
        };

        if rng.random_bool(accept_prob.into()) {
            current = new_layout;
            current_score = new_score;
        }
    }

    current
}

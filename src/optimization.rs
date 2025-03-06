use rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::layout::Layout;

pub fn mutate(rng: &mut impl Rng, layout: &mut Layout) {
    const HOLD_SWAP: f64 = 1.0;
    const KEY_SWAP: f64 = 1.0;
    const VERTICAL_SWAP: f64 = 0.8;

    let layer_count = layout.layer_count();
    let size = layout.layer_size();

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

pub fn anneal(layout: Layout, eval: impl Fn(&Layout) -> Option<f32>) -> Layout {
    let mut current = layout;
    let mut current_score = eval(&current).unwrap();
    let mut rng = SmallRng::from_os_rng();

    const ITERS: i32 = 10000;
    for i in 0..ITERS {
        let temperature = 0.02 * (1.0 - (i as f32 / ITERS as f32));

        let mut new_layout = current.clone();
        let (new_layout, new_score) = loop {
            mutate(&mut rng, &mut new_layout);
            let new_score = eval(&new_layout);
            if let Some(score) = new_score {
                break (new_layout, score);
            }
            current.clone_into(&mut new_layout);
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

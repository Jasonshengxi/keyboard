use std::num::NonZeroU8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    Shift,
    Layer(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutKey {
    tap: Option<NonZeroU8>,
    hold: Option<Behavior>,
}

impl LayoutKey {
    pub fn new_raw(tap: Option<NonZeroU8>, hold: Option<Behavior>) -> Self {
        Self { tap, hold }
    }

    pub fn new(tap: u8, hold: Option<Behavior>) -> Self {
        Self {
            tap: NonZeroU8::new(tap),
            hold,
        }
    }

    pub fn tap(&self) -> Option<NonZeroU8> {
        self.tap
    }

    pub fn hold(&self) -> Option<Behavior> {
        self.hold
    }
}

pub struct LayoutLayer {
    keys: Vec<Option<LayoutKey>>,
}

impl LayoutLayer {
    pub fn new(keys: Vec<Option<LayoutKey>>) -> Self {
        Self { keys }
    }

    pub fn set_key(&mut self, index: usize, key: Option<LayoutKey>) {
        self.keys[index] = key;
    }

    pub fn keys(&self) -> &[Option<LayoutKey>] {
        &self.keys
    }
}

pub struct Layout {
    layers: Vec<LayoutLayer>,
}

impl Layout {
    pub fn new(layers: Vec<LayoutLayer>) -> Self {
        Self { layers }
    }

    pub fn layers(&self) -> &[LayoutLayer] {
        &self.layers
    }

    pub fn key(&self, layer: u8, index: usize) -> Option<LayoutKey> {
        self.layers[layer as usize].keys[index]
    }

    pub fn first_layer(&self) -> &LayoutLayer {
        self.layers().first().unwrap()
    }

    pub fn find_on_base<F: FnMut(LayoutKey) -> bool + Copy>(
        &self,
        mut func: F,
    ) -> impl Iterator<Item = KeyLoc> + use<'_, F> {
        self.first_layer()
            .keys()
            .iter()
            .enumerate()
            .filter_map(move |(i, l_key)| {
                l_key
                    .as_ref()
                    .copied()
                    .is_some_and(&mut func)
                    .then_some(KeyLoc::new(0, i))
            })
    }

    pub fn find_all_key<F: FnMut(LayoutKey) -> bool + Copy>(
        &self,
        mut func: F,
    ) -> impl Iterator<Item = KeyLoc> + use<'_, F> {
        self.layers()
            .iter()
            .enumerate()
            .flat_map(move |(li, layer)| {
                layer
                    .keys()
                    .iter()
                    .enumerate()
                    .filter_map(move |(i, l_key)| {
                        l_key
                            .as_ref()
                            .copied()
                            .is_some_and(&mut func)
                            .then_some(KeyLoc::new(li as u8, i))
                    })
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeyLoc {
    layer: u8,
    index: usize,
}

impl KeyLoc {
    pub fn new(layer: u8, index: usize) -> Self {
        Self { layer, index }
    }

    pub fn layer(&self) -> u8 {
        self.layer
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

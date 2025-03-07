use serde::{Deserialize, Serialize};
use std::{char::TryFromCharError, num::NonZeroU8};

#[derive(Serialize, Deserialize)]
struct SerdeBehaviors(String);

impl From<SerdeBehaviors> for BaseBehavior {
    fn from(value: SerdeBehaviors) -> Self {
        Self(
            value
                .0
                .chars()
                .map(|ch| match ch {
                    ' ' => None,
                    'S' => Some(Behavior::Shift),
                    c if c.is_ascii_digit() && c != '0' => Some(Behavior::Layer(
                        NonZeroU8::new(c.to_digit(10).unwrap() as u8).unwrap(),
                    )),
                    _ => unreachable!(),
                })
                .collect(),
        )
    }
}

impl From<BaseBehavior> for SerdeBehaviors {
    fn from(value: BaseBehavior) -> Self {
        Self(
            value
                .0
                .into_iter()
                .map(|b| match b {
                    Some(Behavior::Shift) => 'S',
                    Some(Behavior::Layer(layer)) => char::from(b'0' + layer.get()),
                    None => ' ',
                })
                .collect(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    Shift,
    Layer(NonZeroU8),
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct SerdeLayer(String);

impl From<LayoutLayer> for SerdeLayer {
    fn from(value: LayoutLayer) -> Self {
        Self(
            value
                .keys
                .into_iter()
                .map(|x| {
                    x.map_or(' ', |v| match char::from(v.get()) {
                        ' ' => 'S',
                        x => x,
                    })
                })
                .collect(),
        )
    }
}

impl TryFrom<SerdeLayer> for LayoutLayer {
    type Error = TryFromCharError;

    fn try_from(value: SerdeLayer) -> Result<Self, Self::Error> {
        value
            .0
            .chars()
            .map(|x| {
                u8::try_from(match x {
                    'S' => ' ',
                    ' ' => '\0',
                    _ => x,
                })
                .map(NonZeroU8::new)
            })
            .collect::<Result<_, _>>()
            .map(|v| Self::new(v))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "SerdeLayer", into = "SerdeLayer")]
pub struct LayoutLayer {
    keys: Vec<Option<NonZeroU8>>,
}

impl LayoutLayer {
    pub fn new(keys: Vec<Option<NonZeroU8>>) -> Self {
        Self { keys }
    }

    pub fn into_keys(self) -> Vec<Option<NonZeroU8>> {
        self.keys
    }

    pub fn set_key(&mut self, index: usize, key: Option<NonZeroU8>) {
        self.keys[index] = key;
    }

    pub fn keys(&self) -> &[Option<NonZeroU8>] {
        &self.keys
    }

    pub fn keys_mut(&mut self) -> &mut Vec<Option<NonZeroU8>> {
        &mut self.keys
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn key_mut(&mut self, index: usize) -> &mut Option<NonZeroU8> {
        &mut self.keys[index]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "SerdeBehaviors", into = "SerdeBehaviors")]
struct BaseBehavior(Vec<Option<Behavior>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    size: usize,
    base_hold: BaseBehavior,
    layers: Vec<LayoutLayer>,
}

impl Layout {
    pub fn new(base_hold: Vec<Option<Behavior>>, layers: Vec<LayoutLayer>) -> Self {
        let size = base_hold.len();
        layers
            .iter()
            .for_each(|layer| assert_eq!(layer.len(), size));
        Self {
            size,
            base_hold: BaseBehavior(base_hold),
            layers,
        }
    }

    pub fn layers(&self) -> &[LayoutLayer] {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut Vec<LayoutLayer> {
        &mut self.layers
    }

    pub fn layer(&self, layer: u8) -> &LayoutLayer {
        &self.layers[usize::from(layer)]
    }

    pub fn layer_mut(&mut self, layer: u8) -> &mut LayoutLayer {
        &mut self.layers[usize::from(layer)]
    }

    pub fn layer_count(&self) -> u8 {
        self.layers().len() as u8
    }

    pub fn layer_size(&self) -> usize {
        self.layers[0].len()
    }

    pub fn key(&self, layer: u8, index: usize) -> Option<NonZeroU8> {
        self.layers[layer as usize].keys[index]
    }

    pub fn first_layer(&self) -> &LayoutLayer {
        self.layers().first().unwrap()
    }

    pub fn find_on_base<F: FnMut(Behavior) -> bool + Copy>(
        &self,
        mut func: F,
    ) -> impl Iterator<Item = KeyLoc> + use<'_, F> {
        self.base_hold()
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

    pub fn find_all_key<F: FnMut(NonZeroU8) -> bool + Copy>(
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

    pub fn base_hold(&self) -> &[Option<Behavior>] {
        &self.base_hold.0
    }

    pub fn base_hold_mut(&mut self) -> &mut Vec<Option<Behavior>> {
        &mut self.base_hold.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

use array_map::Indexable;
use glam::Vec2;

#[derive(Debug, Clone, Copy, Default, Indexable)]
pub enum Hand {
    #[default]
    Left,
    Right,
}

impl Hand {
    pub const ALL: [Self; 2] = [
        Self::Left,
        Self::Right,
    ];
}

#[derive(Debug, Clone, Copy, Default, Indexable)]
pub enum Finger {
    #[default]
    Thumb,
    Index,
    Middle,
    Ring,
    Pinky,
}

impl Finger {
    pub const ALL: [Self; 5] = [
        Self::Thumb,
        Self::Index,
        Self::Middle,
        Self::Ring,
        Self::Pinky,
    ];
}

#[derive(Debug, Clone, Copy)]
pub struct HandFinger {
    pub hand: Hand,
    pub finger: Finger,
}

impl HandFinger {
    pub fn new(hand: Hand, finger: Finger) -> Self {
        Self { hand, finger }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HandFingerIter {
    hand: Hand,
    finger: Finger,
}

impl Iterator for HandFingerIter {
    type Item = HandFinger;

    fn next(&mut self) -> Option<Self::Item> {
        let current = HandFinger {
            hand: self.hand,
            finger: self.finger,
        };

        let (next_finger, next_hand) = match self.finger {
            Finger::Thumb => (Finger::Index, false),
            Finger::Index => (Finger::Middle, false),
            Finger::Middle => (Finger::Ring, false),
            Finger::Ring => (Finger::Pinky, false),
            Finger::Pinky => (Finger::Thumb, true),
        };

        let (next_hand, alive) = match (self.hand, next_hand) {
            (Hand::Left, true) => (Hand::Right, true),
            (Hand::Left, false) => (Hand::Left, true),
            (Hand::Right, true) => (Hand::Right, false),
            (Hand::Right, false) => (Hand::Right, true),
        };

        self.finger = next_finger;
        self.hand = next_hand;

        alive.then_some(current)
    }
}

unsafe impl Indexable for HandFinger {
    const SIZE: usize = 10;

    const SET_SIZE: usize = array_map::set_size(Self::SIZE);

    type Iter = HandFingerIter;

    fn index(self) -> usize {
        (match self.hand {
            Hand::Left => 0,
            Hand::Right => 5,
        }) + match self.finger {
            Finger::Thumb => 0,
            Finger::Index => 1,
            Finger::Middle => 2,
            Finger::Ring => 3,
            Finger::Pinky => 4,
        }
    }

    fn iter() -> Self::Iter {
        HandFingerIter::default()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Key {
    pos: Vec2,
    finger: HandFinger,
    is_base: bool,
}

impl Key {
    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn finger(&self) -> HandFinger {
        self.finger
    }
}

#[derive(Debug, Clone)]
pub struct Keyboard {
    keys: Vec<Key>,
}

impl Keyboard {
    pub fn new(keys: Vec<Key>) -> Self {
        Self { keys }
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    pub fn key(&self, index: usize) -> Key {
        self.keys[index]
    }

    pub fn ferris_sweep() -> Self {
        const X_SPACING: f32 = 18.0;
        const Y_SPACING: f32 = 17.0;
        const Y_STAGGER: [f32; 5] = [19.0, 7.0, 0.0, 5.5, 8.0];
        const FINGERS: [Finger; 5] = [
            Finger::Pinky,
            Finger::Ring,
            Finger::Middle,
            Finger::Index,
            Finger::Index,
        ];

        Self::new(
            (0..10)
                .flat_map(|ix| {
                    let x = ix as f32 * X_SPACING;
                    let (finger_index, hand) = match ix {
                        0..5 => (ix, Hand::Left),
                        5..10 => (9 - ix, Hand::Right),
                        _ => unreachable!(),
                    };
                    let y_add = Y_STAGGER[finger_index];
                    let finger = FINGERS[finger_index];
                    (0..3).map(move |iy| {
                        let y = y_add + iy as f32 * Y_SPACING;
                        let pos = Vec2::new(x, y);
                        Key {
                            pos,
                            finger: HandFinger::new(hand, finger),
                            is_base: iy == 1 && ix != 4 && ix != 5,
                        }
                    })
                })
                .chain((0..4).map(|i| Key {
                    pos: Vec2::new((i as f32 + 3.0) * X_SPACING, Y_STAGGER[4] + 3.0 * Y_SPACING),
                    finger: HandFinger::new(
                        if i < 2 { Hand::Left } else { Hand::Right },
                        Finger::Thumb,
                    ),
                    is_base: i == 1 || i == 2,
                }))
                .collect(),
        )
    }
}

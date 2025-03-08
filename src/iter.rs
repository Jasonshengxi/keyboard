pub enum OneIter<I: Iterator> {
    One(bool),
    Iter(I),
}

impl<I: Iterator + Clone> Clone for OneIter<I> {
    fn clone(&self) -> Self {
        match self {
            Self::One(arg0) => Self::One(arg0.clone()),
            Self::Iter(arg0) => Self::Iter(arg0.clone()),
        }
    }
}

impl<I: Iterator> OneIter<I> {
    pub fn new(value: Option<I>) -> Self {
        match value {
            Some(i) => Self::Iter(i),
            None => Self::One(true),
        }
    }
}

impl<I: Iterator> From<Option<I>> for OneIter<I> {
    fn from(value: Option<I>) -> Self {
        Self::new(value)
    }
}

impl<I: Iterator> Iterator for OneIter<I> {
    type Item = Option<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            OneIter::One(left @ true) => {
                *left = false;
                Some(None)
            }
            OneIter::One(false) => None,
            OneIter::Iter(i) => i.next().map(Some),
        }
    }
}

pub trait Step: Copy {
    fn next_element(self) -> Option<Self>;
}

#[derive(Debug, Clone, Copy)]
pub struct Range<T> {
    at: Option<T>,
    end: T,
}

impl<T> Range<T> {
    pub fn new(at: T, end: T) -> Self {
        Self { at: Some(at), end }
    }
}

impl<T: Step + Eq> Iterator for Range<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.at;
        if let Some(now) = out {
            let next = now.next_element();
            if next == Some(self.end) {
                self.at = None;
            } else {
                self.at = next;
            }
        }
        out
    }
}

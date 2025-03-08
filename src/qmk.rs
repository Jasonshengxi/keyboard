use anyhow::Result as AnyResult;
use std::{fmt::Display, num::NonZeroU8};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::Serialize;

use crate::layout::{Behavior, Layout};

impl QmkKeymap {
    pub fn from_layout(value: Layout) -> AnyResult<Self> {
        let mut layers = Vec::with_capacity(usize::from(value.layer_count()));

        let (base_hold, layout_layers) = value.into_parts();
        let mut layers_iter = layout_layers.into_iter();

        if let Some(base_layer) = layers_iter.next() {
            let keys = base_layer
                .into_keys()
                .into_iter()
                .zip(base_hold)
                .map(|(key, hold)| {
                    let code = KeyCode::try_from_primitive(key.map_or(0, u8::from))?;
                    Ok::<_, <KeyCode as TryFromPrimitive>::Error>(match hold {
                        None => QmkKey::Direct(code),
                        Some(Behavior::Shift) => QmkKey::ModTapShift(code),
                        Some(Behavior::Layer(layer)) => QmkKey::ModTapLayer(code, layer),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            layers.push(QmkLayer { keys });
        }

        for layer in layers_iter {
            let keys = layer
                .into_keys()
                .into_iter()
                .map(|key| KeyCode::try_from_primitive(key.map_or(0, u8::from)).map(QmkKey::Direct))
                .collect::<Result<Vec<_>, _>>()?;

            layers.push(QmkLayer { keys });
        }

        Ok(Self {
            version: 1,
            notes: "".to_string(),
            documentation: "".to_string(),
            keyboard: "ferris/sweep".to_string(),
            keymap: "optimized".to_string(),
            layout: "LAYOUT_split_3x5_2".to_string(),
            layers,
            author: "JsonJ__".to_string(),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct QmkKeymap {
    version: u32,
    notes: String,
    documentation: String,
    keyboard: String,
    keymap: String,
    layout: String,
    layers: Vec<QmkLayer>,
    author: String,
}

#[derive(Serialize, Clone)]
pub struct QmkLayer {
    keys: Vec<QmkKey>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(into = "String")]
pub enum QmkKey {
    Direct(KeyCode),
    ModTapShift(KeyCode),
    ModTapLayer(KeyCode, NonZeroU8),
}

impl From<QmkKey> for String {
    fn from(value: QmkKey) -> Self {
        value.to_string()
    }
}

impl Display for QmkKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QmkKey::Direct(k) => write!(f, "{}", k.as_str()),
            QmkKey::ModTapShift(k) => write!(f, "LSFT_T({})", k.as_str()),
            QmkKey::ModTapLayer(k, l) => write!(f, "LT({},{})", l.get(), k.as_str()),
        }
    }
}

macro_rules! key_code {
    (
        $(#[$attrs:meta])*
        $v:vis enum $name:ident {
            $($var:ident = $code:literal, $str:literal)*
        }
    ) => {
        $(#[$attrs])*
        $v enum $name {
            $($var = $code),*
        }

        impl $name {
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$var => $str),*
                }
            }
        }
    };
}

impl From<KeyCode> for &'static str {
    fn from(value: KeyCode) -> Self {
        value.as_str()
    }
}

key_code! {
    #[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, Serialize)]
    #[repr(u8)]
    #[serde(into = "&'static str")]
    pub enum KeyCode {
        Null = b'\0', "_______"

        A = b'a', "KC_A"
        B = b'b', "KC_B"
        C = b'c', "KC_C"
        D = b'd', "KC_D"
        E = b'e', "KC_E"
        F = b'f', "KC_F"
        G = b'g', "KC_G"
        H = b'h', "KC_H"
        I = b'i', "KC_I"
        J = b'j', "KC_J"
        K = b'k', "KC_K"
        L = b'l', "KC_L"
        M = b'm', "KC_M"
        N = b'n', "KC_N"
        O = b'o', "KC_O"
        P = b'p', "KC_P"
        Q = b'q', "KC_Q"
        R = b'r', "KC_R"
        S = b's', "KC_S"
        T = b't', "KC_T"
        U = b'u', "KC_U"
        V = b'v', "KC_V"
        W = b'w', "KC_W"
        X = b'x', "KC_X"
        Y = b'y', "KC_Y"
        Z = b'z', "KC_Z"
        N1 = b'1', "KC_1"
        N2 = b'2', "KC_2"
        N3 = b'3', "KC_3"
        N4 = b'4', "KC_4"
        N5 = b'5', "KC_5"
        N6 = b'6', "KC_6"
        N7 = b'7', "KC_7"
        N8 = b'8', "KC_8"
        N9 = b'9', "KC_9"
        N0 = b'0', "KC_0"
        Enter = b'\n', "KC_ENT"
        Tab = b'\t', "KC_TAB"
        Space = b' ', "KC_SPC"
        Minus = b'-', "KC_MINS"
        Equal = b'=', "KC_EQL"
        LeftBracket = b'[', "KC_LBRC"
        RightBracket = b']', "KC_RBRC"
        Backslash = b'\\', "KC_BSLS"
        Semicolon = b';', "KC_SCLN"
        Quote = b'\'', "KC_QUOT"
        Grave = b'`', "KC_GRV"
        Comma = b',', "KC_COMM"
        Dot = b'.', "KC_DOT"
        Slash = b'/', "KC_SLSH"

        Tilde = b'~', "KC_TILD"
        Exclaim = b'!', "KC_EXLM"
        At = b'@', "KC_AT"
        Hash = b'#', "KC_HASH"
        Dollar = b'$', "KC_DLR"
        Percent = b'%', "KC_PERC"
        Circumflex = b'^', "KC_CIRC"
        Ampersand = b'&', "KC_AMPR"
        Asterisk = b'*', "KC_ASTR"
        LeftParen = b'(', "KC_LPRN"
        RightParen = b')', "KC_RPRN"
        Underscore = b'_', "KC_UNDS"
        Plus = b'+', "KC_PLUS"
        LeftBrace = b'{', "KC_LCBR"
        RightBrace = b'}', "KC_RCBR"
        Pipe = b'|', "KC_PIPE"
        Colon = b':', "KC_COLN"
        DoubleQuote = b'"', "KC_DQUO"
        LessThan = b'<', "KC_LT"
        MoreThan = b'>', "KC_GT"
        Question = b'?', "KC_QUES"
    }
}

use egui::{Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct HotkeysConfig {
    #[serde(with = "KeyboardShortcutDef")]
    pub inventory: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub skills: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub character: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub quests: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub clan: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub settings: KeyboardShortcut,

    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_1: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_2: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_3: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_4: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_5: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_6: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_7: KeyboardShortcut,
    #[serde(with = "KeyboardShortcutDef")]
    pub hotbar_8: KeyboardShortcut,
}

impl Default for HotkeysConfig {
    fn default() -> Self {
        Self {
            inventory: KeyboardShortcut::new(Modifiers::ALT, Key::I),
            skills: KeyboardShortcut::new(Modifiers::ALT, Key::S),
            character: KeyboardShortcut::new(Modifiers::ALT, Key::A),
            quests: KeyboardShortcut::new(Modifiers::ALT, Key::Q),
            clan: KeyboardShortcut::new(Modifiers::ALT, Key::N),
            settings: KeyboardShortcut::new(Modifiers::ALT, Key::O),

            hotbar_1: KeyboardShortcut::new(Modifiers::NONE, Key::F1),
            hotbar_2: KeyboardShortcut::new(Modifiers::NONE, Key::F2),
            hotbar_3: KeyboardShortcut::new(Modifiers::NONE, Key::F3),
            hotbar_4: KeyboardShortcut::new(Modifiers::NONE, Key::F4),
            hotbar_5: KeyboardShortcut::new(Modifiers::NONE, Key::F5),
            hotbar_6: KeyboardShortcut::new(Modifiers::NONE, Key::F6),
            hotbar_7: KeyboardShortcut::new(Modifiers::NONE, Key::F7),
            hotbar_8: KeyboardShortcut::new(Modifiers::NONE, Key::F8),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "KeyboardShortcut")]
struct KeyboardShortcutDef {
    #[serde(with = "ModifiersDef")]
    pub modifiers: Modifiers,
    #[serde(with = "KeyDef")]
    pub key: Key,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Modifiers")]
struct ModifiersDef {
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub mac_cmd: bool,
    pub command: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Key")]
enum KeyDef {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Escape,
    Tab,
    Backspace,
    Enter,
    Space,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Minus,
    PlusEquals,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
}

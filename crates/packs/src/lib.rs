//! Контент-паки: манифест, реестры, задания, комбо, ачивки, косметика.

pub mod entities;
pub mod install;
pub mod loader;
pub mod manifest;
pub mod validate;

pub use entities::{
    Achievement, Bundle, Category, CmpOp, Combo, Condition, IconDef, Pack, PaletteColor, Repeat,
    StickerDef, Tag, TaskType, ThemeBase, ThemePalette, Token, TokenKind, Unlock, Variant, Window,
};
pub use install::{InstallReport, Limits};
pub use loader::{Issue, PackSet, Severity};
pub use manifest::{Manifest, Progression, PACK_SCHEMA};
pub use validate::{format_issue, has_errors, validate_pack, validate_set};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION_MAJOR")
}

#[cfg(test)]
mod test_support;

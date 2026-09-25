//! Application artwork alongside the toolkit's standard icons.

use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;

const ARTWORK: &[(&str, &[u8])] = &[
    (
        "argand/app.png",
        include_bytes!("../assets/icons/argand-24.png"),
    ),
    (
        "argand/grid.svg",
        include_bytes!("../assets/icons/grid.svg"),
    ),
    (
        "argand/panel-top.svg",
        include_bytes!("../assets/icons/panel-top.svg"),
    ),
    (
        "argand/panel-left.svg",
        include_bytes!("../assets/icons/panel-left.svg"),
    ),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        match ARTWORK.iter().find(|(name, _)| *name == path) {
            Some((_, data)) => Ok(Some(Cow::Borrowed(data))),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut entries = gpui_kit::assets::Assets.list(path)?;
        entries.extend(
            ARTWORK
                .iter()
                .filter(|(name, _)| name.starts_with(path))
                .map(|(name, _)| (*name).into()),
        );
        Ok(entries)
    }
}

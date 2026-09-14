//! Application artwork alongside the toolkit's standard icons.

use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;

const ARTWORK: &[(&str, &[u8])] = &[
    (
        "argand/app.png",
        include_bytes!("../assets/icons/argand-32.png"),
    ),
    (
        "argand/grid.svg",
        include_bytes!("../assets/icons/grid.svg"),
    ),
    (
        "argand/horizontal.svg",
        include_bytes!("../assets/icons/horizontal.svg"),
    ),
    (
        "argand/vertical.svg",
        include_bytes!("../assets/icons/vertical.svg"),
    ),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match ARTWORK.iter().find(|(name, _)| *name == path) {
            Some((_, data)) => Ok(Some(Cow::Borrowed(data))),
            None => gpui_component_assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut entries = gpui_component_assets::Assets.list(path)?;
        entries.extend(
            ARTWORK
                .iter()
                .filter(|(name, _)| name.starts_with(path))
                .map(|(name, _)| (*name).into()),
        );
        Ok(entries)
    }
}

//! Application artwork alongside the toolkit's standard icons.

use gpui_kit::assets::icon_assets;
use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

// The Lucide toolbar artwork the default component bundle omits.
icon_assets!(
    SegmentIcons,
    [PanelTop, MoveHorizontal, MoveVertical, SquareDashed]
);

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
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, data)) = ARTWORK.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(data)));
        }
        match SegmentIcons.load(path)? {
            Some(data) => Ok(Some(data)),
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
        entries.extend(SegmentIcons.list(path)?);
        Ok(entries)
    }
}

#[cfg(test)]
#[path = "assets_test.rs"]
mod tests;

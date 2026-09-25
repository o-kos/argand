//! Application artwork alongside the toolkit's standard icons.

use gpui_kit::assets::icon_assets;
use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

// The Lucide strip above the plot, which the default component bundle omits.
icon_assets!(SegmentIcons, [PanelTop]);

pub struct Assets;

const ARTWORK: &[(&str, &[u8])] = &[(
    "argand/grid.svg",
    include_bytes!("../assets/icons/grid.svg"),
)];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        let Some((_, data)) = ARTWORK.iter().find(|(name, _)| *name == path) else {
            return gpui_kit::assets::Assets
                .load(path)
                .or_else(|_| SegmentIcons.load(path));
        };
        Ok(Some(Cow::Borrowed(data)))
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
mod tests {
    use super::*;

    #[test]
    fn the_toolbar_icons_resolve_through_the_asset_source() {
        for path in [
            "argand/grid.svg",
            "icons/panel-left.svg",
            "icons/panel-top.svg",
        ] {
            let loaded = Assets.load(path).expect("the source answers").is_some();
            assert!(loaded, "{path} resolves");
        }
    }

    #[test]
    fn the_extra_strip_is_listed_beside_the_default_bundle() {
        let listed = Assets.list("icons/panel-").expect("the source answers");
        assert!(listed.iter().any(|name| name == "icons/panel-left.svg"));
        assert!(listed.iter().any(|name| name == "icons/panel-top.svg"));
    }
}

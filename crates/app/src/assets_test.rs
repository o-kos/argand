use super::*;

#[test]
fn the_toolbar_icons_resolve_through_the_asset_source() {
    for path in [
        "argand/app.png",
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

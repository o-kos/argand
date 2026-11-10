use super::*;

#[test]
fn the_about_tag_resolves_through_the_composed_asset_source() {
    assert!(
        Assets
            .load("icons/tag.svg")
            .expect("the source answers")
            .is_some()
    );
}

use super::*;

#[test]
fn cached_columns_match_full_rendering_after_updates_and_scale_changes() {
    for reduce in [Reduce::Max, Reduce::Mean, Reduce::MeanPower] {
        check_updates(reduce);
    }
}

#[test]
fn fixed_scale_refreshes_changed_columns_and_gaps_without_touching_other_columns() {
    let mut store = ColumnStore::new(5, 2, Reduce::Max);
    let mut cache = SnapshotCache::new(5, 2);
    let mut partial = Partial::new(2, 2);
    partial.cols = vec![0, 4];
    partial.rows = vec![0.01, 0.02, 0.5, 0.7];
    store.absorb(&partial, 2);
    cache.refresh(&store);
    let mut grid = DbGrid {
        width: 5,
        height: 2,
        values: cache.values.clone(),
        t0: 0.0,
        t1: 1.0,
        f0: 0.0,
        f1: 1.0,
    };
    let shading = Shading {
        colormap: Colormap::Oceanic,
        db_min: -80.0,
        db_max: 0.0,
    };
    cache.shade(&grid, shading);
    let before = cache.image.clone();
    partial.cols = vec![0];
    partial.rows = vec![0.1, 0.2];
    store.absorb(&partial, 2);
    cache.dirty[0] = true;
    cache.refresh(&store);
    assert_eq!(cache.dirty, vec![true, true, true, true, false]);
    grid.values.clone_from(&cache.values);
    cache.shade(&grid, shading);
    assert_eq!(cache.image.rgba, shade(&grid, shading).rgba);
    for y in 0..2 {
        let pixel =
            |image: &SpectrogramImage, x: usize| image.get(x, y).expect("a pixel inside the image");
        assert_ne!(pixel(&cache.image, 0), pixel(&before, 0));
        assert_eq!(pixel(&cache.image, 3), pixel(&cache.image, 0));
        assert_eq!(pixel(&cache.image, 4), pixel(&before, 4));
    }
}

fn check_updates(reduce: Reduce) {
    let mut store = ColumnStore::new(7, 3, reduce);
    let mut cache = SnapshotCache::new(7, 3);
    for (column, value) in [(0, 0.1), (4, 0.3), (0, 0.8), (2, 0.5), (4, 0.9)] {
        let mut partial = Partial::new(3, 4);
        partial.cols.push(column);
        partial.rows = vec![value, value * 0.5, 0.0];
        if matches!(reduce, Reduce::Mean) {
            for v in &mut partial.rows {
                *v = 20.0 * v.max(MAG_FLOOR).log10();
            }
        }
        store.absorb(&partial, 3);
        cache.dirty[column as usize] = true;
        cache.refresh(&store);
        let mut values = store.clone().finish();
        if matches!(reduce, Reduce::Max) {
            for v in &mut values {
                *v = 20.0 * v.max(MAG_FLOOR).log10();
            }
        }
        assert_eq!(cache.values, values);
        let grid = DbGrid {
            width: 7,
            height: 3,
            values,
            t0: 1.0,
            t1: 9.0,
            f0: -7.0,
            f1: 7.0,
        };
        for colormap in [Colormap::Oceanic, Colormap::Grayscale] {
            for db_min in [-100.0, -50.0, -50.0] {
                let shading = Shading {
                    colormap,
                    db_min,
                    db_max: 0.0,
                };
                cache.shade(&grid, shading);
                assert_eq!(cache.image.rgba, shade(&grid, shading).rgba);
                assert_eq!((cache.image.t0, cache.image.t1), (1.0, 9.0));
                assert!(cache.dirty.iter().all(|dirty| !dirty));
            }
        }
    }
}

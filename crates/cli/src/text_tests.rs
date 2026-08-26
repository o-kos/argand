use super::*;

fn blank(w: u32, h: u32) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb([0, 0, 0]))
}

fn ink(canvas: &RgbImage) -> usize {
    canvas.pixels().filter(|p| p.0 != [0, 0, 0]).count()
}

#[test]
fn width_grows_with_text_and_size() {
    let t = TextRenderer::new();
    assert!(t.width("mm", 14.0) > t.width("m", 14.0));
    assert!(t.width("12.579 MHz", 20.0) > t.width("12.579 MHz", 10.0));
    assert_eq!(t.width("", 14.0), 0.0);
}

#[test]
fn drawing_puts_ink_on_the_canvas() {
    let t = TextRenderer::new();
    let mut canvas = blank(200, 40);
    assert_eq!(ink(&canvas), 0);
    t.draw(&mut canvas, "24 kHz", 4.0, 26.0, 14.0, Rgb([255, 255, 255]), Align::Left);
    assert!(ink(&canvas) > 20, "expected visible glyphs");
}

#[test]
fn alignment_moves_the_run_around_the_anchor() {
    let t = TextRenderer::new();
    let text = "-110 dB";
    let anchor = 100.0;

    let bounds = |align| {
        let mut canvas = blank(200, 40);
        t.draw(&mut canvas, text, anchor, 26.0, 14.0, Rgb([255, 255, 255]), align);
        let xs: Vec<u32> = canvas
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0 != [0, 0, 0])
            .map(|(x, _, _)| x)
            .collect();
        (*xs.iter().min().unwrap(), *xs.iter().max().unwrap())
    };

    let (left_min, _) = bounds(Align::Left);
    let (center_min, center_max) = bounds(Align::Center);
    let (_, right_max) = bounds(Align::Right);

    assert!(left_min >= anchor as u32 - 1, "left starts at the anchor");
    assert!(right_max <= anchor as u32 + 1, "right ends at the anchor");
    let center = (center_min + center_max) / 2;
    assert!(center.abs_diff(anchor as u32) <= 2, "centre straddles it");
}

#[test]
fn drawing_off_canvas_does_not_panic() {
    let t = TextRenderer::new();
    let mut canvas = blank(40, 20);
    for (x, y) in [(-500.0, 10.0), (500.0, 10.0), (10.0, -500.0), (10.0, 500.0)] {
        t.draw(&mut canvas, "edge", x, y, 14.0, Rgb([255, 255, 255]), Align::Left);
    }
}

#[test]
fn glyphs_blend_rather_than_overwrite() {
    let t = TextRenderer::new();
    let mut canvas = RgbImage::from_pixel(120, 40, Rgb([20, 30, 40]));
    t.draw(&mut canvas, "iiii", 4.0, 28.0, 16.0, Rgb([255, 255, 255]), Align::Left);
    // Anti-aliased edges land between the background and the text colour.
    let blended = canvas
        .pixels()
        .any(|p| p.0[0] > 20 && p.0[0] < 255);
    assert!(blended, "expected partially covered pixels");
}

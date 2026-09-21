use super::*;
use pretty_assertions::assert_eq;
use ratatui::layout::Rect;

fn write_png(path: &Path, width: u32, height: u32) {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([10u8, 20u8, 30u8, 255u8]));
    image.save(path).unwrap();
}

fn wide_area() -> Rect {
    Rect::new(0, 0, 100, 40)
}

#[test]
fn layout_stacks_thumbnails_above_bottom() {
    let dir = tempfile::tempdir().unwrap();
    let square = dir.path().join("square.png");
    write_png(&square, 64, 64);
    // 24 cols * 64/64 / 2 = 12 rows, clamped to 8.
    let placements =
        layout_message_images(wide_area(), 40, std::slice::from_ref(&square));
    assert_eq!(placements.len(), 1);
    let placement = &placements[0];
    assert_eq!(placement.path, square);
    assert_eq!(placement.columns, 24);
    assert_eq!(placement.rows, 8);
    assert_eq!(placement.x, 100 - 24 - 1);
    assert_eq!(placement.y, 40 - 1 - 8);
}

#[test]
fn layout_derives_rows_from_aspect_ratio() {
    let dir = tempfile::tempdir().unwrap();
    let wide = dir.path().join("wide.png");
    write_png(&wide, 160, 40);
    // 24 * 40/160 / 2 = 3 rows.
    let placements = layout_message_images(wide_area(), 40, &[wide]);
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].rows, 3);
}

#[test]
fn layout_stacks_multiple_images_upward() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.png");
    let second = dir.path().join("second.png");
    write_png(&first, 64, 64);
    write_png(&second, 160, 40);
    let placements = layout_message_images(wide_area(), 40, &[first, second]);
    assert_eq!(placements.len(), 2);
    assert_eq!(placements[0].rows, 8);
    assert_eq!(placements[0].y, 40 - 1 - 8);
    assert_eq!(placements[1].rows, 3);
    assert_eq!(placements[1].y, 40 - 1 - 8 - 1 - 3);
    assert!(placements[0].key != placements[1].key);
}

#[test]
fn layout_caps_visible_images() {
    let dir = tempfile::tempdir().unwrap();
    let mut images = Vec::new();
    for index in 0..MAX_OVERLAY_IMAGES + 2 {
        let path = dir.path().join(format!("img{index}.png"));
        write_png(&path, 160, 40);
        images.push(path);
    }
    let placements = layout_message_images(wide_area(), 40, &images);
    assert_eq!(placements.len(), MAX_OVERLAY_IMAGES);
}

#[test]
fn layout_skips_missing_and_narrow() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.png");
    assert!(layout_message_images(wide_area(), 40, &[missing]).is_empty());

    let square = dir.path().join("square.png");
    write_png(&square, 64, 64);
    let narrow = Rect::new(0, 0, 10, 40);
    assert!(layout_message_images(narrow, 40, &[square]).is_empty());
}

#[test]
fn image_cache_key_changes_with_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("img.png");
    write_png(&path, 64, 64);
    let before = image_cache_key(&path).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    write_png(&path, 128, 128);
    let after = image_cache_key(&path).unwrap();
    assert!(before != after);
}

#[test]
fn draw_transmits_once_places_every_frame_and_deletes_stale() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("img.png");
    write_png(&path, 64, 64);
    let placements = layout_message_images(wide_area(), 40, &[path]);
    assert_eq!(placements.len(), 1);

    let mut state = MessageImageOverlayState::default();
    let mut first = Vec::new();
    draw_message_images(&mut first, &mut state, &placements, ImageProtocol::Kitty).unwrap();
    let first = String::from_utf8_lossy(&first);
    // Transmit (a=T) once, then place (a=p) every frame.
    assert_eq!(first.matches("a=T").count(), 1);
    assert_eq!(first.matches("a=p").count(), 1);
    assert!(first.contains("f=24"));

    let mut second = Vec::new();
    draw_message_images(&mut second, &mut state, &placements, ImageProtocol::Kitty).unwrap();
    let second = String::from_utf8_lossy(&second);
    assert_eq!(second.matches("a=T").count(), 0);
    assert_eq!(second.matches("a=p").count(), 1);

    let mut third = Vec::new();
    draw_message_images(&mut third, &mut state, &[], ImageProtocol::Kitty).unwrap();
    let third = String::from_utf8_lossy(&third);
    assert!(third.contains("a=d"));
    assert!(state.visible.is_empty());
    assert!(state.transmitted.is_empty());
}

#[test]
fn draw_skips_non_kitty_protocols() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("img.png");
    write_png(&path, 64, 64);
    let placements = layout_message_images(wide_area(), 40, &[path]);
    let mut state = MessageImageOverlayState::default();
    let mut out = Vec::new();
    draw_message_images(&mut out, &mut state, &placements, ImageProtocol::Sixel).unwrap();
    assert!(out.is_empty());
}

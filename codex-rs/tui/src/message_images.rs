//! Terminal image overlays for message attachments.
//!
//! When the terminal speaks the kitty graphics protocol, attached images render
//! as thumbnails in a wrap-reserved band at the right edge, stacked upward from
//! the composer (above the ambient pet when one is shown). Placement is
//! recomputed every frame from the current layout; pixel data is transmitted
//! once per image and re-placed by id. Terminals without kitty support keep the
//! `[Image #N]` text labels. Sixel is intentionally unsupported for now: the
//! per-frame redraw/clear dance is not worth it for a v1.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use ratatui::layout::Rect;

use crate::pets::image_protocol;
use crate::pets::image_protocol::ImageProtocol;

/// Fixed thumbnail width in terminal columns.
const THUMB_COLUMNS: u16 = 24;
/// Thumbnail height bounds in terminal rows.
const THUMB_MIN_ROWS: u16 = 3;
const THUMB_MAX_ROWS: u16 = 8;
/// Gap between the overlay band and the viewport edges/content.
const OVERLAY_GAP_COLUMNS: u16 = 1;
const OVERLAY_GAP_ROWS: u16 = 1;
/// Maximum thumbnails stacked in the overlay.
pub(crate) const MAX_OVERLAY_IMAGES: usize = 3;
/// Images larger than this are never transmitted.
const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;
/// First kitty image id for message overlays (pets use 0xC0DE/0xC0DF).
const FIRST_IMAGE_ID: u32 = 0x1000;

/// Approximate terminal cell aspect (height_px / width_px) used to derive
/// thumbnail rows from the image aspect ratio.
const CELL_ASPECT: f64 = 2.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageImagePlacement {
    /// Stable key for the transmit cache (path + length + mtime).
    pub(crate) key: u64,
    pub(crate) path: PathBuf,
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) columns: u16,
    pub(crate) rows: u16,
}

/// Width in columns the overlay reserves from text wrapping, including the gap.
pub(crate) fn overlay_reserved_columns() -> u16 {
    THUMB_COLUMNS.saturating_add(OVERLAY_GAP_COLUMNS)
}

/// Lay out up to [`MAX_OVERLAY_IMAGES`] thumbnails stacked upward from
/// `bottom_y` (exclusive) in the right edge band. Images that do not fit above
/// `area.y`, are unreadable, or exceed [`MAX_IMAGE_BYTES`] are skipped.
pub(crate) fn layout_message_images(
    area: Rect,
    bottom_y: u16,
    images: &[PathBuf],
) -> Vec<MessageImagePlacement> {
    if area.width < THUMB_COLUMNS.saturating_add(OVERLAY_GAP_COLUMNS) {
        return Vec::new();
    }
    let x = area
        .x
        .saturating_add(area.width)
        .saturating_sub(THUMB_COLUMNS)
        .saturating_sub(OVERLAY_GAP_COLUMNS);
    let mut cursor_bottom = bottom_y.saturating_sub(OVERLAY_GAP_ROWS);
    let mut placements = Vec::new();
    for path in images.iter().take(MAX_OVERLAY_IMAGES) {
        let Some(key) = image_cache_key(path) else {
            continue;
        };
        let rows = thumbnail_rows(path);
        let top = cursor_bottom.saturating_sub(rows);
        if top < area.y || rows == 0 {
            continue;
        }
        placements.push(MessageImagePlacement {
            key,
            path: path.clone(),
            x,
            y: top,
            columns: THUMB_COLUMNS,
            rows,
        });
        cursor_bottom = top.saturating_sub(OVERLAY_GAP_ROWS);
    }
    placements
}

fn thumbnail_rows(path: &Path) -> u16 {
    let (width, height) = match image::image_dimensions(path) {
        Ok((width, height)) if width > 0 && height > 0 => (width, height),
        _ => return (THUMB_MIN_ROWS + THUMB_MAX_ROWS) / 2,
    };
    let rows = f64::from(THUMB_COLUMNS) * f64::from(height) / f64::from(width) / CELL_ASPECT;
    (rows.round() as u16).clamp(THUMB_MIN_ROWS, THUMB_MAX_ROWS)
}

fn image_cache_key(path: &Path) -> Option<u64> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata.modified().ok().hash(&mut hasher);
    Some(hasher.finish())
}

#[derive(Debug, Default)]
pub(crate) struct MessageImageOverlayState {
    transmitted: HashMap<u64, u32>,
    pub(crate) visible: HashSet<u32>,
    next_id: u32,
}

/// Draw the current placements: transmit new images once, re-place every
/// visible image, and delete images that are no longer placed.
pub(crate) fn draw_message_images(
    writer: &mut impl Write,
    state: &mut MessageImageOverlayState,
    placements: &[MessageImagePlacement],
    protocol: ImageProtocol,
) -> std::io::Result<()> {
    use crossterm::cursor::MoveTo;
    use crossterm::cursor::RestorePosition;
    use crossterm::cursor::SavePosition;
    use crossterm::queue;

    if !matches!(
        protocol,
        ImageProtocol::Kitty | ImageProtocol::KittyLocalFile
    ) {
        clear_message_images(writer, state)?;
        return Ok(());
    }

    let mut placed = HashSet::new();
    for placement in placements {
        let image_id = match state.transmitted.get(&placement.key) {
            Some(image_id) => *image_id,
            None => {
                let Some(payload) = transmit_thumbnail(&placement.path) else {
                    continue;
                };
                let image_id = state.next_id.max(FIRST_IMAGE_ID);
                state.next_id = image_id.wrapping_add(1);
                let Ok(command) = image_protocol::kitty_transmit_rgba_with_id(
                    &payload.rgba,
                    payload.width_px,
                    payload.height_px,
                    placement.columns,
                    placement.rows,
                    Some(image_id),
                ) else {
                    continue;
                };
                queue!(writer, SavePosition)?;
                queue!(writer, MoveTo(placement.x, placement.y))?;
                write!(writer, "{command}")?;
                queue!(writer, RestorePosition)?;
                state.transmitted.insert(placement.key, image_id);
                image_id
            }
        };
        queue!(writer, SavePosition)?;
        queue!(writer, MoveTo(placement.x, placement.y))?;
        write!(
            writer,
            "{}",
            image_protocol::kitty_place_image(image_id, placement.columns, placement.rows)
        )?;
        queue!(writer, RestorePosition)?;
        placed.insert(image_id);
    }

    for image_id in state.visible.difference(&placed) {
        write!(writer, "{}", image_protocol::kitty_delete_image(*image_id))?;
    }
    state.visible = placed;
    state.transmitted.retain(|_, id| state.visible.contains(id));
    writer.flush()?;
    Ok(())
}

/// Delete every visible overlay image.
pub(crate) fn clear_message_images(
    writer: &mut impl Write,
    state: &mut MessageImageOverlayState,
) -> std::io::Result<()> {
    for image_id in state.visible.drain() {
        write!(writer, "{}", image_protocol::kitty_delete_image(image_id))?;
    }
    state.transmitted.clear();
    writer.flush()?;
    Ok(())
}

struct Thumbnail {
    rgba: Vec<u8>,
    width_px: u32,
    height_px: u32,
}

fn transmit_thumbnail(path: &Path) -> Option<Thumbnail> {
    let data = std::fs::read(path).ok()?;
    if data.is_empty() || data.len() as u64 > MAX_IMAGE_BYTES {
        return None;
    }
    let decoded = image::load_from_memory(&data).ok()?;
    let thumb = decoded.thumbnail(320, 320);
    let rgba = thumb.to_rgba8();
    let (width_px, height_px) = rgba.dimensions();
    if width_px == 0 || height_px == 0 {
        return None;
    }
    Some(Thumbnail {
        rgba: rgba.into_raw(),
        width_px,
        height_px,
    })
}

#[cfg(test)]
#[path = "message_images_tests.rs"]
mod tests;

//! Screen-context pipeline: capture a screen region and extract its text.
//!
//! Privacy rules enforced here:
//! - Pixels live only in memory (DIB section -> BMP byte buffer -> WinRT
//!   in-memory stream). Nothing touches disk.
//! - Only called at activation, gated by the user's settings.
//! - The raw image is dropped as soon as OCR completes; only extracted
//!   text survives (truncated before entering prompts).

use crate::windows::Rect;
use windows::Graphics::Imaging::{BitmapDecoder, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, SRCCOPY, STRETCH_HALFTONE,
};

/// Longest edge allowed into the OCR engine; larger regions are downscaled.
const MAX_OCR_EDGE: i32 = 1900;
/// Characters of OCR text injected into a prompt.
const PROMPT_TEXT_LIMIT: usize = 1400;

/// Extract readable text from a physical-pixel screen rectangle.
/// Empty string means "nothing legible" (not an error).
pub fn capture_screen_text(rect: Rect) -> Result<String, String> {
    if rect.width <= 0 || rect.height <= 0 {
        return Ok(String::new());
    }

    let (pixels, w, h) = capture_region_bgra(&rect)?;
    let bmp_bytes = encode_bmp(&pixels, w, h);
    ocr_bmp(&bmp_bytes)
}

/// BitBlt the region into a 32bpp top-down DIB, downscaling when huge.
/// Returns tightly packed BGRA pixels.
fn capture_region_bgra(rect: &Rect) -> Result<(Vec<u8>, i32, i32), String> {
    unsafe {
        let screen_dc = unsafe { GetDC(None) };
        if screen_dc.is_invalid() {
            return Err("GetDC(screen) failed".into());
        }
        let mem_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };

        let mut w = rect.width;
        let mut h = rect.height;
        let scale_down = w > MAX_OCR_EDGE || h > MAX_OCR_EDGE;
        if scale_down {
            let factor = MAX_OCR_EDGE as f32 / w.max(h) as f32;
            w = ((w as f32) * factor).round() as i32;
            h = ((h as f32) * factor).round() as i32;
        }

        let mut bi = BITMAPINFO::default();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w;
        // Negative height => top-down rows (easier to reason about).
        bi.bmiHeader.biHeight = -h;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0;

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(Some(mem_dc), &bi, DIB_RGB_COLORS, &mut bits, None, 0)
            .map_err(|e| format!("CreateDIBSection: {e}"))?;

        let old = SelectObject(mem_dc, hbmp.into());

        let blt_ok = if scale_down {
            let _ = SetStretchBltMode(mem_dc, STRETCH_HALFTONE);
            StretchBlt(
                mem_dc, 0, 0, w, h,
                Some(screen_dc), rect.x, rect.y, rect.width, rect.height,
                SRCCOPY,
            )
            .as_bool()
        } else {
            BitBlt(
                mem_dc, 0, 0, w, h,
                Some(screen_dc), rect.x, rect.y,
                SRCCOPY,
            )
            .is_ok()
        };

        let len = (w as usize) * (h as usize) * 4;
        let mut pixels = Vec::<u8>::with_capacity(len);
        std::ptr::copy_nonoverlapping(bits as *const u8, pixels.as_mut_ptr(), len);
        pixels.set_len(len);

        SelectObject(mem_dc, old);
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);

        if !blt_ok {
            return Err("BitBlt/StretchBlt failed".into());
        }
        Ok((pixels, w, h))
    }
}

/// Wrap raw BGRA rows into a bottom-up BMP container in memory.
fn encode_bmp(pixels: &[u8], w: i32, h: i32) -> Vec<u8> {
    const FILE_HEADER_LEN: usize = 14;
    const INFO_HEADER_LEN: usize = 40;

    // BMP conventionally stores rows bottom-up; flip our top-down rows.
    let row_len = (w as usize) * 4;
    let mut flipped = vec![0u8; pixels.len()];
    for row in 0..h as usize {
        let src = row * row_len;
        let dst = (h as usize - 1 - row) * row_len;
        flipped[dst..dst + row_len].copy_from_slice(&pixels[src..src + row_len]);
    }

    let data_offset = (FILE_HEADER_LEN + INFO_HEADER_LEN) as u32;

    let mut out = Vec::with_capacity(data_offset as usize + flipped.len());
    // BITMAPFILEHEADER (packed, little-endian).
    out.push(b'B');
    out.push(b'M');
    out.extend_from_slice(&(data_offset as u32 + flipped.len() as u32).to_le_bytes()); // size
    out.extend_from_slice(&[0u8; 4]); // reserved
    out.extend_from_slice(&data_offset.to_le_bytes());

    // BITMAPINFOHEADER.
    out.extend_from_slice(&(INFO_HEADER_LEN as u32).to_le_bytes()); // biSize
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes()); // positive => bottom-up
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bpp
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(flipped.len() as u32).to_le_bytes()); // size image
    out.extend_from_slice(&0u32.to_le_bytes()); // x pels
    out.extend_from_slice(&0u32.to_le_bytes()); // y pels
    out.extend_from_slice(&0u32.to_le_bytes()); // clr used
    out.extend_from_slice(&0u32.to_le_bytes()); // clr important

    out.extend_from_slice(&flipped);
    out
}

/// Decode the in-memory BMP and run Windows OCR over it.
fn ocr_bmp(bmp: &[u8]) -> Result<String, String> {
    unsafe {
        let stream =
            InMemoryRandomAccessStream::new().map_err(|e| format!("stream: {e}"))?;
        let output = stream
            .GetOutputStreamAt(0)
            .map_err(|e| format!("output stream: {e}"))?;
        let writer =
            DataWriter::CreateDataWriter(&output).map_err(|e| format!("writer: {e}"))?;
        writer.WriteBytes(bmp).map_err(|e| format!("write bytes: {e}"))?;
        writer
            .StoreAsync()
            .map_err(|e| format!("store op: {e}"))?
            .get()
            .map_err(|e| format!("store: {e}"))?;
        writer
            .FlushAsync()
            .map_err(|e| format!("flush op: {e}"))?
            .get()
            .map_err(|e| format!("flush: {e}"))?;
        drop(writer);
        stream.Seek(0).map_err(|e| format!("seek: {e}"))?;

        let bmp_guid = BitmapDecoder::BmpDecoderId().map_err(|e| format!("bmp guid: {e}"))?;
        let decoder: BitmapDecoder = BitmapDecoder::CreateWithIdAsync(bmp_guid, &stream)
            .map_err(|e| format!("decoder op: {e}"))?
            .get()
            .map_err(|e| format!("decoder: {e}"))?;

        let soft: SoftwareBitmap = decoder
            .GetSoftwareBitmapAsync()
            .map_err(|e| format!("bitmap op: {e}"))?
            .get()
            .map_err(|e| format!("software bitmap: {e}"))?;

        let engine = match create_ocr_engine() {
            Some(e) => e,
            None => return Err("no OCR language available".into()),
        };

        let result = engine
            .RecognizeAsync(&soft)
            .map_err(|e| format!("recognize op: {e}"))?
            .get()
            .map_err(|e| format!("recognize: {e}"))?;

        let text = result.Text().map_err(|e| format!("text: {e}"))?;
        Ok(normalize_text(text.to_string()))
    }
}

/// Prefer the user's own OCR languages; fall back to zh / en recognizers.
fn create_ocr_engine() -> Option<OcrEngine> {
    if let Ok(e) = OcrEngine::TryCreateFromUserProfileLanguages() {
        return Some(e);
    }
    if let Ok(langs) = OcrEngine::AvailableRecognizerLanguages() {
        for lang in langs {
            let tag = lang.LanguageTag().ok()?.to_string();
            if tag.starts_with("zh") || tag == "en-US" {
                if let Ok(e) = OcrEngine::TryCreateFromLanguage(&lang) {
                    return Some(e);
                }
            }
        }
    }
    None
}

fn normalize_text(s: String) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Trim OCR output for prompt injection.
pub fn excerpt_for_prompt(text: &str) -> String {
    let mut end = text.len().min(PROMPT_TEXT_LIMIT);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let cut = &text[..end];
    if text.len() > PROMPT_TEXT_LIMIT {
        format!("{cut}…")
    } else {
        cut.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excerpt_respects_char_boundaries_and_limit() {
        let long = "字".repeat(PROMPT_TEXT_LIMIT + 100);
        let ex = excerpt_for_prompt(&long);
        assert!(ex.chars().count() <= PROMPT_TEXT_LIMIT + 1); // + ellipsis
        assert!(ex.ends_with('…'));

        let short = "hello";
        assert_eq!(excerpt_for_prompt(short), "hello");
    }

    #[test]
    fn empty_rect_is_empty_not_error() {
        let r = Rect { x: 0, y: 0, width: 0, height: 10 };
        assert_eq!(capture_screen_text(r).unwrap(), "");
    }

    #[test]
    fn bmp_header_is_wellformed() {
        // Two BGRA rows of one pixel each (top-down input).
        let row0 = [10u8, 20, 30, 255];
        let row1 = [40u8, 50, 60, 255];
        let mut pixels = Vec::new();
        pixels.extend_from_slice(&row0);
        pixels.extend_from_slice(&row1);
        let bmp = encode_bmp(&pixels, 1, 2);
        assert_eq!(bmp[0], b'B');
        assert_eq!(bmp[1], b'M');
        let size = u32::from_le_bytes([bmp[2], bmp[3], bmp[4], bmp[5]]) as usize;
        assert_eq!(size, bmp.len());
        let data_offset = u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]) as usize;
        assert_eq!(data_offset, 54);
        // Bottom-up storage flips rows: row1 lands first.
        assert_eq!(bmp[54..58], row1);
        assert_eq!(bmp[58..62], row0);
    }

    /// Manual end-to-end check on a real desktop:
    /// `cargo test --lib -- --ignored --nocapture capture_real_screen`
    #[test]
    #[ignore]
    fn capture_real_screen_ocr() {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let rect = Rect {
            x: w / 6,
            y: h / 4,
            width: w * 2 / 3,
            height: h / 2,
        };
        let text = capture_screen_text(rect).expect("capture+ocr should not fail");
        println!("OCR captured {} chars", text.chars().count());
        println!("preview: {}", &text.chars().take(200).collect::<String>());
        // Pipeline must run; emptiness is legal (e.g. empty desktop area).
        assert!(text.chars().count() < 100_000);
    }
}

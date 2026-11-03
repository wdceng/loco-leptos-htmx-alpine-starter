//! A picture shrunk in the browser before it goes, so a phone's photo of
//! 4 to 8 MB doesn't go up whole over mobile data: a file field marked
//! `data-shrink`, its longest side, and `data-shrink-under`, the bytes it
//! is brought under (an avatar's, `uploads::SHRINK_SIDE` and
//! `SHRINK_UNDER`, views/account.rs), armed by the `Navigation` island on
//! every page. The picture chosen is read by the browser, turned upright
//! by its EXIF, drawn on a canvas at most that long on white, and saved as
//! a JPEG, the one format every browser makes from a canvas (Safari makes
//! no WebP), at the first of [`QUALITIES`] that brings it under the bytes;
//! the field then holds that file, so the form sends it, plain or htmx's.
//! A JPEG already within both is left as it is. The server is unchanged:
//! it decodes whatever comes, that being its check (src/uploads.rs);
//! where no WASM runs, or the browser can't read the picture (an iPhone's
//! HEIC in Chrome), the original goes and the server says what it says of
//! it.
//!
//! While it works the form's buttons are out of use, so a click can't send
//! the large one, and back in use once every picture chosen is done. As an
//! islands app on this stack shrinks its photos, its crop left out.

/// The JPEG qualities tried in turn until the picture is under its bytes,
/// the last one kept where none is.
#[cfg(any(feature = "hydrate", test))]
const QUALITIES: [f64; 4] = [0.9, 0.85, 0.8, 0.75];

#[cfg(feature = "hydrate")]
pub(super) use browser::chosen;

/// The server compiles the island too, but never runs a browser's change.
#[cfg(not(feature = "hydrate"))]
pub(super) fn chosen(_field: web_sys::HtmlInputElement) {}

#[cfg(feature = "hydrate")]
mod browser {
    use std::cell::Cell;

    use leptos::{
        prelude::*,
        task::spawn_local,
        wasm_bindgen::{JsCast, JsValue},
    };
    use wasm_bindgen_futures::JsFuture;
    use web_sys::js_sys::{Array, Promise};

    use super::{QUALITIES, fit, jpeg_name};

    thread_local! {
        /// The pictures being shrunk: the form's buttons back in use when
        /// the last one is done, a second chosen while the first works
        /// included.
        static WORKING: Cell<u32> = const { Cell::new(0) };
        /// The latest run's number: an earlier one still working puts
        /// nothing in the field.
        static LATEST: Cell<u64> = const { Cell::new(0) };
    }

    /// A picture chosen in `field`: shrunk in the background, its form's
    /// buttons out of use meanwhile. Nothing where the field holds no file
    /// or its numbers are missing.
    pub fn chosen(field: web_sys::HtmlInputElement) {
        let number = |name: &str| field.get_attribute(name)?.parse::<u32>().ok();
        let (Some(side), Some(under), Some(file)) = (
            number("data-shrink"),
            number("data-shrink-under"),
            field.files().and_then(|files| files.get(0)),
        ) else {
            return;
        };
        let turn = LATEST.get() + 1;
        LATEST.set(turn);
        WORKING.set(WORKING.get() + 1);
        hold(&field, true);
        spawn_local(async move {
            if let Some(picture) = shrink(&file, side, f64::from(under)).await
                && LATEST.get() == turn
            {
                put(&field, &picture, &file.name());
            }
            WORKING.set(WORKING.get().saturating_sub(1));
            if WORKING.get() == 0 {
                hold(&field, false);
            }
        });
    }

    /// The field's form's buttons out of use (`held`) or back in use.
    fn hold(field: &web_sys::HtmlInputElement, held: bool) {
        let Some(form) = field.form() else {
            return;
        };
        let Ok(buttons) = form.query_selector_all("button") else {
            return;
        };
        for at in 0..buttons.length() {
            if let Some(button) = buttons
                .item(at)
                .and_then(|node| node.dyn_into::<web_sys::HtmlButtonElement>().ok())
            {
                button.set_disabled(held);
            }
        }
    }

    /// What to send in `file`'s place: the picture at most `side` long
    /// and, where a quality reaches it, under `under` bytes; none where the
    /// file is a JPEG within both already, or the browser can't read or
    /// draw it.
    async fn shrink(file: &web_sys::File, side: u32, under: f64) -> Option<web_sys::Blob> {
        let bitmap: web_sys::ImageBitmap =
            JsFuture::from(window().create_image_bitmap_with_blob(file).ok()?)
                .await
                .ok()?
                .dyn_into()
                .ok()?;
        let (whole_width, whole_height) = (bitmap.width(), bitmap.height());
        if whole_width.max(whole_height) <= side
            && file.type_() == "image/jpeg"
            && file.size() <= under
        {
            bitmap.close();
            return None;
        }
        let (width, height) = fit(whole_width, whole_height, side);
        let canvas: web_sys::HtmlCanvasElement =
            document().create_element("canvas").ok()?.dyn_into().ok()?;
        canvas.set_width(width);
        canvas.set_height(height);
        let context: web_sys::CanvasRenderingContext2d =
            canvas.get_context("2d").ok()??.dyn_into().ok()?;
        // White under it: a PNG's transparent parts would turn black in a
        // JPEG.
        context.set_fill_style_str("white");
        context.fill_rect(0.0, 0.0, f64::from(width), f64::from(height));
        let drawn = context.draw_image_with_image_bitmap_and_dw_and_dh(
            &bitmap,
            0.0,
            0.0,
            f64::from(width),
            f64::from(height),
        );
        bitmap.close();
        drawn.ok()?;
        let mut last = None;
        for quality in QUALITIES {
            let picture = jpeg(&canvas, quality).await?;
            if picture.size() <= under {
                return Some(picture);
            }
            last = Some(picture);
        }
        last
    }

    /// The canvas as a JPEG at `quality`; none where the browser makes
    /// none.
    async fn jpeg(canvas: &web_sys::HtmlCanvasElement, quality: f64) -> Option<web_sys::Blob> {
        let made = Promise::new(&mut |done, failed| {
            if canvas
                .to_blob_with_type_and_encoder_options(&done, "image/jpeg", &JsValue::from(quality))
                .is_err()
            {
                let _ = failed.call0(&JsValue::NULL);
            }
        });
        JsFuture::from(made).await.ok()?.dyn_into().ok()
    }

    /// `picture` put in the field in place of the file chosen, named as it
    /// was with the JPEG's ending.
    fn put(field: &web_sys::HtmlInputElement, picture: &web_sys::Blob, name: &str) -> Option<()> {
        let options = web_sys::FilePropertyBag::new();
        options.set_type("image/jpeg");
        let file = web_sys::File::new_with_blob_sequence_and_options(
            &Array::of1(picture),
            &jpeg_name(name),
            &options,
        )
        .ok()?;
        let files = web_sys::DataTransfer::new().ok()?;
        files.items().add_with_file(&file).ok()?;
        field.set_files(files.files().as_ref());
        Some(())
    }
}

/// The size a picture is drawn at: its longer side at most `side`, the
/// other in proportion, rounded; never enlarged.
#[cfg(any(feature = "hydrate", test))]
fn fit(width: u32, height: u32, side: u32) -> (u32, u32) {
    let longer = width.max(height);
    if longer <= side {
        return (width, height);
    }
    let scaled = |length: u32| {
        let length =
            (u64::from(length) * u64::from(side) + u64::from(longer) / 2) / u64::from(longer);
        u32::try_from(length).unwrap_or(side).max(1)
    };
    (scaled(width), scaled(height))
}

/// `name` with the JPEG's ending in place of its own.
#[cfg(any(feature = "hydrate", test))]
fn jpeg_name(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let stem = if stem.is_empty() { "photo" } else { stem };
    format!("{stem}.jpg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_shrunk_to_its_longer_side_in_proportion() {
        assert_eq!(fit(4000, 3000, 2048), (2048, 1536));
        assert_eq!(fit(3000, 4000, 2048), (1536, 2048));
        assert_eq!(fit(2049, 1000, 2048), (2048, 1000));
        assert_eq!(fit(4096, 1001, 2048), (2048, 501));
    }

    #[test]
    fn a_picture_within_its_side_is_never_enlarged() {
        assert_eq!(fit(1200, 800, 2048), (1200, 800));
        assert_eq!(fit(9000, 1, 2048), (2048, 1));
    }

    #[test]
    fn the_qualities_go_down() {
        assert!(QUALITIES.windows(2).all(|pair| pair[0] > pair[1]));
    }

    #[test]
    fn the_file_keeps_its_name_with_the_jpegs_ending() {
        assert_eq!(jpeg_name("IMG_1234.HEIC"), "IMG_1234.jpg");
        assert_eq!(jpeg_name("photo.final.png"), "photo.final.jpg");
        assert_eq!(jpeg_name(".png"), "photo.jpg");
    }
}

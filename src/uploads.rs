//! The pictures people upload, an account's avatar the template's one:
//! checked and made into what the pages show, on the server, with fixed
//! settings, and kept outside the site's folder (`settings.uploads.path`).
//! Only the picture the pages show is kept, never the upload. As an islands
//! app on this stack keeps its photos.
//!
//! - The browser shrinks a large photo before it goes ([`SHRINK_SIDE`],
//!   [`SHRINK_UNDER`], the `shrink` in src/islands.rs), so a phone's 4 to 8
//!   MB photo doesn't go up over mobile data. Where no WASM runs, the
//!   original goes, up to [`MAX_BYTES`].
//! - The server decodes whatever comes, that being its check: a JPEG, a
//!   PNG or a WebP, at most [`MAX_SIDE`] pixels a side, so no picture takes
//!   the server's memory to decode. It turns it upright by its EXIF, cuts
//!   the middle square, makes it [`AVATAR_SIDE`] pixels and saves it as a
//!   JPEG on white ([`avatar`]). The work runs off the async threads.
//! - The file is named by its contents' hash ([`name_of`]), so a new avatar
//!   has a new address and every file can be cached for a year
//!   ([`PREFIX`], controllers/uploads.rs).

use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use image::{
    DynamicImage, ImageDecoder, ImageReader, Limits, Rgb, RgbImage, codecs::jpeg::JpegEncoder,
    imageops::FilterType,
};
use loco_rs::{Error, Result};
use sha2::{Digest, Sha256};

use crate::settings::UploadsSettings;

/// The largest upload taken, in bytes: the upload's body limit too.
pub const MAX_BYTES: usize = 10_000_000;

/// The most pixels a side of the upload may have.
pub const MAX_SIDE: u32 = 6_000;

/// An avatar's side, in pixels: twice the largest it is shown at.
pub const AVATAR_SIDE: u32 = 256;

/// What the browser shrinks a picture to before it goes: its longer side
/// at most this.
pub const SHRINK_SIDE: u32 = 1_024;

/// The bytes the browser brings a shrunk picture under.
pub const SHRINK_UNDER: u32 = 1_000_000;

/// The JPEG's quality: the one setting, fixed.
const QUALITY: u8 = 85;

/// The folder the avatars live in, under the uploads' folder.
const AVATARS: &str = "avatars";

/// Where an avatar is served, its file's name after it.
pub const PREFIX: &str = "/uploads/avatars/";

/// What an upload that isn't taken is told.
pub const NOT_A_PICTURE: &str =
    "That isn't a picture we can use. Send a JPEG, PNG or WebP photo, at most 6000 pixels a side.";

/// The avatars' folder, made when missing: at boot, so a server that may
/// not write there says so at once.
///
/// # Errors
/// The folder can't be made.
pub fn avatars_folder(settings: &UploadsSettings) -> Result<PathBuf> {
    let folder = settings.path.join(AVATARS);
    std::fs::create_dir_all(&folder).map_err(|e| {
        Error::Message(format!(
            "the uploads' folder `{}` can't be made (settings.uploads.path): {e}",
            folder.display()
        ))
    })?;
    Ok(folder)
}

/// `upload` made into an avatar: decoded, upright, its middle square at
/// [`AVATAR_SIDE`], a JPEG on white. Slow work: run it off the async
/// threads (`spawn_blocking`).
///
/// # Errors
/// [`NOT_A_PICTURE`]: not a JPEG, PNG or WebP that decodes within
/// [`MAX_SIDE`].
pub fn avatar(upload: &[u8]) -> std::result::Result<Vec<u8>, &'static str> {
    let mut reader = ImageReader::new(Cursor::new(upload))
        .with_guessed_format()
        .map_err(|_| NOT_A_PICTURE)?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| NOT_A_PICTURE)?;
    let orientation = decoder.orientation().map_err(|_| NOT_A_PICTURE)?;
    let mut picture = DynamicImage::from_decoder(decoder).map_err(|_| NOT_A_PICTURE)?;
    picture.apply_orientation(orientation);

    let side = picture.width().min(picture.height());
    let square = picture.crop_imm(
        (picture.width() - side) / 2,
        (picture.height() - side) / 2,
        side,
        side,
    );
    let length = AVATAR_SIDE.min(side);
    let small = square
        .resize_exact(length, length, FilterType::Lanczos3)
        .to_rgba8();
    // On white: a PNG's transparent parts would turn black in a JPEG.
    let on_white = RgbImage::from_fn(length, length, |x, y| {
        let [r, g, b, a] = small.get_pixel(x, y).0;
        let over = |channel: u8| {
            let mixed = (u16::from(channel) * u16::from(a) + 255 * (255 - u16::from(a))) / 255;
            u8::try_from(mixed).unwrap_or(u8::MAX)
        };
        Rgb([over(r), over(g), over(b)])
    });
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, QUALITY)
        .encode_image(&on_white)
        .map_err(|_| NOT_A_PICTURE)?;
    Ok(jpeg)
}

/// A file's name from its contents: their SHA-256 as hex, the JPEG's
/// ending. The same picture is one file, whoever sent it.
#[must_use]
pub fn name_of(jpeg: &[u8]) -> String {
    format!("{:x}.jpg", Sha256::digest(jpeg))
}

/// Whether `name` is one [`name_of`] makes: the only names served, so no
/// address reaches outside the folder.
#[must_use]
pub fn is_name(name: &str) -> bool {
    name.strip_suffix(".jpg").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

/// `jpeg` kept in `folder` by its name, unless it is there already: its
/// name.
///
/// # Errors
/// The file can't be written.
pub async fn keep(folder: &Path, jpeg: &[u8]) -> std::io::Result<String> {
    let name = name_of(jpeg);
    let path = folder.join(&name);
    if !tokio::fs::try_exists(&path).await? {
        tokio::fs::write(&path, jpeg).await?;
    }
    Ok(name)
}

/// The file `name` taken away from `folder`, gone already or not.
///
/// # Errors
/// The file is there and can't be removed.
pub async fn forget(folder: &Path, name: &str) -> std::io::Result<()> {
    if !is_name(name) {
        return Ok(());
    }
    match tokio::fs::remove_file(folder.join(name)).await {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        DynamicImage::new_rgba8(width, height)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("a PNG");
        bytes
    }

    #[test]
    fn a_picture_becomes_a_square_jpeg_at_the_avatars_side() {
        let jpeg = avatar(&png(800, 600)).expect("taken");
        let made = image::load_from_memory(&jpeg).expect("a picture");
        assert_eq!((made.width(), made.height()), (AVATAR_SIDE, AVATAR_SIDE));
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF], "a JPEG");
    }

    #[test]
    fn a_small_picture_is_never_enlarged() {
        let jpeg = avatar(&png(100, 120)).expect("taken");
        let made = image::load_from_memory(&jpeg).expect("a picture");
        assert_eq!((made.width(), made.height()), (100, 100));
    }

    #[test]
    fn what_isnt_a_picture_or_is_too_large_is_refused() {
        assert_eq!(avatar(b"not a picture"), Err(NOT_A_PICTURE));
        assert_eq!(avatar(&png(MAX_SIDE + 1, 10)), Err(NOT_A_PICTURE));
    }

    #[test]
    fn a_name_is_the_hash_and_nothing_else() {
        let name = name_of(b"x");
        assert!(is_name(&name), "{name}");
        for wrong in ["../secret.jpg", "abc.jpg", &name.replace(".jpg", ".png")] {
            assert!(!is_name(wrong), "{wrong}");
        }
    }
}

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
//!   JPEG on white ([`avatar`]). A picture that would take more than
//!   [`MAX_DECODED`] decoded is refused from its header. The work runs off
//!   the async threads, [`AT_ONCE`] uploads at most, the rest waiting
//!   before their bodies are read ([`Place`]).
//! - The file is named by its contents' hash ([`name_of`]), so a new avatar
//!   has a new address and every file can be cached for a year
//!   ([`PREFIX`], controllers/uploads.rs).

use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use image::{
    DynamicImage, ImageDecoder, ImageReader, Limits, Rgb, RgbImage, codecs::jpeg::JpegEncoder,
    imageops::FilterType,
};
use loco_rs::{Error, Result};
use sha2::{Digest, Sha256};
use tokio::sync::{Semaphore, SemaphorePermit};

use crate::settings::UploadsSettings;

/// The largest upload taken, in bytes: the upload's body limit too.
pub const MAX_BYTES: usize = 10_000_000;

/// The most pixels a side of the upload may have.
pub const MAX_SIDE: u32 = 6_000;

/// The most a picture may take decoded, in bytes, checked before a pixel
/// is read: an 8-bit one with transparency at [`MAX_SIDE`] a side, about
/// 144 MB. `image` holds only the sides to the limits, never the bytes a
/// 16-bit picture takes, twice that.
pub const MAX_DECODED: u64 = MAX_SIDE as u64 * MAX_SIDE as u64 * 4;

/// How many uploads are read and made at once: the rest wait for a place
/// before their bodies are read.
pub const AT_ONCE: usize = 4;

/// The places [`AT_ONCE`] gives.
static PLACES: Semaphore = Semaphore::const_new(AT_ONCE);

/// What a picture too large to decode is told: a 16-bit one near the
/// largest sides.
pub const TOO_DEEP: &str =
    "That picture is too large to work on. Save it as a JPEG and send it again.";

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

/// The writes so far, each file's own number while it is written ([`keep`]).
static PARTS: AtomicU64 = AtomicU64::new(0);

/// What an upload over [`MAX_BYTES`] is told: one the browser couldn't
/// shrink, or sent where no WASM runs.
pub const TOO_LARGE: &str = "That picture is over 10 MB. Send a smaller one.";

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
/// [`AVATAR_SIDE`], a JPEG on white. Slow work: run it through
/// [`make_avatar`].
///
/// # Errors
/// [`NOT_A_PICTURE`]: not a JPEG, PNG or WebP that decodes within
/// [`MAX_SIDE`]. [`TOO_DEEP`]: more than [`MAX_DECODED`] to decode.
pub fn avatar(upload: &[u8]) -> std::result::Result<Vec<u8>, &'static str> {
    let mut reader = ImageReader::new(Cursor::new(upload))
        .with_guessed_format()
        .map_err(|_| NOT_A_PICTURE)?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| NOT_A_PICTURE)?;
    // The bytes it takes decoded, held to the most before a pixel is read.
    if decoder.total_bytes() > MAX_DECODED {
        return Err(TOO_DEEP);
    }
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

/// A place among the uploads at once ([`AT_ONCE`]), given back when
/// dropped.
#[derive(Debug)]
pub struct Place {
    _permit: SemaphorePermit<'static>,
}

impl Place {
    /// The next free place, waited for.
    ///
    /// # Errors
    /// The places were closed, which they never are.
    pub async fn wait() -> Result<Self> {
        let permit = PLACES
            .acquire()
            .await
            .map_err(|e| Error::Message(format!("uploads: {e}")))?;
        Ok(Self { _permit: permit })
    }
}

/// [`avatar`] on a blocking thread, the upload's [`Place`] going with its
/// bytes: given back when the work ends, not when the request waiting for
/// it does.
///
/// # Errors
/// The blocking task was lost; a refusal is the inner `Err`.
pub async fn make_avatar(
    place: Place,
    upload: Vec<u8>,
) -> Result<std::result::Result<Vec<u8>, &'static str>> {
    tokio::task::spawn_blocking(move || {
        let made = avatar(&upload);
        drop((upload, place));
        made
    })
    .await
    .map_err(Error::wrap)
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
/// name. Written under a name of its own first, then renamed, so a file by
/// its hash is always whole: one cut short by a full disk or a crash would
/// otherwise stay, found there the next time, and be cached for a year.
/// Each write's own name ([`PARTS`]), so two sending the same picture at
/// once never write into one file.
///
/// # Errors
/// The file can't be written.
pub async fn keep(folder: &Path, jpeg: &[u8]) -> std::io::Result<String> {
    let name = name_of(jpeg);
    let path = folder.join(&name);
    if !tokio::fs::try_exists(&path).await? {
        let number = PARTS.fetch_add(1, Ordering::Relaxed);
        let part = folder.join(format!(".{name}.{number}.part"));
        tokio::fs::write(&part, jpeg).await?;
        tokio::fs::rename(&part, &path).await?;
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

    /// A PNG's signature and header chunk alone, `side` square, RGBA at
    /// `bits` a channel: what the decoder reads before a pixel.
    fn png_header(side: u32, bits: u8) -> Vec<u8> {
        let crc = |bytes: &[u8]| {
            let mut crc = 0xFFFF_FFFF_u32;
            for byte in bytes {
                crc ^= u32::from(*byte);
                for _ in 0..8 {
                    crc = if crc & 1 == 1 {
                        (crc >> 1) ^ 0xEDB8_8320
                    } else {
                        crc >> 1
                    };
                }
            }
            !crc
        };
        let mut chunk = b"IHDR".to_vec();
        chunk.extend(side.to_be_bytes());
        chunk.extend(side.to_be_bytes());
        // Bit depth, RGBA, deflate, adaptive filters, no interlace.
        chunk.extend([bits, 6, 0, 0, 0]);
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(13_u32.to_be_bytes());
        png.extend(&chunk);
        png.extend(crc(&chunk).to_be_bytes());
        // An empty image data chunk: the decoder reads the header up to it.
        png.extend(0_u32.to_be_bytes());
        png.extend(b"IDAT");
        png.extend(crc(b"IDAT").to_be_bytes());
        png
    }

    /// A 16-bit picture at the largest sides takes 288 MB decoded and is
    /// refused from its header; the same at 8 bits passes the check.
    #[test]
    fn a_sixteen_bit_picture_past_the_most_is_refused_before_its_pixels() {
        assert_eq!(avatar(&png_header(MAX_SIDE, 16)), Err(TOO_DEEP));
        // 8 bits: past the check, its missing pixels then unreadable.
        assert_eq!(avatar(&png_header(MAX_SIDE, 8)), Err(NOT_A_PICTURE));
    }

    /// An upload's place is given back with its bytes, once the work ends.
    #[tokio::test]
    async fn a_place_goes_with_its_bytes() {
        let place = Place::wait().await.expect("a place");
        assert_eq!(PLACES.available_permits(), AT_ONCE - 1);
        let made = make_avatar(place, png(10, 10)).await.expect("the task");
        assert!(made.is_ok(), "{made:?}");
        assert_eq!(PLACES.available_permits(), AT_ONCE);
    }

    /// The refusals name the limits they say.
    #[test]
    fn the_refusals_name_the_limits() {
        assert!(TOO_LARGE.contains(&format!("{} MB", MAX_BYTES / 1_000_000)));
        assert!(NOT_A_PICTURE.contains(&format!("{MAX_SIDE} pixels")));
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

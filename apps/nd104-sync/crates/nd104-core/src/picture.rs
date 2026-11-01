//! Own picture/GIF for the screen: checks a file and converts it.
//!
//! Checks before anything is sent (limits from the model's [`PictureSpec`],
//! the same as the vendor's web driver):
//! - file type PNG, JPEG, GIF or BMP, file size
//! - number of GIF frames
//! - size after conversion
//!
//! Any resolution is accepted: the picture is scaled to the screen size,
//! either filling it (edges cut off) or fitting it (black bars). Transparent
//! areas become black. Frames are sent as baseline JPEG.

use std::io::Cursor;

use image::codecs::gif::GifDecoder;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{AnimationDecoder, ImageFormat, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::model::{EncodedFrame, PictureSpec};

/// How a picture with another aspect ratio is fitted to the screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Fill the screen, cut off what sticks out.
    #[default]
    Fill,
    /// Show everything, black bars where needed.
    Contain,
}

/// Why a file cannot be used. Shown to the user (translated by `code`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum PictureProblem {
    FileTooLarge { bytes: usize, max: usize },
    UnsupportedFormat,
    Damaged { detail: String },
    TooManyFrames { frames: usize, max: usize },
    TooLargeAfterConversion { bytes: usize, max: usize },
}

impl std::fmt::Display for PictureProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileTooLarge { bytes, max } => {
                write!(
                    f,
                    "file has {} KB, at most {} KB allowed",
                    bytes / 1024,
                    max / 1024
                )
            }
            Self::UnsupportedFormat => write!(f, "only PNG, JPEG, GIF and BMP are supported"),
            Self::Damaged { detail } => write!(f, "cannot read the picture: {detail}"),
            Self::TooManyFrames { frames, max } => {
                write!(f, "GIF has {frames} frames, at most {max} allowed")
            }
            Self::TooLargeAfterConversion { bytes, max } => write!(
                f,
                "{} KB after conversion, at most {} KB allowed",
                bytes / 1024,
                max / 1024
            ),
        }
    }
}

impl std::error::Error for PictureProblem {}

/// Hints about the conversion (no reason to refuse the file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum PictureNote {
    /// Scaled from another resolution.
    Scaled {
        width: u32,
        height: u32,
    },
    /// Smaller than the screen: enlarged, may look blurry.
    Enlarged,
    /// Other aspect ratio: edges cut off (fill) or black bars (contain).
    Cropped,
    Bars,
    /// Transparent areas shown black.
    Transparency,
    /// Frames shorter than 10 ms or longer than 2.55 s were adjusted.
    DelayAdjusted,
    /// GIF frames compressed more to fit the size limit.
    ReducedQuality,
}

/// A checked and converted picture, ready to send.
#[derive(Debug, Clone)]
pub struct PreparedPicture {
    pub format: &'static str,
    pub source_width: u32,
    pub source_height: u32,
    pub file_bytes: usize,
    pub frames: Vec<EncodedFrame>,
    /// Bytes sent to the screen (header + frames).
    pub payload_bytes: usize,
    pub notes: Vec<PictureNote>,
}

const STILL_QUALITY: u8 = 92;
const GIF_QUALITIES: [u8; 4] = [85, 70, 55, 40];

/// Checks and converts a picture file.
pub fn prepare(
    bytes: &[u8],
    spec: &PictureSpec,
    fit: Fit,
) -> Result<PreparedPicture, PictureProblem> {
    if bytes.len() > spec.max_file_bytes {
        return Err(PictureProblem::FileTooLarge {
            bytes: bytes.len(),
            max: spec.max_file_bytes,
        });
    }
    let format = match image::guess_format(bytes) {
        Ok(f @ (ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::Bmp)) => f,
        _ => return Err(PictureProblem::UnsupportedFormat),
    };
    let damaged = |e: image::ImageError| PictureProblem::Damaged {
        detail: e.to_string(),
    };

    let mut notes = Vec::new();
    // (image, delay in 10 ms units)
    let source: Vec<(RgbaImage, u8)> = if format == ImageFormat::Gif {
        let decoder = GifDecoder::new(Cursor::new(bytes)).map_err(damaged)?;
        let mut frames = Vec::new();
        for frame in decoder.into_frames() {
            let frame = frame.map_err(damaged)?;
            if frames.len() == spec.max_frames {
                // count the rest without keeping them
                let total = frames.len() + 1;
                return Err(PictureProblem::TooManyFrames {
                    frames: total + count_remaining(bytes, total),
                    max: spec.max_frames,
                });
            }
            let (num, den) = frame.delay().numer_denom_ms();
            let ms = num.checked_div(den).unwrap_or(0);
            // browsers show 0/very short delays as 100 ms
            let units = if ms < 20 { 10 } else { ms.div_ceil(10) };
            if !(1..=255).contains(&units) || ms < 20 {
                notes.push(PictureNote::DelayAdjusted);
            }
            frames.push((frame.into_buffer(), units.clamp(1, 255) as u8));
        }
        if frames.is_empty() {
            return Err(PictureProblem::Damaged {
                detail: "GIF without frames".into(),
            });
        }
        frames
    } else {
        let image = image::load_from_memory_with_format(bytes, format).map_err(damaged)?;
        vec![(image.to_rgba8(), 1)]
    };

    let (sw, sh) = source[0].0.dimensions();
    if (sw, sh) != (spec.width, spec.height) {
        notes.push(PictureNote::Scaled {
            width: sw,
            height: sh,
        });
        if sw < spec.width && sh < spec.height {
            notes.push(PictureNote::Enlarged);
        }
        // aspect ratio differs by more than 1 %
        if (u64::from(sw) * u64::from(spec.height)).abs_diff(u64::from(sh) * u64::from(spec.width))
            > u64::from(sw) * u64::from(spec.height) / 100
        {
            notes.push(match fit {
                Fit::Fill => PictureNote::Cropped,
                Fit::Contain => PictureNote::Bars,
            });
        }
    }
    if source
        .iter()
        .any(|(img, _)| img.pixels().any(|p| p.0[3] < 255))
    {
        notes.push(PictureNote::Transparency);
    }
    notes.dedup();

    let scaled: Vec<(RgbaImage, u8)> = source
        .into_iter()
        .map(|(img, delay)| (scale(&img, spec.width, spec.height, fit), delay))
        .collect();

    let qualities: &[u8] = if scaled.len() == 1 {
        &[STILL_QUALITY, 80, 65]
    } else {
        &GIF_QUALITIES
    };
    let mut last_size = 0;
    for (i, &quality) in qualities.iter().enumerate() {
        let frames = scaled
            .iter()
            .map(|(img, delay)| {
                Ok(EncodedFrame {
                    jpeg: encode_jpeg(img, quality)?,
                    delay: *delay,
                })
            })
            .collect::<Result<Vec<_>, PictureProblem>>()?;
        let payload_bytes = payload_size(&frames);
        last_size = payload_bytes;
        if payload_bytes <= spec.max_bytes {
            if i > 0 {
                notes.push(PictureNote::ReducedQuality);
            }
            return Ok(PreparedPicture {
                format: match format {
                    ImageFormat::Png => "PNG",
                    ImageFormat::Jpeg => "JPEG",
                    ImageFormat::Gif => "GIF",
                    _ => "BMP",
                },
                source_width: sw,
                source_height: sh,
                file_bytes: bytes.len(),
                frames,
                payload_bytes,
                notes,
            });
        }
    }
    Err(PictureProblem::TooLargeAfterConversion {
        bytes: last_size,
        max: spec.max_bytes,
    })
}

fn count_remaining(bytes: &[u8], skip: usize) -> usize {
    GifDecoder::new(Cursor::new(bytes))
        .map(|d| d.into_frames().skip(skip).take_while(Result::is_ok).count())
        .unwrap_or(0)
}

/// Same layout as `nd104_protocol::picture_payload`: 8 header bytes, one
/// delay byte per frame, four size bytes per frame, then the frames.
fn payload_size(frames: &[EncodedFrame]) -> usize {
    8 + frames.len() * 5 + frames.iter().map(|f| f.jpeg.len()).sum::<usize>()
}

fn scale(img: &RgbaImage, width: u32, height: u32, fit: Fit) -> RgbaImage {
    let (sw, sh) = img.dimensions();
    let fx = f64::from(width) / f64::from(sw);
    let fy = f64::from(height) / f64::from(sh);
    let factor = match fit {
        Fit::Fill => fx.max(fy),
        Fit::Contain => fx.min(fy),
    };
    let w = ((f64::from(sw) * factor).round() as u32).max(1);
    let h = ((f64::from(sh) * factor).round() as u32).max(1);
    let resized = if (w, h) == (sw, sh) {
        img.clone()
    } else {
        imageops::resize(img, w, h, FilterType::CatmullRom)
    };
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 255]));
    let x = (i64::from(width) - i64::from(w)) / 2;
    let y = (i64::from(height) - i64::from(h)) / 2;
    imageops::overlay(&mut canvas, &resized, x, y);
    canvas
}

fn encode_jpeg(img: &RgbaImage, quality: u8) -> Result<Vec<u8>, PictureProblem> {
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality)
        .encode_image(&rgb)
        .map_err(|e| PictureProblem::Damaged {
            detail: e.to_string(),
        })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};

    const SPEC: PictureSpec = PictureSpec {
        width: 320,
        height: 172,
        max_frames: 3,
        max_bytes: 200_000,
        max_file_bytes: 300_000,
    };

    fn png(w: u32, h: u32, alpha: u8) -> Vec<u8> {
        let img = RgbaImage::from_pixel(w, h, Rgba([200, 30, 30, alpha]));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    fn gif(frames: usize, delay_ms: u32) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut out);
            enc.set_repeat(Repeat::Infinite).unwrap();
            for i in 0..frames {
                let img = RgbaImage::from_pixel(64, 34, Rgba([(i * 40) as u8, 100, 200, 255]));
                enc.encode_frame(Frame::from_parts(
                    img,
                    0,
                    0,
                    Delay::from_numer_denom_ms(delay_ms, 1),
                ))
                .unwrap();
            }
        }
        out
    }

    #[test]
    fn exact_size_png_needs_no_scaling() {
        let p = prepare(&png(320, 172, 255), &SPEC, Fit::Fill).unwrap();
        assert_eq!(p.frames.len(), 1);
        assert_eq!(p.frames[0].delay, 1);
        assert!(p.notes.is_empty(), "{:?}", p.notes);
        assert_eq!(&p.frames[0].jpeg[..2], &[0xFF, 0xD8]);
        let decoded = image::load_from_memory(&p.frames[0].jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (320, 172));
        assert_eq!(p.payload_bytes, 8 + 5 + p.frames[0].jpeg.len());
    }

    #[test]
    fn other_sizes_are_scaled_with_notes() {
        let p = prepare(&png(100, 100, 128), &SPEC, Fit::Fill).unwrap();
        assert!(p.notes.contains(&PictureNote::Scaled {
            width: 100,
            height: 100
        }));
        assert!(p.notes.contains(&PictureNote::Enlarged));
        assert!(p.notes.contains(&PictureNote::Cropped));
        assert!(p.notes.contains(&PictureNote::Transparency));
        let p = prepare(&png(640, 344, 255), &SPEC, Fit::Contain).unwrap();
        assert_eq!(
            p.notes,
            vec![PictureNote::Scaled {
                width: 640,
                height: 344
            }]
        );
        let p = prepare(&png(400, 400, 255), &SPEC, Fit::Contain).unwrap();
        assert!(p.notes.contains(&PictureNote::Bars));
    }

    #[test]
    fn gif_frames_and_delays() {
        let p = prepare(&gif(3, 120), &SPEC, Fit::Fill).unwrap();
        assert_eq!(p.frames.len(), 3);
        assert!(p.frames.iter().all(|f| f.delay == 12));
        let p = prepare(&gif(2, 0), &SPEC, Fit::Fill).unwrap();
        assert!(p.frames.iter().all(|f| f.delay == 10));
        assert!(p.notes.contains(&PictureNote::DelayAdjusted));
    }

    #[test]
    fn refuses_what_breaks_the_limits() {
        assert_eq!(
            prepare(&gif(5, 100), &SPEC, Fit::Fill).unwrap_err(),
            PictureProblem::TooManyFrames { frames: 5, max: 3 }
        );
        assert_eq!(
            prepare(b"not a picture at all", &SPEC, Fit::Fill).unwrap_err(),
            PictureProblem::UnsupportedFormat
        );
        let big = vec![0u8; 300_001];
        assert!(matches!(
            prepare(&big, &SPEC, Fit::Fill).unwrap_err(),
            PictureProblem::FileTooLarge { .. }
        ));
        let tiny = PictureSpec {
            max_bytes: 100,
            ..SPEC
        };
        assert!(matches!(
            prepare(&png(320, 172, 255), &tiny, Fit::Fill).unwrap_err(),
            PictureProblem::TooLargeAfterConversion { .. }
        ));
        let mut broken = png(320, 172, 255);
        broken.truncate(60);
        assert!(matches!(
            prepare(&broken, &SPEC, Fit::Fill).unwrap_err(),
            PictureProblem::Damaged { .. }
        ));
    }
}

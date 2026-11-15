//! Static wallpaper loading and output placement.

use std::path::Path;

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    utils::{Buffer, Logical, Point, Rectangle, Size, Transform},
};

use crate::config::WallpaperMode;

pub struct Wallpaper {
    pub buffer: MemoryRenderBuffer,
    source_size: Size<i32, Logical>,
}

#[derive(Debug, Clone, Copy)]
pub struct WallpaperPlacement {
    pub location: Point<f64, Logical>,
    pub source: Option<Rectangle<f64, Logical>>,
    pub size: Size<i32, Logical>,
}

impl Wallpaper {
    pub fn load(path: &Path) -> Result<Self, String> {
        let decoded = image::ImageReader::open(path)
            .map_err(|error| format!("could not open wallpaper '{}': {error}", path.display()))?
            .with_guessed_format()
            .map_err(|error| format!("could not identify wallpaper '{}': {error}", path.display()))?
            .decode()
            .map_err(|error| format!("could not decode wallpaper '{}': {error}", path.display()))?;

        let rgba = decoded.into_rgba8();
        let (width, height) = rgba.dimensions();

        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(format!(
                "wallpaper '{}' has unsupported dimensions {width}x{height}",
                path.display()
            ));
        }

        let buffer_size: Size<i32, Buffer> = Size::from((width as i32, height as i32));
        let source_size: Size<i32, Logical> = Size::from((width as i32, height as i32));
        let buffer = MemoryRenderBuffer::from_slice(
            rgba.as_raw(),
            Fourcc::Abgr8888,
            buffer_size,
            1,
            Transform::Normal,
            None,
        );

        Ok(Self {
            buffer,
            source_size,
        })
    }

    pub fn placement(
        &self,
        output_size: Size<i32, Logical>,
        mode: WallpaperMode,
    ) -> WallpaperPlacement {
        placement_for(self.source_size, output_size, mode)
    }
}

fn placement_for(
    source_size: Size<i32, Logical>,
    output_size: Size<i32, Logical>,
    mode: WallpaperMode,
) -> WallpaperPlacement {
    let source_width = source_size.w as f64;
    let source_height = source_size.h as f64;
    let output_width = output_size.w.max(1) as f64;
    let output_height = output_size.h.max(1) as f64;

    match mode {
        WallpaperMode::Stretch => WallpaperPlacement {
            location: Point::from((0.0, 0.0)),
            source: None,
            size: output_size,
        },
        WallpaperMode::Fill => {
            let source_aspect = source_width / source_height;
            let output_aspect = output_width / output_height;

            let (crop_x, crop_y, crop_width, crop_height) = if source_aspect > output_aspect {
                let crop_width = source_height * output_aspect;
                (
                    (source_width - crop_width) / 2.0,
                    0.0,
                    crop_width,
                    source_height,
                )
            } else {
                let crop_height = source_width / output_aspect;
                (
                    0.0,
                    (source_height - crop_height) / 2.0,
                    source_width,
                    crop_height,
                )
            };

            WallpaperPlacement {
                location: Point::from((0.0, 0.0)),
                source: Some(Rectangle::new(
                    Point::from((crop_x, crop_y)),
                    Size::from((crop_width, crop_height)),
                )),
                size: output_size,
            }
        }
        WallpaperMode::Fit => {
            let factor = (output_width / source_width).min(output_height / source_height);
            let width = ((source_width * factor).round() as i32).clamp(1, output_size.w.max(1));
            let height = ((source_height * factor).round() as i32).clamp(1, output_size.h.max(1));

            WallpaperPlacement {
                location: Point::from((
                    (output_width - width as f64) / 2.0,
                    (output_height - height as f64) / 2.0,
                )),
                source: None,
                size: Size::from((width, height)),
            }
        }
        WallpaperMode::Center => WallpaperPlacement {
            location: Point::from((
                (output_width - source_width) / 2.0,
                (output_height - source_height) / 2.0,
            )),
            source: None,
            size: source_size,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_crops_the_wider_dimension() {
        let placement = placement_for(
            Size::from((1920, 1080)),
            Size::from((1000, 1000)),
            WallpaperMode::Fill,
        );

        let source = placement.source.unwrap();
        assert!((source.size.w - 1080.0).abs() < 0.01);
        assert!((source.loc.x - 420.0).abs() < 0.01);
        assert_eq!(placement.size, Size::from((1000, 1000)));
    }

    #[test]
    fn fit_preserves_aspect_ratio_and_centers() {
        let placement = placement_for(
            Size::from((1920, 1080)),
            Size::from((1000, 1000)),
            WallpaperMode::Fit,
        );

        assert_eq!(placement.size, Size::from((1000, 563)));
        assert_eq!(placement.location.x, 0.0);
        assert!(placement.location.y > 0.0);
        assert!(placement.source.is_none());
    }

    #[test]
    fn stretch_uses_the_full_output() {
        let placement = placement_for(
            Size::from((1920, 1080)),
            Size::from((1280, 720)),
            WallpaperMode::Stretch,
        );

        assert_eq!(placement.size, Size::from((1280, 720)));
        assert_eq!(placement.location, Point::from((0.0, 0.0)));
    }

    #[test]
    fn center_keeps_the_original_image_size() {
        let placement = placement_for(
            Size::from((800, 600)),
            Size::from((1920, 1080)),
            WallpaperMode::Center,
        );

        assert_eq!(placement.size, Size::from((800, 600)));
        assert_eq!(placement.location, Point::from((560.0, 240.0)));
    }
}

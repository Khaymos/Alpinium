//! Cursor theme loading and the compositor-provided fallback cursor.

use std::{collections::HashMap, env, fs};

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    input::pointer::CursorIcon,
    utils::{Logical, Point, Size, Transform},
};

pub struct CursorAsset {
    pub buffer: MemoryRenderBuffer,
    pub size: Size<i32, Logical>,
    pub hotspot: Point<i32, Logical>,
}

pub struct CursorManager {
    theme: xcursor::CursorTheme,
    target_size: u32,
    cache: HashMap<String, Option<CursorAsset>>,
    active: Option<String>,
    fallback: CursorAsset,
}

impl CursorManager {
    pub fn new() -> Self {
        let theme_name = env::var("XCURSOR_THEME")
            .ok()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "default".to_string());

        let target_size = env::var("XCURSOR_SIZE")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|size| *size > 0)
            .unwrap_or(24)
            .clamp(8, 128);

        let mut manager = Self {
            theme: xcursor::CursorTheme::load(&theme_name),
            target_size,
            cache: HashMap::new(),
            active: None,
            fallback: fallback_arrow(),
        };
        manager.set_named(CursorIcon::Default);
        manager
    }

    pub fn set_named(&mut self, icon: CursorIcon) {
        let key = icon.name().to_string();

        if !self.cache.contains_key(&key) {
            let mut names = vec![icon.name()];
            names.extend_from_slice(icon.alt_names());

            if icon.name() == "default" {
                names.extend(["left_ptr", "arrow"]);
            }

            let asset = names
                .into_iter()
                .find_map(|name| load_theme_cursor(&self.theme, name, self.target_size));

            self.cache.insert(key.clone(), asset);
        }

        self.active = Some(key);
    }

    pub fn current(&self) -> &CursorAsset {
        self.active
            .as_ref()
            .and_then(|key| self.cache.get(key))
            .and_then(Option::as_ref)
            .unwrap_or(&self.fallback)
    }
}

fn load_theme_cursor(
    theme: &xcursor::CursorTheme,
    name: &str,
    target_size: u32,
) -> Option<CursorAsset> {
    let path = theme.load_icon(name)?;
    let contents = fs::read(path).ok()?;
    let images = xcursor::parser::parse_xcursor(&contents)?;

    let image = images
        .into_iter()
        .filter(|image| image.width > 0 && image.height > 0)
        .min_by_key(|image| image.size.abs_diff(target_size))?;

    let width = i32::try_from(image.width).ok()?;
    let height = i32::try_from(image.height).ok()?;
    let expected_bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;

    if image.pixels_rgba.len() < expected_bytes {
        return None;
    }

    let buffer = MemoryRenderBuffer::from_slice(
        &image.pixels_rgba,
        Fourcc::Abgr8888,
        (width, height),
        1,
        Transform::Normal,
        None,
    );

    Some(CursorAsset {
        buffer,
        size: Size::from((width, height)),
        hotspot: Point::from((
            image.xhot.min(image.width - 1) as i32,
            image.yhot.min(image.height - 1) as i32,
        )),
    })
}

fn fallback_arrow() -> CursorAsset {
    const WIDTH: usize = 24;
    const HEIGHT: usize = 24;

    const OUTER: [(f64, f64); 7] = [
        (1.0, 1.0),
        (1.0, 21.0),
        (6.0, 16.0),
        (10.0, 23.0),
        (14.0, 21.0),
        (10.0, 14.0),
        (18.0, 14.0),
    ];
    const INNER: [(f64, f64); 7] = [
        (3.0, 5.0),
        (3.0, 16.0),
        (6.0, 13.0),
        (11.0, 20.0),
        (12.0, 19.0),
        (8.0, 12.0),
        (15.0, 12.0),
    ];

    fn inside(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
        let mut result = false;
        let mut previous = polygon.len() - 1;

        for current in 0..polygon.len() {
            let (xi, yi) = polygon[current];
            let (xj, yj) = polygon[previous];

            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                result = !result;
            }

            previous = current;
        }

        result
    }

    let mut pixels = vec![0u8; WIDTH * HEIGHT * 4];

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let px = x as f64 + 0.5;
            let py = y as f64 + 0.5;
            let color = if inside(px, py, &INNER) {
                Some([255, 255, 255, 255])
            } else if inside(px, py, &OUTER) {
                Some([0, 0, 0, 255])
            } else {
                None
            };

            if let Some(color) = color {
                let offset = (y * WIDTH + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }

    CursorAsset {
        buffer: MemoryRenderBuffer::from_slice(
            &pixels,
            Fourcc::Abgr8888,
            (WIDTH as i32, HEIGHT as i32),
            1,
            Transform::Normal,
            None,
        ),
        size: Size::from((WIDTH as i32, HEIGHT as i32)),
        hotspot: Point::from((1, 1)),
    }
}

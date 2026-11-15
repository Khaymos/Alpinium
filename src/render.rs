//! Shared render-element construction for animated windows and cursors.

use std::time::Instant;

use smithay::{
    backend::renderer::{
        ImportAll, ImportDmaWl, ImportMem, ImportMemWl, Renderer,
        element::{
            AsRenderElements, Kind, Wrap,
            memory::MemoryRenderBufferRenderElement,
            surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
            utils::{
                ConstrainAlign, ConstrainScaleBehavior, CropRenderElement, RelocateRenderElement,
                RescaleRenderElement, constrain_render_elements,
            },
        },
    },
    desktop::space::SpaceRenderElements,
    input::pointer::CursorImageStatus,
    output::Output,
    utils::{Logical, Physical, Point, Rectangle, Scale, Size},
};

use crate::nested::AlpiniumState;

#[allow(type_alias_bounds)]
pub type AnimatedSpaceRenderElements<R>
where
    R: ImportAll + ImportMemWl + ImportDmaWl,
= SpaceRenderElements<
    R,
    CropRenderElement<RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>>,
>;

smithay::backend::renderer::element::render_elements! {
    pub AlpiniumRenderElements<R, WindowElement, CursorElement> where
        R: ImportAll + ImportMem + ImportMemWl + ImportDmaWl;
    Window=WindowElement,
    Cursor=CursorElement,
}

/// Build window and cursor render elements. Cursor elements are placed first
/// because the renderer consumes elements in front-to-back order.
pub fn space_render_elements<R>(
    state: &AlpiniumState,
    renderer: &mut R,
    output: &Output,
) -> Vec<
    AlpiniumRenderElements<R, AnimatedSpaceRenderElements<R>, MemoryRenderBufferRenderElement<R>>,
>
where
    R: Renderer + ImportAll + ImportMem + ImportMemWl + ImportDmaWl,
    R::TextureId: Send + Clone + 'static,
{
    let output_scale = output.current_scale().fractional_scale();
    let scale = Scale::from(output_scale);
    let now = Instant::now();
    let mut elements = Vec::new();

    // Space iterators are back-to-front; Smithay's renderer expects front-to-back.
    for window in state.space.elements_for_output(output).rev() {
        let Some(mapped_location) = state.space.element_location(window) else {
            continue;
        };

        let geometry = window.geometry();
        let animated = state.animation_geometry(window, now);
        let base_location = animated
            .as_ref()
            .map(|rect| {
                Point::from((
                    rect.x.round() as i32 - geometry.loc.x,
                    rect.y.round() as i32 - geometry.loc.y,
                ))
            })
            .unwrap_or_else(|| mapped_location - geometry.loc);
        let base_physical = base_location.to_physical_precise_round(output_scale);
        let surfaces = window.render_elements::<WaylandSurfaceRenderElement<R>>(
            renderer,
            base_physical,
            scale,
            1.0,
        );

        if let Some(animated) = animated {
            let constrain_location: Point<i32, Logical> =
                Point::from((animated.x.round() as i32, animated.y.round() as i32));
            let constrain_size: Size<i32, Logical> = (
                animated.width.round() as i32,
                animated.height.round() as i32,
            )
                .into();
            let constrain = Rectangle::new(
                constrain_location.to_physical_precise_round(output_scale),
                constrain_size.to_physical_precise_round(output_scale),
            );
            let reference = geometry.to_physical_precise_round(output_scale);

            for cropped in constrain_render_elements(
                surfaces,
                base_physical,
                constrain,
                reference,
                ConstrainScaleBehavior::Stretch,
                ConstrainAlign::CENTER,
                output_scale,
            ) {
                elements.push(AlpiniumRenderElements::Window(
                    SpaceRenderElements::Element(Wrap::from(cropped)),
                ));
            }
        } else {
            elements.extend(surfaces.into_iter().map(|surface| {
                AlpiniumRenderElements::Window(SpaceRenderElements::Surface(surface))
            }));
        }
    }

    let pointer_location = state
        .seat
        .get_pointer()
        .map(|pointer| pointer.current_location());
    let output_geometry = state.space.output_geometry(output);

    if let (Some(pointer_location), Some(output_geometry)) = (pointer_location, output_geometry) {
        match &state.cursor_image {
            CursorImageStatus::Hidden => {}
            CursorImageStatus::Surface(surface) => {
                let hotspot = smithay::wayland::compositor::with_states(surface, |surface_state| {
                    surface_state
                        .data_map
                        .get::<smithay::input::pointer::CursorImageSurfaceData>()
                        .map(|data| data.lock().unwrap().hotspot)
                        .unwrap_or_default()
                });

                let cursor_location: Point<f64, Logical> =
                    pointer_location - hotspot.to_f64() - output_geometry.loc.to_f64();
                let cursor_elements =
                    render_elements_from_surface_tree::<R, WaylandSurfaceRenderElement<R>>(
                        renderer,
                        surface,
                        cursor_location.to_physical_precise_round(output_scale),
                        scale,
                        1.0,
                        Kind::Cursor,
                    );

                let cursor_elements = cursor_elements
                    .into_iter()
                    .map(|cursor| {
                        AlpiniumRenderElements::Window(SpaceRenderElements::Surface(cursor))
                    })
                    .collect::<Vec<_>>();
                elements.splice(0..0, cursor_elements);
            }
            CursorImageStatus::Named(_) => {
                let cursor = state.cursor_manager.current();
                let cursor_location: Point<f64, Logical> =
                    pointer_location - cursor.hotspot.to_f64() - output_geometry.loc.to_f64();

                if let Ok(cursor_element) = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    {
                        let physical_location: Point<i32, Physical> =
                            cursor_location.to_physical_precise_round(output_scale);
                        physical_location.to_f64()
                    },
                    &cursor.buffer,
                    None,
                    None,
                    Some(cursor.size),
                    Kind::Cursor,
                ) {
                    elements.insert(0, AlpiniumRenderElements::Cursor(cursor_element));
                }
            }
        }
    }

    elements
}

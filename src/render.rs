//! Shared render-element construction for animated windows.

use std::time::Instant;

use smithay::{
    backend::renderer::{
        ImportAll, Renderer,
        element::{
            AsRenderElements, Wrap,
            surface::WaylandSurfaceRenderElement,
            utils::{
                ConstrainAlign, ConstrainScaleBehavior, CropRenderElement, RelocateRenderElement,
                RescaleRenderElement, constrain_render_elements,
            },
        },
    },
    desktop::space::SpaceRenderElements,
    output::Output,
    utils::{Logical, Point, Rectangle, Scale, Size},
};

use crate::nested::AlpiniumState;

pub type AnimatedSpaceRenderElements<R> = SpaceRenderElements<
    R,
    CropRenderElement<RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>>,
>;

/// Build window render elements with a shared geometry transition. The Space
/// mapping remains at its target location, preserving z-order; the render
/// element is relocated and rescaled for the current animation sample.
pub fn space_render_elements<R>(
    state: &AlpiniumState,
    renderer: &mut R,
    output: &Output,
) -> Vec<AnimatedSpaceRenderElements<R>>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
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
                elements.push(SpaceRenderElements::Element(Wrap::from(cropped)));
            }
        } else {
            elements.extend(surfaces.into_iter().map(SpaceRenderElements::Surface));
        }
    }

    elements
}

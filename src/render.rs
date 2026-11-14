//! Shared render-element construction for animated windows.

use std::time::Instant;

use smithay::{
    backend::renderer::{
        element::{
            AsRenderElements, Wrap,
            surface::WaylandSurfaceRenderElement,
            utils::{Relocate, RelocateRenderElement, RescaleRenderElement},
        },
        ImportAll, Renderer,
    },
    desktop::space::SpaceRenderElements,
    output::Output,
    utils::{Point, Scale},
};

use crate::nested::AlpiniumState;

pub type AnimatedSpaceRenderElements<R: Renderer + ImportAll> =
    SpaceRenderElements<
        R,
        RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>,
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
        let base_location = mapped_location - geometry.loc;
        let base_physical = base_location.to_physical_precise_round(output_scale);
        let surfaces = window.render_elements::<WaylandSurfaceRenderElement<R>>(
            renderer,
            base_physical,
            scale,
            1.0,
        );

        if let Some(animated) = state.animation_geometry(window, now) {
            let source_width = f64::from(geometry.size.w.max(1));
            let source_height = f64::from(geometry.size.h.max(1));
            let animated_scale = Scale::from((
                (animated.width / source_width).max(0.01),
                (animated.height / source_height).max(0.01),
            ));
            let animated_location = Point::from((
                animated.x.round() as i32 - geometry.loc.x,
                animated.y.round() as i32 - geometry.loc.y,
            ));
            let animated_physical =
                animated_location.to_physical_precise_round(output_scale);
            let offset = animated_physical - base_physical;

            for surface in surfaces {
                let scaled = RescaleRenderElement::from_element(
                    surface,
                    base_physical,
                    animated_scale,
                );
                let relocated = RelocateRenderElement::from_element(
                    scaled,
                    offset,
                    Relocate::Relative,
                );
                elements.push(SpaceRenderElements::Element(Wrap::from(relocated)));
            }
        } else {
            elements.extend(surfaces.into_iter().map(SpaceRenderElements::Surface));
        }
    }

    elements
}

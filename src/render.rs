//! Shared render-element construction for animated windows.

use std::time::Instant;

use smithay::{
    backend::renderer::{
        element::{
            AsRenderElements, Wrap,
            surface::WaylandSurfaceRenderElement,
            utils::RescaleRenderElement,
        },
        ImportAll, Renderer,
    },
    desktop::space::SpaceRenderElements,
    output::Output,
    utils::Scale,
};

use crate::nested::AlpiniumState;

pub type AnimatedSpaceRenderElements<R> =
    SpaceRenderElements<R, RescaleRenderElement<WaylandSurfaceRenderElement<R>>>;

/// Build window render elements, applying the current geometry animation as a
/// render-time scale. Space locations are advanced separately by the animation
/// state, so input/focus handling remains attached to the moving window.
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
        let render_location =
            (mapped_location - geometry.loc).to_physical_precise_round(output_scale);
        let surfaces = window.render_elements::<WaylandSurfaceRenderElement<R>>(
            renderer,
            render_location,
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

            for surface in surfaces {
                let scaled = RescaleRenderElement::from_element(
                    surface,
                    render_location,
                    animated_scale,
                );
                elements.push(SpaceRenderElements::Element(Wrap::from(scaled)));
            }
        } else {
            elements.extend(surfaces.into_iter().map(SpaceRenderElements::Surface));
        }
    }

    elements
}

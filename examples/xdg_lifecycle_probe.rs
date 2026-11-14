//! Small standalone XDG client that exercises map -> unmap -> remap.
//!
//! Run it against a running Alpinium nested compositor with:
//! WAYLAND_DISPLAY=wayland-N cargo run --example xdg_lifecycle_probe

use std::{
    error::Error,
    io::{self, Seek, SeekFrom, Write},
    os::fd::AsFd,
    thread,
    time::{Duration, Instant},
};

use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 100;

#[derive(Default)]
struct State {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    outputs: Vec<wl_output::WlOutput>,
    surface: Option<wl_surface::WlSurface>,
    xdg_surface: Option<xdg_surface::XdgSurface>,
    toplevel: Option<xdg_toplevel::XdgToplevel>,
    buffer: Option<wl_buffer::WlBuffer>,
    configure_count: usize,
    enter_count: usize,
    leave_count: usize,
    close_requested: bool,
}

fn protocol_error(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, message)
}

fn main() -> Result<(), Box<dyn Error>> {
    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue::<State>();
    let qh = event_queue.handle();

    let display = connection.display();
    display.get_registry(&qh, ());

    let mut state = State::default();
    event_queue.roundtrip(&mut state)?;

    let compositor = state
        .compositor
        .as_ref()
        .ok_or_else(|| protocol_error("compositor did not advertise wl_compositor"))?
        .clone();
    let shm = state
        .shm
        .as_ref()
        .ok_or_else(|| protocol_error("compositor did not advertise wl_shm"))?
        .clone();
    let wm_base = state
        .wm_base
        .as_ref()
        .ok_or_else(|| protocol_error("compositor did not advertise xdg_wm_base"))?
        .clone();

    // Create a real SHM buffer. A visible opaque color makes the test window
    // easy to distinguish while keeping the probe dependency-light.
    let byte_len = (WIDTH * HEIGHT * 4) as i32;
    let mut file = tempfile::tempfile()?;
    file.set_len(byte_len as u64)?;
    file.seek(SeekFrom::Start(0))?;
    let pixel = [0x40, 0xA0, 0xFF, 0xFF];
    for _ in 0..(WIDTH * HEIGHT) {
        file.write_all(&pixel)?;
    }

    let pool = shm.create_pool(file.as_fd(), byte_len, &qh, ());
    let buffer = pool.create_buffer(
        0,
        WIDTH as i32,
        HEIGHT as i32,
        (WIDTH * 4) as i32,
        wl_shm::Format::Argb8888,
        &qh,
        (),
    );
    pool.destroy();
    state.buffer = Some(buffer.clone());

    // Create an XDG toplevel and perform its initial bufferless commit.
    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Alpinium XDG lifecycle probe".to_string());
    surface.commit();

    state.surface = Some(surface.clone());
    state.xdg_surface = Some(xdg_surface);
    state.toplevel = Some(toplevel.clone());

    event_queue.roundtrip(&mut state)?;
    if state.configure_count == 0 {
        return Err(protocol_error("initial XDG configure was not received").into());
    }
    println!("PASS: initial XDG configure received and acknowledged");

    // Map the surface by attaching its first buffer.
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, WIDTH as i32, HEIGHT as i32);
    surface.commit();

    // Space::map_element is reconciled with outputs during the compositor's
    // redraw/Space::refresh cycle. Wait for enter before asserting a matching
    // leave after unmapping; roundtrip alone does not guarantee a redraw.
    let map_deadline = Instant::now() + Duration::from_secs(5);
    while state.enter_count == 0 && Instant::now() < map_deadline {
        event_queue.roundtrip(&mut state)?;
        if state.enter_count == 0 {
            thread::sleep(Duration::from_millis(16));
        }
    }
    if state.enter_count == 0 {
        return Err(protocol_error("surface did not receive wl_surface.enter after initial map").into());
    }
    println!("PASS: surface entered its output after initial map");

    let enters_before_unmap = state.enter_count;
    let leaves_before_unmap = state.leave_count;

    // Unmap: a null buffer attachment followed by a commit.
    surface.attach(None, 0, 0);
    surface.commit();

    let unmap_deadline = Instant::now() + Duration::from_secs(5);
    while state.leave_count <= leaves_before_unmap && Instant::now() < unmap_deadline {
        event_queue.roundtrip(&mut state)?;
        if state.leave_count <= leaves_before_unmap {
            thread::sleep(Duration::from_millis(16));
        }
    }
    if state.leave_count <= leaves_before_unmap {
        return Err(protocol_error("surface did not receive wl_surface.leave after unmap").into());
    }
    println!("PASS: buffer detach caused the surface to leave its output");

    // XDG requires a fresh bufferless commit after unmapping. The compositor
    // must then send a new configure, which this callback acknowledges.
    let configures_before_remap = state.configure_count;
    surface.commit();
    event_queue.roundtrip(&mut state)?;

    if state.configure_count <= configures_before_remap {
        return Err(protocol_error("fresh XDG configure was not sent after the post-unmap commit").into());
    }
    println!("PASS: fresh XDG configure received and acknowledged after unmap");

    // Re-map by attaching a buffer again after acknowledging the new configure.
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, WIDTH as i32, HEIGHT as i32);
    surface.commit();

    let remap_deadline = Instant::now() + Duration::from_secs(5);
    while state.enter_count <= enters_before_unmap && Instant::now() < remap_deadline {
        event_queue.roundtrip(&mut state)?;
        if state.enter_count <= enters_before_unmap {
            thread::sleep(Duration::from_millis(16));
        }
    }
    if state.enter_count <= enters_before_unmap {
        return Err(protocol_error("surface did not receive wl_surface.enter after remap").into());
    }
    if state.close_requested {
        return Err(protocol_error("compositor unexpectedly sent xdg_toplevel.close").into());
    }
    println!("PASS: surface re-entered its output after remapping");
    println!(
        "PASS: lifecycle sequence complete ({} configures, {} enters, {} leaves)",
        state.configure_count, state.enter_count, state.leave_count
    );

    // Tear down cleanly so Alpinium also receives the normal destroy callback.
    toplevel.destroy();
    state
        .xdg_surface
        .as_ref()
        .expect("XDG surface was created")
        .destroy();
    surface.destroy();
    buffer.destroy();
    wm_base.destroy();
    event_queue.roundtrip(&mut state)?;

    Ok(())
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    ));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ));
                }
                "xdg_wm_base" => {
                    state.wm_base = Some(registry.bind::<xdg_wm_base::XdgWmBase, _, _>(
                        name,
                        version.min(6),
                        qh,
                        (),
                    ));
                }
                "wl_output" => {
                    state.outputs.push(registry.bind::<wl_output::WlOutput, _, _>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    ));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
            state.configure_count += 1;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Close = event {
            state.close_requested = true;
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_surface::WlSurface,
        event: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_surface::Event::Enter { .. } => state.enter_count += 1,
            wl_surface::Event::Leave { .. } => state.leave_count += 1,
            _ => {}
        }
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);
delegate_noop!(State: ignore wl_output::WlOutput);

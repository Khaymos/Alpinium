//! Native single-GPU DRM/KMS backend and safe hardware diagnostics.
//!
//! The first runtime intentionally targets the verified internal eDP-1 panel.
//! It reuses Alpinium's Wayland state and input routing, and keeps session pause
//! and resume wired to libseat. External outputs and hotplug are future work.

use std::{
    cell::{Cell, RefCell},
    error::Error,
    ffi::OsString,
    os::unix::fs::MetadataExt,
    path::Path,
    path::PathBuf,
    process::Command,
    rc::Rc,
    time::Duration,
};

use smithay::{
    backend::{
        allocator::{
            Fourcc,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        },
        drm::{
            DrmDevice,
            DrmDeviceFd,
            DrmEvent,
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements},
            compositor::FrameFlags,
        },
        egl::{EGLContext, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
            ImportMemWl,
        },
        session::{Event as SessionEvent, Session, libseat::LibSeatSession},
        udev::UdevBackend,
    },
    output::{Mode as WlMode, Output, PhysicalProperties},
    reexports::{
        calloop::EventLoop,
        drm::control::{Device as DrmControlDevice, ModeTypeFlags, connector},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::Display,
    },
    utils::DeviceFd,
};

use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner, SimpleCrtcMapper};

use crate::{
    config::Config,
    nested::AlpiniumState,
};

type NativeDrmOutputManager = DrmOutputManager<
    GbmAllocator<DrmDeviceFd>,
    GbmFramebufferExporter<DrmDeviceFd>,
    (),
    DrmDeviceFd,
>;

type NativeDrmOutput = DrmOutput<
    GbmAllocator<DrmDeviceFd>,
    GbmFramebufferExporter<DrmDeviceFd>,
    (),
    DrmDeviceFd,
>;

fn drm_device_paths(seat_name: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let udev = UdevBackend::new(seat_name)?;
    let devices = udev
        .device_list()
        .map(|(_, path)| path.to_path_buf())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("card"))
        })
        .collect::<Vec<_>>();

    if devices.is_empty() {
        return Err("udev did not find a primary DRM card for this seat".into());
    }

    Ok(devices)
}

pub fn probe() -> Result<(), Box<dyn Error>> {
    println!("Alpinium native backend probe");
    println!("Mode setting: disabled (safe discovery only)");

    let (mut session, _notifier) = LibSeatSession::new()?;
    let seat_name = session.seat();
    println!("libseat session acquired on {seat_name}");

    let devices = drm_device_paths(&seat_name)?;
    println!("Found {} DRM device(s):", devices.len());

    for path in devices {
        let device_id = std::fs::metadata(&path)?.rdev();
        println!();
        println!("DRM device: {} (dev_t {device_id})", path.display());

        let fd = session.open(
            Path::new(&path),
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )?;
        let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (drm, _drm_notifier) = DrmDevice::new(drm_fd, true)?;
        let resources = drm.resource_handles()?;
        let mut connected = 0usize;

        for handle in resources.connectors() {
            let info = drm.get_connector(*handle, false)?;
            if info.state() != connector::State::Connected {
                continue;
            }

            connected += 1;
            println!(
                "  Connected connector: {}-{}",
                info.interface().as_str(),
                info.interface_id()
            );

            let modes = info.modes();
            if modes.is_empty() {
                println!("    No modes reported");
                continue;
            }

            for (index, mode) in modes.iter().enumerate() {
                let preferred = mode.mode_type().contains(ModeTypeFlags::PREFERRED);
                let (width, height) = mode.size();
                println!(
                    "    {}: {}x{} @ {} Hz{}",
                    index,
                    width,
                    height,
                    mode.vrefresh(),
                    if preferred { " (preferred)" } else { "" }
                );
            }
        }

        if connected == 0 {
            println!("  No connected display connectors found");
        }
    }

    println!();
    println!("PASS: libseat acquired the session and DRM connector discovery completed");
    println!("No connector was configured and no display mode was changed.");
    Ok(())
}

/// Initialize GBM/EGL/GLES without creating a DRM output or modesetting.
pub fn renderer_probe() -> Result<(), Box<dyn Error>> {
    println!("Alpinium native renderer probe");
    println!("Mode setting: disabled (GBM/EGL/GLES initialization only)");

    let (mut session, _notifier) = LibSeatSession::new()?;
    let seat_name = session.seat();
    println!("libseat session acquired on {seat_name}");

    let devices = drm_device_paths(&seat_name)?;
    let path = devices
        .first()
        .ok_or("udev did not find any primary DRM card for this seat")?;
    let device_id = std::fs::metadata(path)?.rdev();
    println!("Using DRM device: {} (dev_t {device_id})", path.display());

    let fd = session.open(
        path.as_path(),
        OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
    )?;
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let gbm = GbmDevice::new(drm_fd)?;
    println!("GBM initialized (backend: {})", gbm.backend_name());

    // SAFETY: the GBM device opened for this libseat session backs this EGL display.
    let egl_display = unsafe { EGLDisplay::new(gbm)? };
    let (egl_major, egl_minor) = egl_display.get_egl_version();
    println!("EGL initialized: {egl_major}.{egl_minor}");

    let egl_context = EGLContext::new(&egl_display)?;
    // SAFETY: this context is created and used on the current thread only.
    let renderer = unsafe { GlesRenderer::new(egl_context)? };
    println!("OpenGL ES renderer initialized");
    println!("Renderer capabilities: {:?}", renderer.capabilities());

    println!();
    println!("PASS: GBM, EGL, and GLES renderer initialization completed");
    println!("No DRM output was created and no display mode was changed.");
    Ok(())
}

pub fn run(
    config: Config,
    config_source: Option<PathBuf>,
    command: Option<OsString>,
) -> Result<(), Box<dyn Error>> {
    let mut event_loop: EventLoop<AlpiniumState> = EventLoop::try_new()?;
    let display: Display<AlpiniumState> = Display::new()?;

    let (mut session, session_notifier) = LibSeatSession::new()?;
    let seat_name = session.seat();
    println!("libseat session acquired on {seat_name}");

    let devices = drm_device_paths(&seat_name)?;
    let path = devices
        .first()
        .ok_or("udev did not find a primary DRM card for this seat")?
        .clone();
    let device_id = std::fs::metadata(&path)?.rdev();
    let fd = session.open(
        &path,
        OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
    )?;
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let (drm, drm_notifier) = DrmDevice::new(drm_fd.clone(), true)?;
    let gbm = GbmDevice::new(drm_fd)?;

    // SAFETY: the GBM device is owned by this compositor and bound to its DRM fd.
    let egl_display = unsafe { EGLDisplay::new(gbm.clone())? };
    let egl_context = EGLContext::new(&egl_display)?;
    // SAFETY: renderer and context remain on this event-loop thread.
    let mut renderer = unsafe { GlesRenderer::new(egl_context)? };

    let render_formats = renderer.egl_context().dmabuf_render_formats().iter().copied().collect::<Vec<_>>();
    let allocator = GbmAllocator::new(
        gbm.clone(),
        GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
    );
    let exporter = GbmFramebufferExporter::new(gbm.clone(), None);
    let mut output_manager = NativeDrmOutputManager::new(
        drm,
        allocator,
        exporter,
        Some(gbm),
        [Fourcc::Argb8888, Fourcc::Abgr8888],
        render_formats,
    );

    let mut scanner = DrmScanner::<SimpleCrtcMapper>::new();
    let connected = scanner
        .scan_connectors(output_manager.device())?
        .into_iter()
        .find_map(|event| match event {
            DrmScanEvent::Connected {
                connector,
                crtc: Some(crtc),
            } if format!(
                "{}-{}",
                connector.interface().as_str(),
                connector.interface_id()
            ) == "eDP-1" => Some((connector, crtc)),
            _ => None,
        })
        .ok_or("the verified internal eDP-1 connector is not available")?;

    let (connector, crtc) = connected;
    let mode_index = connector
        .modes()
        .iter()
        .position(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
        .unwrap_or(0);
    let drm_mode = *connector
        .modes()
        .get(mode_index)
        .ok_or("eDP-1 did not report any display modes")?;
    let wl_mode = WlMode::from(drm_mode);
    let (physical_width, physical_height) = connector.size().unwrap_or((0, 0));
    let output = Output::new(
        "eDP-1".to_string(),
        PhysicalProperties {
            size: (physical_width as i32, physical_height as i32).into(),
            subpixel: connector.subpixel().into(),
            make: "Unknown".into(),
            model: "eDP-1".into(),
        },
    );

    let mut state = AlpiniumState::new(&mut event_loop, display, config);
    let _output_global = output.create_global::<AlpiniumState>(&state.display_handle);
    output.set_preferred(wl_mode);
    output.change_current_state(Some(wl_mode), None, None, Some((0, 0).into()));
    state.space.map_output(&output, (0, 0));
    state.apply_layout();
    state.shm_state.update_formats(renderer.shm_formats());

    let planes = output_manager.device().planes(&crtc)?;
    let drm_output = output_manager.initialize_output::<
        GlesRenderer,
        WaylandSurfaceRenderElement<GlesRenderer>,
    >(
        crtc,
        drm_mode,
        &[connector.handle()],
        &output,
        Some(planes),
        &mut renderer,
        &DrmOutputRenderElements::default(),
    )?;

    let drm_output = Rc::new(RefCell::new(drm_output));
    let output_manager = Rc::new(RefCell::new(output_manager));
    let session_active = Rc::new(Cell::new(true));
    let frame_pending = Rc::new(Cell::new(false));

    let mut libinput_context =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(session.clone().into());
    libinput_context.udev_assign_seat(&seat_name).map_err(|_| "libinput failed to assign the active seat")?;
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    let input_insert = event_loop.handle().insert_source(
        libinput_backend,
        move |event, _, state: &mut AlpiniumState| {
            state.process_input_event(event);
        },
    );
    input_insert?;

    let drm_output_for_events = drm_output.clone();
    let pending_for_events = frame_pending.clone();
    event_loop.handle().insert_source(
        drm_notifier,
        move |event, _, _state: &mut AlpiniumState| match event {
            DrmEvent::VBlank(event_crtc) if event_crtc == crtc => {
                if let Err(error) = drm_output_for_events.borrow().frame_submitted() {
                    eprintln!("alpinium: DRM frame completion failed: {error:?}");
                }
                pending_for_events.set(false);
            }
            DrmEvent::VBlank(_) => {}
            DrmEvent::Error(error) => {
                eprintln!("alpinium: DRM event error: {error:?}");
            }
        },
    )?;

    let manager_for_session = output_manager.clone();
    let active_for_session = session_active.clone();
    let pending_for_session = frame_pending.clone();
    let mut input_for_session = libinput_context.clone();
    event_loop.handle().insert_source(
        session_notifier,
        move |event, _, _state: &mut AlpiniumState| match event {
            SessionEvent::PauseSession => {
                active_for_session.set(false);
                pending_for_session.set(false);
                input_for_session.suspend();
                manager_for_session.borrow_mut().pause();
                println!("Alpinium paused: seat switched away");
            }
            SessionEvent::ActivateSession => {
                if let Err(error) = input_for_session.resume() {
                    eprintln!("alpinium: could not resume libinput: {error:?}");
                }
                match manager_for_session.borrow_mut().activate(false) {
                    Ok(()) => {
                        pending_for_session.set(false);
                        active_for_session.set(true);
                        println!("Alpinium resumed: seat is active");
                    }
                    Err(error) => {
                        eprintln!("alpinium: could not reactivate DRM output: {error}");
                    }
                }
            }
        },
    )?;

    let config_description = config_source
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "built-in defaults".to_string());
    println!("Alpinium — native DRM/KMS compositor");
    println!("Configuration: {config_description}");
    println!("DRM device: {} (dev_t {device_id})", path.display());
    println!(
        "Output: eDP-1, {}x{} @ {} Hz",
        wl_mode.size.w,
        wl_mode.size.h,
        drm_mode.vrefresh()
    );
    println!("Wayland socket: {}", state.socket_name.to_string_lossy());
    println!("Exit: Super+Shift+E. Switch back to KDE: Ctrl+Alt+F7.");

    if let Some(program) = command {
        println!("Launching native Wayland client: {}", program.to_string_lossy());
        Command::new(&program)
            .env("WAYLAND_DISPLAY", &state.socket_name)
            .spawn()?;
    }

    let output_for_render = output.clone();
    let drm_output_for_render = drm_output.clone();
    let active_for_render = session_active.clone();
    let pending_for_render = frame_pending.clone();

    event_loop.run(Some(Duration::from_millis(16)), &mut state, move |state| {
        if !active_for_render.get() || pending_for_render.get() {
            return;
        }

        state.space.refresh();
        state.apply_layout();
        state.advance_animations(std::time::Instant::now());

        if let Err(error) = render_native_frame(
            state,
            &output_for_render,
            &mut renderer,
            &drm_output_for_render,
            &pending_for_render,
        ) {
            eprintln!("alpinium: native frame failed: {error}");
            state.loop_signal.stop();
            return;
        }

        state.popups.cleanup();
        let _ = state.display_handle.flush_clients();
    })?;

    Ok(())
}

fn render_native_frame(
    state: &mut AlpiniumState,
    output: &Output,
    renderer: &mut GlesRenderer,
    drm_output: &Rc<RefCell<NativeDrmOutput>>,
    frame_pending: &Cell<bool>,
) -> Result<(), Box<dyn Error>> {
    let elements = crate::render::space_render_elements(state, renderer, output);

    let frame = drm_output.borrow_mut().render_frame(
        renderer,
        &elements,
        [0.08, 0.08, 0.08, 1.0],
        FrameFlags::empty(),
    )?;

    if !frame.is_empty {
        drm_output.borrow_mut().queue_frame(())?;
        frame_pending.set(true);
    }

    state.space.elements().for_each(|window| {
        window.send_frame(
            output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        );
    });

    Ok(())
}

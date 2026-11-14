//! Safe discovery and renderer probes for the future DRM/KMS backend.
//!
//! These diagnostics acquire the active seat and inspect the DRM device.
//! The renderer probe initializes GBM/EGL/GLES but intentionally does not
//! create a DRM output, configure a connector, or modeset.

use std::{
    error::Error,
    os::unix::fs::MetadataExt,
    path::Path,
    path::PathBuf,
};

use smithay::{
    backend::{
        allocator::gbm::GbmDevice,
        drm::{DrmDevice, DrmDeviceFd},
        egl::{EGLContext, EGLDisplay},
        renderer::gles::GlesRenderer,
        session::{Session, libseat::LibSeatSession},
        udev::UdevBackend,
    },
    reexports::{
        drm::control::{Device, ModeTypeFlags, connector},
        rustix::fs::OFlags,
    },
    utils::DeviceFd,
};

fn drm_device_paths(
    seat_name: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let udev = UdevBackend::new(seat_name)?;
    let devices = udev
        .device_list()
        .map(|(_, path)| path.to_path_buf())
        .collect::<Vec<_>>();

    if devices.is_empty() {
        return Err("udev did not find any DRM devices for this seat".into());
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

/// Initialize the GPU rendering stack without taking over a display output.
///
/// This deliberately stops before DrmDevice::create_surface or any KMS commit,
/// so it is safe to use as a driver-compatibility check from a TTY.
pub fn renderer_probe() -> Result<(), Box<dyn Error>> {
    println!("Alpinium native renderer probe");
    println!("Mode setting: disabled (GBM/EGL/GLES initialization only)");

    let (mut session, _notifier) = LibSeatSession::new()?;
    let seat_name = session.seat();
    println!("libseat session acquired on {seat_name}");

    let devices = drm_device_paths(&seat_name)?;
    let path = devices
        .first()
        .ok_or("udev did not find any DRM devices for this seat")?;
    let device_id = std::fs::metadata(path)?.rdev();
    println!("Using DRM device: {} (dev_t {device_id})", path.display());

    let fd = session.open(
        path.as_path(),
        OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
    )?;
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));

    let gbm = GbmDevice::new(drm_fd)?;
    println!("GBM initialized (backend: {})", gbm.backend_name());

    // SAFETY: Alpinium is the only EGL user in this process; the native
    // display is backed by the GBM device opened for this libseat session.
    let egl_display = unsafe { EGLDisplay::new(gbm)? };
    let (egl_major, egl_minor) = egl_display.get_egl_version();
    println!("EGL initialized: {egl_major}.{egl_minor}");

    let egl_context = EGLContext::new(&egl_display)?;
    // SAFETY: this context was just created on the current thread and is not
    // active or shared with any other thread.
    let renderer = unsafe { GlesRenderer::new(egl_context)? };

    let renderer_kind = if renderer.is_software() {
        "software"
    } else {
        "hardware"
    };
    println!("OpenGL ES renderer initialized ({renderer_kind})");
    println!("Renderer capabilities: {:?}", renderer.capabilities());

    println!();
    println!("PASS: GBM, EGL, and GLES renderer initialization completed");
    println!("No DRM output was created and no display mode was changed.");
    Ok(())
}

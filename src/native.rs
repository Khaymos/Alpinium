//! Safe discovery probe for the future DRM/KMS backend.
//!
//! This opens the active seat's DRM devices through libseat and queries
//! connector state. It intentionally does not create a DRM output or modeset.

use std::{
    error::Error,
    path::Path,
};

use smithay::{
    backend::{
        drm::{DrmDevice, DrmDeviceFd},
        session::{Session, libseat::LibSeatSession},
        udev::UdevBackend,
    },
    reexports::{
        drm::control::{Device, ModeTypeFlags, connector},
        rustix::fs::OFlags,
    },
    utils::DeviceFd,
};

pub fn probe() -> Result<(), Box<dyn Error>> {
    println!("Alpinium native backend probe");
    println!("Mode setting: disabled (safe discovery only)");

    let (mut session, _notifier) = LibSeatSession::new()?;
    let seat_name = session.seat();
    println!("libseat session acquired on {seat_name}");

    let udev = UdevBackend::new(&seat_name)?;
    let devices: Vec<_> = udev
        .device_list()
        .map(|(device_id, path)| (device_id, path.to_path_buf()))
        .collect();

    if devices.is_empty() {
        return Err("udev did not find any DRM devices for this seat".into());
    }

    println!("Found {} DRM device(s):", devices.len());
    for (device_id, path) in devices {
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

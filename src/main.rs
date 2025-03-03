#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::{arg, Command};
use hidapi::{self, HidApi};
use log::*;
use std::{error::Error, vec::Vec};

const REPORT_ID: u8 = 1;
const MIN_BRIGHTNESS: u32 = 400;
const MAX_BRIGHTNESS: u32 = 60000;
const BRIGHTNESS_RANGE: u32 = MAX_BRIGHTNESS - MIN_BRIGHTNESS;

const SD_PRODUCT_ID: u16 = 0x1114;
const SD_VENDOR_ID: u16 = 0x05ac;
const SD_INTERFACE_NR: i32 = 0x7;

fn get_brightness(handle: &mut hidapi::HidDevice) -> Result<u32, Box<dyn Error>> {
    let mut buf = Vec::with_capacity(7); // report id, 4 bytes brightness, 2 bytes unknown
    buf.push(REPORT_ID);
    buf.extend(0_u32.to_le_bytes());
    buf.extend(0_u16.to_le_bytes());
    let size = handle.get_feature_report(&mut buf)?;
    if size != buf.len() {
        Err(format!(
            "Get HID feature report: Expected a size of {}, got {}",
            buf.len(),
            size
        ))?
    }
    let brightness = u32::from_le_bytes(buf[1..5].try_into()?);
    Ok(brightness)
}

fn get_brightness_percent(handle: &mut hidapi::HidDevice) -> Result<u8, Box<dyn Error>> {
    let value = (get_brightness(handle)? - MIN_BRIGHTNESS) as f32;
    let value_percent = (value / BRIGHTNESS_RANGE as f32 * 100.0) as u8;
    Ok(value_percent)
}

fn set_brightness(handle: &mut hidapi::HidDevice, brightness: u32) -> Result<(), Box<dyn Error>> {
    let mut buf = Vec::with_capacity(7); // report id, 4 bytes brightness, 2 bytes unknown
    buf.push(REPORT_ID);
    buf.extend(brightness.to_le_bytes());
    buf.extend(0_u16.to_le_bytes());
    handle.send_feature_report(&mut buf)?;
    Ok(())
}

fn set_brightness_percent(handle: &mut hidapi::HidDevice, brightness: u8) -> Result<(), Box<dyn Error>> {
    let nits =
        ((brightness as f32 * BRIGHTNESS_RANGE as f32) / 100.0 + MIN_BRIGHTNESS as f32) as u32;
    let nits = std::cmp::min(nits, MAX_BRIGHTNESS);
    let nits = std::cmp::max(nits, MIN_BRIGHTNESS);
    set_brightness(handle, nits)?;
    Ok(())
}

fn studio_displays(hapi: &HidApi) -> Result<Vec<&hidapi::DeviceInfo>, Box<dyn Error>> {
    Ok(hapi
        .device_list()
        .filter(|x| {
            x.product_id() == SD_PRODUCT_ID
                && x.vendor_id() == SD_VENDOR_ID
                && x.interface_number() == SD_INTERFACE_NR
        })
        .collect())
}

fn cli() -> Command {
    Command::new("asdbctl")
        .about("Tool to get or set the brightness for Apple Studio Displays")
        // The serial option is defined at the root so it applies to any subcommand.
        .arg(arg!(-s --serial <SERIAL> "Serial number of the display for which to adjust the brightness"))
        .arg(arg!(-v --verbose ... "Turn debugging information on"))
        .subcommand(Command::new("get").about("Get the current brightness in %"))
        .subcommand(
            Command::new("set")
                .about("Set the current brightness in %")
                .arg(
                    arg!(<BRIGHTNESS> "Brightness percentage")
                        .value_parser(clap::value_parser!(u8).range(0..101)),
                )
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("up")
                .arg(
                    arg!(-s --step <STEP> "Step size in percent")
                        .required(false)
                        .default_value("10")
                        .value_parser(clap::value_parser!(u8).range(1..101)),
                )
                .about("Increase the brightness"),
        )
        .subcommand(
            Command::new("down")
                .arg(
                    arg!(-s --step <STEP> "Step size in percent")
                        .required(false)
                        .default_value("10")
                        .value_parser(clap::value_parser!(u8).range(1..101)),
                )
                .about("Decrease the brightness"),
        )
        .subcommand(Command::new("gui").about("Launch Windows taskbar interface with brightness slider"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = cli().get_matches();
    let verbosity = *matches.get_one::<u8>("verbose").unwrap_or(&0) as usize;
    stderrlog::new().module(module_path!()).verbosity(verbosity).init().unwrap();

    // If no subcommand is provided, launch the GUI.
    if matches.subcommand().is_none() {
        let serial = matches.get_one::<String>("serial").map(|s| s.to_string());
        gui::launch_gui(serial)?;
        return Ok(());
    }

    // --- Existing CLI mode ---
    let hapi = HidApi::new()?;
    let displays = studio_displays(&hapi)?;
    if displays.is_empty() {
        Err("No Apple Studio Display found")?;
    }

    for display in displays {
        let mut handle = hapi.open_path(display.path())?;
        if let Some(s) = display.serial_number() {
            info!("display serial number {}", s);
        }
        if let Some(serial) = matches.get_one::<String>("serial") {
            if let Some(s) = display.serial_number() {
                if s != *serial {
                    continue;
                }
            }
        }
        match matches.subcommand() {
            Some(("get", _)) => {
                let brightness = get_brightness_percent(&mut handle)?;
                println!("brightness {}", brightness);
            }
            Some(("set", sub_matches)) => {
                let brightness = *sub_matches.get_one::<u8>("BRIGHTNESS").expect("required");
                set_brightness_percent(&mut handle, brightness)?;
            }
            Some(("up", sub_matches)) => {
                let step = *sub_matches.get_one::<u8>("step").expect("required");
                let brightness = get_brightness_percent(&mut handle)?;
                let new_brightness = std::cmp::min(100, brightness + step);
                set_brightness_percent(&mut handle, new_brightness)?;
            }
            Some(("down", sub_matches)) => {
                let step = *sub_matches.get_one::<u8>("step").expect("required");
                let brightness = get_brightness_percent(&mut handle)?;
                // Use saturating_sub to prevent underflow.
                let new_brightness = brightness.saturating_sub(step);
                set_brightness_percent(&mut handle, new_brightness)?;
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

//
// --- GUI module using Slint ---
//

mod gui {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;


    slint::include_modules!();

    // Launch the GUI.
    // It finds the display (optionally filtering by serial if provided),
    // opens the device handle, reads the initial brightness, and creates the UI.
    // When the slider is moved, the brightness is updated.
    pub fn launch_gui(serial: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        
        let hapi = HidApi::new()?;
        let displays = studio_displays(&hapi)?;
        if displays.is_empty() {
            return Err("No Apple Studio Display found".into());
        }
        let display = displays
            .into_iter()
            .find(|d| {
                if let Some(ref serial_filter) = serial {
                    d.serial_number().map(|s| s == serial_filter).unwrap_or(false)
                } else {
                    true
                }
            })
            .ok_or("No display found with specified serial")?;

        let handle = hapi.open_path(display.path())?;
        let mut handle = handle; // make mutable to use for get/set calls

        let initial_brightness = get_brightness_percent(&mut handle)?;

        // Instantiate the Slint UI.
        let ui = BrightnessUI::new().unwrap();
        ui.set_brightness(initial_brightness as f32);

        // Wrap the handle in a Rc<RefCell> so it can be shared with the callback.
        let handle_rc = Rc::new(RefCell::new(handle));
        let handle_for_callback = handle_rc.clone();

        // Connect the Slint callback so that whenever the slider changes,
        // the display brightness is updated.
        ui.on_brightness_changed(move |new_value: f32| {
            let new_value_u8 = new_value as u8;
            if let Err(e) = set_brightness_percent(&mut handle_for_callback.borrow_mut(), new_value_u8) {
                eprintln!("Failed to set brightness: {:?}", e);
            }
        });

        // Run the GUI event loop.
        let _ = ui.run();
        Ok(())
    }
}

use std::ptr;

use winapi::{
    shared::{
        minwindef::{BOOL, LPARAM, TRUE},
        windef::{HDC, HMONITOR, LPRECT},
    },
    um::winuser::EnumDisplayMonitors,
};

#[derive(Debug)]
pub struct Monitor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

unsafe extern "system" fn monitor_enum_proc(
    _: HMONITOR,
    _: HDC,
    rect: LPRECT,
    monitors: LPARAM,
) -> BOOL {
    let monitors = &mut *(monitors as *mut Vec<Monitor>);

    let rect = *rect;
    monitors.push(Monitor {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left) as u32,
        height: (rect.bottom - rect.top) as u32,
    });

    TRUE
}

pub fn enumerate() -> Vec<Monitor> {
    let mut monitors = Vec::new();

    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(monitor_enum_proc),
            ptr::addr_of_mut!(monitors) as isize,
        );
    }

    monitors
}

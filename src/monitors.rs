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
    pub width: i32,
    pub height: i32,
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
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
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
            &mut monitors as *mut _ as isize,
        );
    }

    monitors
}

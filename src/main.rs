mod console;
mod ffi;
mod gl;
mod screenshot;
mod zoomer;

use std::{
    ffi::CStr,
    mem::{size_of, size_of_val},
};

use ffi::{c_str, c_str_ptr};
use gl::*;

use nalgebra_glm::{vec2, vec3, Mat4, Vec2, Vec3};
use winapi::{
    shared::{
        minwindef::*,
        windef::{HDC, HWND},
        windowsx::{GET_X_LPARAM, GET_Y_LPARAM},
    },
    um::{libloaderapi::GetModuleHandleA, sysinfoapi::GetTickCount, wingdi::*, winuser::*},
};

use crate::zoomer::Zoomer;

const WIDTH: i32 = 1920;
const HEIGHT: i32 = 1080;

fn main() {
    console::init();

    let instance = unsafe { GetModuleHandleA(std::ptr::null()) };
    assert!(!instance.is_null());

    let class = unsafe {
        RegisterClassExA(&WNDCLASSEXA {
            cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: c_str_ptr!("ZoomerClass"),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            ..Default::default()
        })
    };
    assert!(class != 0);

    let window = unsafe {
        CreateWindowExA(
            0,
            std::mem::transmute(class as usize),
            c_str_ptr!("Zoomer or something"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIDTH,
            HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        )
    };
    assert!(!window.is_null());

    let hdc = unsafe { GetDC(window) };
    assert!(!hdc.is_null());

    let mut zoomer = Zoomer::default();

    zoomer.init(window);

    unsafe {
        SetWindowLongPtrA(window, GWLP_USERDATA, &mut zoomer as *mut _ as isize);
    }

    let _start_time = unsafe { GetTickCount() };

    unsafe {
        ShowWindow(window, SW_SHOW);

        let mut message = MSG::default();

        loop {
            while PeekMessageA(&mut message, window, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageA(&message);
            }

            let _time = GetTickCount() - _start_time;
        }
    }
}

unsafe extern "system" fn window_proc(
    handle: HWND,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> LRESULT {
    use winapi::um::winuser::*;

    let zoomer = &mut *(GetWindowLongPtrW(handle, GWLP_USERDATA) as *mut Zoomer);

    match message {
        WM_MOUSEWHEEL => {
            let delta = GET_WHEEL_DELTA_WPARAM(w_param);
            let x = GET_X_LPARAM(l_param);
            let y = GET_Y_LPARAM(l_param);

            println!("delta = {}, x = {}, y = {}", delta, x, y);
        }
        WM_SIZE => {
            let width = LOWORD(l_param as DWORD);
            let height = HIWORD(l_param as DWORD);

            zoomer.on_resize(width as u32, height as u32);
        }
        WM_DESTROY => {
            PostQuitMessage(0);
        }
        _ => return DefWindowProcA(handle, message, w_param, l_param),
    }

    0
}

#![feature(backtrace)]

mod camera;
mod highlighter;
mod screenshot;
mod zoomer;

mod console;
mod ffi;
mod gl;
mod imgui_impl;
mod interpolation;
mod monitors;

use std::time::Instant;

use winapi::{
    shared::{
        minwindef::*,
        windef::{HWND, POINT, RECT},
        windowsx::{GET_X_LPARAM, GET_Y_LPARAM},
        winerror::S_OK,
    },
    um::{
        libloaderapi::GetModuleHandleA,
        shellscalingapi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE},
        winuser::*,
    },
};

use ffi::c_str_ptr;
use imgui_impl::*;
use zoomer::Zoomer;

use crate::gl::wglSwapIntervalEXT;

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
            c_str_ptr!("Zoomer"),
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

    unsafe {
        assert_eq!(SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE), S_OK);
    }

    let mut zoomer = Zoomer::new();

    let (client_width, client_height) = unsafe {
        let mut rect = RECT::default();

        GetClientRect(window, &mut rect);

        (rect.right - rect.left, rect.bottom - rect.top)
    };

    zoomer.init(window, client_width, client_height);

    // Store a pointer to the zoomer object in the window so that we can access it from the `window_proc`.
    unsafe {
        SetWindowLongPtrA(window, GWLP_USERDATA, &mut zoomer as *mut _ as isize);
    }

    // Enable V-Sync. It seems like this is the default, but just in case.
    unsafe { wglSwapIntervalEXT(1) };

    let mut message = MSG::default();
    let mut dt_timer = Instant::now();

    unsafe {
        ShowWindow(window, SW_SHOW);

        'main: loop {
            while PeekMessageA(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if message.message == WM_QUIT {
                    break 'main;
                }

                TranslateMessage(&message);
                DispatchMessageA(&message);
            }

            zoomer.render();
            zoomer.update(dt_timer.elapsed().as_secs_f32());

            dt_timer = Instant::now();
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    use winapi::um::winuser::*;

    let zoomer = GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Zoomer;

    if zoomer.is_null() {
        // zoomer has not been initialized yet.
        return DefWindowProcA(window, message, w_param, l_param);
    }

    let zoomer = &mut *zoomer;

    // SetCapture() allows from when mouse is outside of the window to be captured.
    if ImGui_ImplWin32_WndProcHandler(window, message, w_param, l_param) != 0 {
        return 1;
    }

    match message {
        WM_SIZE => {
            let width = LOWORD(l_param as DWORD);
            let height = HIWORD(l_param as DWORD);

            zoomer.on_resize(width, height);
        }
        WM_LBUTTONDOWN => {
            if zoomer.imgui_wants_mouse_events() {
                return 0;
            }

            let x = GET_X_LPARAM(l_param);
            let y = GET_Y_LPARAM(l_param);

            zoomer.on_left_mouse_down(x, y);
        }
        WM_LBUTTONUP => {
            zoomer.on_left_mouse_up();
        }
        WM_MOUSEMOVE => {
            if zoomer.imgui_wants_mouse_events() {
                return 0;
            }

            let x = GET_X_LPARAM(l_param);
            let y = GET_Y_LPARAM(l_param);

            zoomer.on_mouse_move(x, y, w_param & MK_LBUTTON != 0);
        }
        WM_MOUSEWHEEL => {
            if zoomer.imgui_wants_mouse_events() {
                return 0;
            }

            let delta = GET_WHEEL_DELTA_WPARAM(w_param);
            let x = GET_X_LPARAM(l_param);
            let y = GET_Y_LPARAM(l_param);

            let mut point = POINT { x, y };
            ScreenToClient(window, &mut point);

            zoomer.on_mouse_wheel(delta, point.x, point.y, w_param & MK_CONTROL != 0);
        }
        WM_KEYDOWN => {
            if zoomer.imgui_wants_keyboard_events() {
                return 0;
            }

            let key = w_param as u8;

            zoomer.on_key_down(key);
        }
        WM_DESTROY => {
            PostQuitMessage(0);
        }
        _ => return DefWindowProcA(window, message, w_param, l_param),
    }

    0
}

use std::{ffi::c_void, os::raw::c_char};

use winapi::shared::{
    minwindef::{LPARAM, LRESULT, UINT, WPARAM},
    windef::HWND,
};

extern "C" {
    pub fn ImGui_ImplWin32_Init(window: *const c_void) -> bool;
    pub fn ImGui_ImplOpenGL3_Init(gl_version: *const c_char) -> bool;

    pub fn ImGui_ImplWin32_WndProcHandler(
        window: HWND,
        msg: UINT,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT;

    pub fn ImGui_ImplOpenGL3_NewFrame();
    pub fn ImGui_ImplWin32_NewFrame();

    pub fn ImGui_ImplOpenGL3_RenderDrawData(draw_data: *mut imgui::DrawData);
}

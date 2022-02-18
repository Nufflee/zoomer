mod ffi;
mod gl;
mod screenshot;

use std::{
    ffi::CStr,
    mem::{size_of, size_of_val},
};

use ffi::{c_str, c_str_ptr};
use gl::*;
use screenshot::take_screenshot;

use nalgebra::Matrix4;
use stb::image_write::stbi_write_png;
use winapi::{
    shared::{
        minwindef::*,
        windef::{HDC, HWND},
        windowsx::{GET_X_LPARAM, GET_Y_LPARAM},
    },
    um::{
        libloaderapi::GetModuleHandleA, sysinfoapi::GetTickCount, wingdi::*, winnt::HANDLE,
        winuser::*,
    },
};

unsafe extern "system" fn window_proc(
    handle: HWND,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> LRESULT {
    use winapi::um::winuser::*;

    match message {
        WM_MOUSEWHEEL => {
            let delta = GET_WHEEL_DELTA_WPARAM(w_param);
            let x = GET_X_LPARAM(l_param);
            let y = GET_Y_LPARAM(l_param);

            println!("delta = {}, x = {}, y = {}", delta, x, y);
        }
        WM_DESTROY => {
            PostQuitMessage(0);
        }
        WM_SIZE => {
            let width = LOWORD(l_param as DWORD);
            let height = HIWORD(l_param as DWORD);

            println!("width = {}, height = {}", width, height);
            glViewport(0, 0, width as GLuint, height as GLuint);
        }
        _ => return DefWindowProcA(handle, message, w_param, l_param),
    }

    0
}

fn is_wgl_extension_supported(hdc: HDC, extension_name: &str) -> bool {
    let extensions = unsafe {
        let extensions = CStr::from_ptr(wglGetExtensionsStringARB(hdc))
            .to_str()
            .expect("non UTF8 characters in WGL extensions string");

        extensions.split(' ').collect::<Vec<_>>()
    };

    extensions.contains(&extension_name)
}

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() {
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
            WIDTH as i32,
            HEIGHT as i32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        )
    };

    assert!(!window.is_null());

    let hdc = unsafe { GetDC(window) };

    unsafe {
        // Current format probably doesn't support OpenGL, so let's create a new poggers one.
        let format_descriptor = PIXELFORMATDESCRIPTOR {
            nSize: size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            dwFlags: PFD_DRAW_TO_WINDOW
                | PFD_SUPPORT_OPENGL
                | PFD_SUPPORT_COMPOSITION
                | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cAlphaBits: 8,
            iLayerType: PFD_MAIN_PLANE,
            ..Default::default()
        };

        let format_index = ChoosePixelFormat(hdc, &format_descriptor);
        assert!(format_index != 0);

        assert!(SetPixelFormat(hdc, format_index, &format_descriptor) != 0);

        #[rustfmt::skip]
        let attribs = [
            WGL_CONTEXT_MAJOR_VERSION_ARB, 3,
            WGL_CONTEXT_MINOR_VERSION_ARB, 2,
            WGL_CONTEXT_PROFILE_MASK_ARB, WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
            0
        ];

        // Reference: https://github.com/glfw/glfw/blob/master/src/wgl_context.c#L535
        // Create and bind a dummy OpenGL context so we can load extension functions.
        let dummy_context = wglCreateContext(hdc);
        wglMakeCurrent(hdc, dummy_context);

        if !is_wgl_extension_supported(hdc, "WGL_ARB_create_context_profile") {
            panic!("`WGL_ARB_create_context_profile` extension not supported");
        }

        let opengl_handle = wglCreateContextAttribsARB(hdc, std::ptr::null_mut(), attribs.as_ptr());
        assert!(!opengl_handle.is_null());

        // Clean up the dummy context.
        wglMakeCurrent(hdc, std::ptr::null_mut());
        wglDeleteContext(dummy_context);

        assert!(wglMakeCurrent(hdc, opengl_handle) != 0);

        println!("OpenGL context created!");

        let version = glGetString(GL_VERSION);
        assert!(!version.is_null());
        println!(
            "OpenGL version: {}",
            CStr::from_ptr(version as *const i8).to_str().unwrap()
        );

        glClearColor(0.5, 0.5, 0.5, 1.0);
    }

    #[rustfmt::skip]
    let vertices: [GLfloat; 18] = [
         1.0 * 8.0,  1.0 * 4.5, 0.0,
         1.0 * 8.0, -1.0 * 4.5, 0.0,
        -1.0 * 8.0, -1.0 * 4.5, 0.0,

        -1.0 * 8.0, -1.0 * 4.5, 0.0,
        -1.0 * 8.0,  1.0 * 4.5, 0.0,
         1.0 * 8.0,  1.0 * 4.5, 0.0
    ];

    #[rustfmt::skip]
    let colors: [GLfloat; 18] = [
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,

        0.0, 0.0, 1.0,
        1.0, 1.0, 1.0,
        1.0, 0.0, 0.0,
    ];

    let vao = unsafe {
        let mut vao = 0;

        glGenVertexArrays(1, &mut vao);

        vao
    };

    let vertex_buffer = unsafe {
        let mut vbo = 0;

        glGenBuffers(1, &mut vbo);

        vbo
    };

    let color_buffer = unsafe {
        let mut buffer = 0;

        glGenBuffers(1, &mut buffer);

        buffer
    };

    unsafe {
        glBindVertexArray(vao);
        glBindBuffer(GL_ARRAY_BUFFER, vertex_buffer);
        {
            glBufferData(
                GL_ARRAY_BUFFER,
                size_of_val(&vertices) as u32,
                vertices.as_ptr() as *const GLvoid,
                GL_STATIC_DRAW,
            );

            glVertexAttribPointer(
                0,
                3,
                GL_FLOAT,
                false,
                3 * size_of::<GLfloat>() as GLsizei,
                std::ptr::null(),
            );
            glEnableVertexAttribArray(0);
        }

        glBindBuffer(GL_ARRAY_BUFFER, color_buffer);
        {
            glBufferData(
                GL_ARRAY_BUFFER,
                size_of_val(&colors) as u32,
                colors.as_ptr() as *const GLvoid,
                GL_STATIC_DRAW,
            );

            glVertexAttribPointer(
                1,
                3,
                GL_FLOAT,
                false,
                3 * size_of::<GLfloat>() as GLsizei,
                std::ptr::null(),
            );
            glEnableVertexAttribArray(1);
        }

        glBindBuffer(GL_ARRAY_BUFFER, 0);
        glBindVertexArray(0);
    }

    let vertex_shader_source = c_str!(
        r#"
        #version 330 core

        layout(location = 0) in vec3 position;
        layout(location = 1) in vec3 color;

        uniform mat4 u_ViewMatrix;
        uniform mat4 u_ProjectionMatrix;

        out vec3 v_Color;

        void main() {
            gl_Position = u_ViewMatrix * u_ProjectionMatrix * vec4(position, 1.0);
            v_Color = color;
        }
    "#
    );

    let fragment_shader_source = c_str!(
        r#"
        #version 330 core

        in vec3 v_Color;

        out vec4 color;

        void main() {
            color = vec4(v_Color, 1.0);
        }
    "#
    );

    let shader_program = unsafe {
        let vertex_shader = glCreateShader(GL_VERTEX_SHADER);

        glShaderSource(
            vertex_shader,
            1,
            &vertex_shader_source.as_ptr(),
            std::ptr::null(),
        );
        glCompileShader(vertex_shader);

        let mut success = true;

        glGetShaderiv(
            vertex_shader,
            GL_COMPILE_STATUS,
            &mut success as *mut _ as *mut GLint,
        );

        if !success {
            eprintln!("Failed to compile the vertex shader!");
        }

        let fragment_shader = glCreateShader(GL_FRAGMENT_SHADER);

        glShaderSource(
            fragment_shader,
            1,
            &fragment_shader_source.as_ptr(),
            std::ptr::null(),
        );
        glCompileShader(fragment_shader);

        let mut success = true;

        glGetShaderiv(
            vertex_shader,
            GL_COMPILE_STATUS,
            &mut success as *mut _ as *mut GLint,
        );

        if !success {
            eprintln!("Failed to compile the fragment shader!");
        }

        let shader_program = glCreateProgram();

        glAttachShader(shader_program, vertex_shader);
        glAttachShader(shader_program, fragment_shader);
        glLinkProgram(shader_program);

        glGetProgramiv(
            shader_program,
            GL_LINK_STATUS,
            &mut success as *mut _ as *mut GLint,
        );

        if !success {
            eprintln!("Failed to link the shader program!");
        }

        shader_program
    };

    let view_matrix_uniform =
        unsafe { glGetUniformLocation(shader_program, c_str_ptr!("u_ViewMatrix")) };
    assert!(view_matrix_uniform != -1);

    let projection_matrix_uniform =
        unsafe { glGetUniformLocation(shader_program, c_str_ptr!("u_ProjectionMatrix")) };
    assert!(projection_matrix_uniform != -1);

    let view_matrix = Matrix4::identity();
    let projection_matrix = Matrix4::new_orthographic(-8.0, 8.0, -4.5, 4.5, -1.0, 1.0);

    println!("View matrix: {:#?}", view_matrix);

    unsafe {
        // TODO: Error handling - will it emit an error if I forget to use a shader program when setting uniforms?
        glUseProgram(shader_program);

        glUniformMatrix4fv(view_matrix_uniform, 1, false, view_matrix.as_ptr());
        glUniformMatrix4fv(
            projection_matrix_uniform,
            1,
            false,
            projection_matrix.as_ptr(),
        );
    }

    // Screenshot stuff
    {
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32 };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32 };
        let start_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let start_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };

        let screenshot = take_screenshot(std::ptr::null_mut(), start_x, start_y, width, height);

        println!("Screenshot: {:?}", &screenshot.pixel_bytes()[..100]);

        stbi_write_png(
            c_str!("rust_window.png"),
            width as i32,
            height as i32,
            4,
            screenshot.pixel_bytes(),
            screenshot.stride() as i32,
        )
        .expect(".png writing failed");

        println!("Screenshot saved!");
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

            glClear(GL_COLOR_BUFFER_BIT);

            glUseProgram(shader_program);
            glBindVertexArray(vao);
            {
                glDrawArrays(GL_TRIANGLES, 0, 6);
            }
            glUseProgram(0);
            glBindVertexArray(0);

            assert!(SwapBuffers(hdc) != 0);
        }
    }
}

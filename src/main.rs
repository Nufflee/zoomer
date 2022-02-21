mod console;
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

use nalgebra::{Matrix4, Vector2, Vector3};
use winapi::{
    shared::{
        minwindef::*,
        windef::{HDC, HWND},
        windowsx::{GET_X_LPARAM, GET_Y_LPARAM},
    },
    um::{
        errhandlingapi::GetLastError, libloaderapi::GetModuleHandleA, sysinfoapi::GetTickCount,
        wingdi::*, winnt::HANDLE, winuser::*,
    },
};

struct Zoomer {
    pub client_width: u16,
    pub client_height: u16,
}

unsafe extern "system" fn window_proc(
    handle: HWND,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> LRESULT {
    use winapi::um::winuser::*;

    let mut zoomer = &mut *(GetWindowLongPtrW(handle, GWLP_USERDATA) as *mut Zoomer);

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

            zoomer.client_width = width;
            zoomer.client_height = height;

            glViewport(0, 0, width as GLuint, height as GLuint);
        }
        _ => return DefWindowProcA(handle, message, w_param, l_param),
    }

    0
}

unsafe extern "C" fn gl_message_callback(
    _source: GLenum,
    type_: GLenum,
    _id: GLuint,
    severity: GLenum,
    _length: GLsizei,
    message: *const GLchar,
    _user_param: *mut GLvoid,
) {
    if severity == GL_DEBUG_SEVERITY_NOTIFICATION {
        return;
    }

    let message = CStr::from_ptr(message);
    let message = message.to_string_lossy();

    use console::{Color, SimpleColor};

    fn type_to_str(type_: GLenum) -> &'static str {
        match type_ {
            GL_DEBUG_TYPE_ERROR => "ERROR",
            GL_DEBUG_TYPE_DEPRECATED_BEHAVIOR => "DEPRECATED BEHAVIOR",
            GL_DEBUG_TYPE_UNDEFINED_BEHAVIOR => "UNDEFINED BEHAVIOR",
            GL_DEBUG_TYPE_PORTABILITY => "PORTABILITY",
            GL_DEBUG_TYPE_PERFORMANCE => "PERFORMANCE",
            GL_DEBUG_TYPE_OTHER => "OTHER",
            _ => unreachable!(),
        }
    }

    fn severity_to_color(severity: GLenum) -> SimpleColor {
        match severity {
            GL_DEBUG_SEVERITY_HIGH => SimpleColor::Red,
            GL_DEBUG_SEVERITY_MEDIUM => SimpleColor::Yellow,
            GL_DEBUG_SEVERITY_LOW => SimpleColor::White,
            GL_DEBUG_SEVERITY_NOTIFICATION => SimpleColor::White,
            _ => unreachable!(),
        }
    }

    let color = severity_to_color(severity);

    console::writeln(
        console::text(format!(
            "OpenGL message [{}]: {}",
            type_to_str(type_),
            message
        ))
        .foreground(Color::Simple(color)),
    );
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

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

fn main() {
    let instance = unsafe { GetModuleHandleA(std::ptr::null()) };

    console::init();

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

    let mut zoomer = Zoomer {
        client_width: 0,
        client_height: 0,
    };

    unsafe {
        SetWindowLongPtrA(window, GWLP_USERDATA, &mut zoomer as *mut _ as isize);
    }

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

        // Reference: https://github.com/glfw/glfw/blob/master/src/wgl_context.c#L535
        // Create and bind a dummy OpenGL context so we can load extension functions.
        let dummy_context = wglCreateContext(hdc);
        wglMakeCurrent(hdc, dummy_context);

        if !is_wgl_extension_supported(hdc, "WGL_ARB_create_context_profile") {
            panic!("`WGL_ARB_create_context_profile` extension not supported");
        }

        #[rustfmt::skip]
        let attribs = [
            WGL_CONTEXT_MAJOR_VERSION_ARB, 3,
            WGL_CONTEXT_MINOR_VERSION_ARB, 2,
            WGL_CONTEXT_FLAGS_ARB, WGL_CONTEXT_DEBUG_BIT_ARB,
            WGL_CONTEXT_PROFILE_MASK_ARB, WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
            0 // null-terminated
        ];

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

        glDebugMessageCallback(gl_message_callback, std::ptr::null_mut());
    }

    #[rustfmt::skip]
    let vertices: [Vector3<f32>; 4] = [
        Vector3::new( -1.0,   1.0, 0.0), // top left
        Vector3::new( -1.0,  -1.0, 0.0), // bottom left
        Vector3::new(  1.0,  -1.0, 0.0), // bottom right
        Vector3::new(  1.0,   1.0, 0.0), // top right
    ];

    #[rustfmt::skip]
    let colors: [Vector3<f32>; 4] = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 1.0, 1.0),
    ];

    #[rustfmt::skip]
    let uvs: [Vector2<f32>; 4] = [
        Vector2::new(0.0, 0.0),
        Vector2::new(0.0, 1.0),
        Vector2::new(1.0, 1.0),
        Vector2::new(1.0, 0.0),
    ];

    #[rustfmt::skip]
    let indices: [u8; 6] = [
        0, 1, 2,
        2, 3, 0
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

    let uv_buffer = unsafe {
        let mut buffer = 0;

        glGenBuffers(1, &mut buffer);

        buffer
    };

    let index_buffer = unsafe {
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

        glBindBuffer(GL_ARRAY_BUFFER, uv_buffer);
        {
            glBufferData(
                GL_ARRAY_BUFFER,
                size_of_val(&uvs) as u32,
                uvs.as_ptr() as *const GLvoid,
                GL_STATIC_DRAW,
            );

            glVertexAttribPointer(
                2,
                2,
                GL_FLOAT,
                false,
                2 * size_of::<GLfloat>() as GLsizei,
                std::ptr::null(),
            );
            glEnableVertexAttribArray(2);
        }

        glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, index_buffer);
        {
            glBufferData(
                GL_ELEMENT_ARRAY_BUFFER,
                size_of_val(&indices) as u32,
                indices.as_ptr() as *const GLvoid,
                GL_STATIC_DRAW,
            );
        }

        glBindBuffer(GL_ARRAY_BUFFER, 0);
        glBindVertexArray(0);
    }

    let vertex_shader_source = c_str!(
        r#"
        #version 330 core

        layout(location = 0) in vec3 position;
        layout(location = 1) in vec3 color;
        layout(location = 2) in vec2 texCoord;

        uniform mat4 u_ViewMatrix;

        out vec3 v_Color;
        out vec2 v_TexCoord;

        void main() {
            v_Color = color;
            v_TexCoord = texCoord;
            gl_Position = u_ViewMatrix * vec4(position, 1.0);
        }
    "#
    );

    let fragment_shader_source = c_str!(
        r#"
        #version 330 core

        in vec3 v_Color;
        in vec2 v_TexCoord;

        out vec4 color;

        uniform sampler2D u_Texture;

        void main() {
            color = texture(u_Texture, v_TexCoord);
            // color = vec4(v_TexCoord, 0.0, 1.0);
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
            let mut info_log = vec![0; 512];

            glGetShaderInfoLog(
                vertex_shader,
                512,
                std::ptr::null_mut(),
                info_log.as_mut_ptr() as *mut GLchar,
            );

            panic!(
                "Failed to compile the vertex shader! Error: {}",
                CStr::from_ptr(info_log.as_ptr()).to_str().unwrap()
            );
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

    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32 };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32 };
    let start_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let start_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };

    let start = std::time::Instant::now();

    let screenshot = take_screenshot(std::ptr::null_mut(), start_x, start_y, width, height);

    println!(
        "Screenshot taken in {} seconds!",
        start.elapsed().as_secs_f32()
    );

    let texture = unsafe {
        let mut texture = 0;

        glGenTextures(1, &mut texture);

        texture
    };

    println!(
        "Screenshot: {}x{} (AR = {})",
        screenshot.width(),
        screenshot.height(),
        screenshot.width() as f32 / screenshot.height() as f32
    );

    unsafe {
        glBindTexture(GL_TEXTURE_2D, texture);

        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);

        glTexImage2D(
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            screenshot.width(),
            screenshot.height(),
            0,
            GL_RGBA as GLenum,
            GL_UNSIGNED_BYTE,
            screenshot.pixel_bytes() as *const _ as *const GLvoid,
        );

        glBindTexture(GL_TEXTURE_2D, 0);
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

            let client_aspect_ratio = zoomer.client_width as f32 / zoomer.client_height as f32;
            let screenshot_aspect_ratio = screenshot.width() as f32 / screenshot.height() as f32;
            let view_matrix = Matrix4::new_nonuniform_scaling(&Vector3::new(
                1.0,
                client_aspect_ratio / screenshot_aspect_ratio,
                1.0,
            ));

            glClear(GL_COLOR_BUFFER_BIT);

            glActiveTexture(GL_TEXTURE0);
            glBindTexture(GL_TEXTURE_2D, texture);
            glUseProgram(shader_program);

            {
                glUniformMatrix4fv(view_matrix_uniform, 1, false, view_matrix.as_ptr());

                glBindVertexArray(vao);
                glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, index_buffer);
                {
                    glDrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_BYTE, std::ptr::null());
                }
            }

            glUseProgram(0);
            glBindVertexArray(0);
            glBindTexture(GL_TEXTURE_2D, 0);

            assert!(SwapBuffers(hdc) != 0);
        }
    }
}

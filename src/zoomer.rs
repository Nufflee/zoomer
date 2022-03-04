use std::ffi::c_void;
use std::{
    ffi::{CStr, CString},
    mem::{size_of, size_of_val},
};

use crate::camera::Camera;
use crate::ffi::c_str_ptr;
use crate::gl::*;
use crate::imgui_impl::*;
use crate::screenshot::take_screenshot;
use crate::{console, screenshot::Screenshot};

use nalgebra_glm::{vec2, vec3, Mat4, Vec2, Vec3};
use winapi::{
    shared::windef::{HDC, HWND},
    um::{wingdi::*, winuser::*},
};

const VERTEX_SHADER: &str = r#"
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
"#;

const FRAGMENT_SHADER: &str = r#"
#version 330 core

in vec3 v_Color;
in vec2 v_TexCoord;

out vec4 color;

uniform sampler2D u_Texture;

void main() {
    color = texture(u_Texture, v_TexCoord);
    // color = vec4(v_TexCoord, 0.0, 1.0);
}
"#;

#[derive(Default)]
pub struct Zoomer {
    pub client_width: u32,
    pub client_height: u32,

    screenshot: Option<Screenshot>,
    hdc: Option<HDC>,
    vao_id: GLuint,
    texture_id: GLuint,
    index_buffer_id: GLuint,
    shader_program_id: GLuint,
    view_matrix_uniform: GLint,
    pub imgui: Option<imgui::Context>,

    last_mouse_pos: Vec2,
    camera: Camera,
}

impl Zoomer {
    pub fn init(&mut self, window: HWND) {
        self.take_screenshot();

        self.hdc = Some(unsafe { GetDC(window) });

        self.create_opengl_context();
        self.init_render_env();

        self.init_imgui(window);

        unsafe {
            glClearColor(0.25, 0.25, 0.28, 1.0);
        }
    }

    fn create_opengl_context(&self) {
        // Current format probably doesn't support OpenGL, so let's create a new poggers one.
        let format_descriptor = PIXELFORMATDESCRIPTOR {
            nSize: size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            dwFlags: PFD_DRAW_TO_WINDOW
                | PFD_SUPPORT_OPENGL
                | PFD_SUPPORT_COMPOSITION
                | PFD_DOUBLEBUFFER,
            cColorBits: 32,
            cAlphaBits: 8,
            ..Default::default()
        };

        let hdc = self.hdc.unwrap();

        let format_index = unsafe { ChoosePixelFormat(hdc, &format_descriptor) };
        assert!(format_index != 0);

        assert!(unsafe { SetPixelFormat(hdc, format_index, &format_descriptor) } != 0);

        // Reference: https://github.com/glfw/glfw/blob/master/src/wgl_context.c#L535
        // Create and bind a dummy OpenGL context so we can load extension functions.
        let dummy_context = unsafe {
            let dummy_context = wglCreateContext(hdc);
            wglMakeCurrent(hdc, dummy_context);

            dummy_context
        };

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

        let opengl_handle =
            unsafe { wglCreateContextAttribsARB(hdc, std::ptr::null_mut(), attribs.as_ptr()) };
        assert!(!opengl_handle.is_null());

        // Clean up the dummy context.
        unsafe {
            wglMakeCurrent(hdc, std::ptr::null_mut());
            wglDeleteContext(dummy_context);
        }

        assert!(unsafe { wglMakeCurrent(hdc, opengl_handle) } != 0);

        println!("OpenGL context created!");

        let version = unsafe { glGetString(GL_VERSION) };
        assert!(!version.is_null());

        println!("OpenGL version: {}", unsafe {
            CStr::from_ptr(version as *const i8).to_str().unwrap()
        });

        unsafe {
            glDebugMessageCallback(gl_message_callback, std::ptr::null_mut());
        }
    }

    fn init_render_env(&mut self) {
        #[rustfmt::skip]
        let vertices: [Vec3; 4] = [
            vec3( -1.0,   1.0, 0.0), // top left
            vec3( -1.0,  -1.0, 0.0), // bottom left
            vec3(  1.0,  -1.0, 0.0), // bottom right
            vec3(  1.0,   1.0, 0.0), // top right
        ];

        #[rustfmt::skip]
        let colors: [Vec3; 4] = [
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
            vec3(1.0, 1.0, 1.0),
        ];

        #[rustfmt::skip]
        let uvs: [Vec2; 4] = [
            vec2(0.0, 0.0),
            vec2(0.0, 1.0),
            vec2(1.0, 1.0),
            vec2(1.0, 0.0),
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
        self.vao_id = vao;

        fn create_buffer() -> GLuint {
            let mut buffer = 0;
            unsafe {
                glGenBuffers(1, &mut buffer);
            }
            buffer
        }

        let vertex_buffer = create_buffer();
        let color_buffer = create_buffer();
        let uv_buffer = create_buffer();
        let index_buffer = create_buffer();

        self.index_buffer_id = index_buffer;

        unsafe {
            glBindVertexArray(vao);
            {
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
            }
            glBindVertexArray(0);
        }

        fn compile_shader_source(source: CString, type_: GLenum) -> GLuint {
            unsafe {
                let shader = glCreateShader(type_);

                glShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
                glCompileShader(shader);

                let mut success = true;
                glGetShaderiv(
                    shader,
                    GL_COMPILE_STATUS,
                    &mut success as *mut _ as *mut GLint,
                );

                if !success {
                    let mut info_log = vec![0; 512];

                    glGetShaderInfoLog(
                        shader,
                        512,
                        std::ptr::null_mut(),
                        info_log.as_mut_ptr() as *mut GLchar,
                    );

                    panic!(
                        "Failed to compile the {} shader! Error: {}",
                        shader_type_to_str(type_),
                        CStr::from_ptr(info_log.as_ptr()).to_str().unwrap()
                    );
                }

                shader
            }
        }

        let shader_program = {
            let vertex_shader =
                compile_shader_source(CString::new(VERTEX_SHADER).unwrap(), GL_VERTEX_SHADER);
            let fragment_shader =
                compile_shader_source(CString::new(FRAGMENT_SHADER).unwrap(), GL_FRAGMENT_SHADER);

            unsafe {
                let mut success = false;

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
                    // TODO: Print the linker error log
                    eprintln!("Failed to link the shader program!");
                }

                shader_program
            }
        };
        self.shader_program_id = shader_program;

        let view_matrix_uniform =
            unsafe { glGetUniformLocation(shader_program, c_str_ptr!("u_ViewMatrix")) };
        assert!(view_matrix_uniform != -1);

        self.view_matrix_uniform = view_matrix_uniform;

        let texture = unsafe {
            let mut texture = 0;

            glGenTextures(1, &mut texture);

            texture
        };

        self.texture_id = texture;

        unsafe {
            glBindTexture(GL_TEXTURE_2D, texture);

            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);

            let screenshot = self.screenshot.as_ref().unwrap();

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
    }

    fn init_imgui(&mut self, window: HWND) {
        let imgui = imgui::Context::create();

        unsafe {
            ImGui_ImplWin32_Init(window as *const c_void);
            ImGui_ImplOpenGL3_Init(c_str_ptr!("#version 330 core"));
        }

        self.imgui = Some(imgui);
    }

    fn take_screenshot(&mut self) {
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32 };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32 };
        let start_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let start_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };

        let start = std::time::Instant::now();

        self.screenshot = Some(take_screenshot(
            std::ptr::null_mut(),
            start_x,
            start_y,
            width,
            height,
        ));
        let screenshot = self.screenshot.as_ref().unwrap();

        println!(
            "Screenshot taken in {} seconds!",
            start.elapsed().as_secs_f32()
        );

        println!(
            "Screenshot size: {}x{} (AR = {})",
            screenshot.width(),
            screenshot.height(),
            screenshot.width() as f32 / screenshot.height() as f32
        );
    }

    pub fn on_resize(&mut self, new_client_width: u16, new_client_height: u16) {
        self.client_width = new_client_width as u32;
        self.client_height = new_client_height as u32;

        unsafe {
            glViewport(0, 0, self.client_width, self.client_height);
        }

        self.render();
    }

    pub fn on_left_mouse_down(&mut self, x: i32, y: i32) {
        self.last_mouse_pos = vec2(x as f32, y as f32);
    }

    /// Converts from screen pixel space ([0, client_width] x [0, client_height]) to normalized screen coordinates or NDC ([-1, 1] x [-1, 1])
    pub fn pixel_to_screen_coords(&self, pixel_coords: Vec2) -> Vec2 {
        vec2(
            pixel_coords.x / self.client_width as f32 * 2.0 - 1.0,
            -1.0 * (pixel_coords.y / self.client_height as f32 * 2.0 - 1.0),
        )
    }

    pub fn on_mouse_move(&mut self, x: i32, y: i32, left_mouse_down: bool) {
        if !left_mouse_down {
            return;
        }

        let mouse_pos = vec2(x as f32, y as f32);
        let delta = mouse_pos - self.last_mouse_pos;

        self.camera.translate(vec2(
            delta.x / self.client_width as f32 * 2.0,
            -1.0 * delta.y / self.client_height as f32 * 2.0,
        ));

        self.last_mouse_pos = mouse_pos;
    }

    pub fn on_mouse_wheel(&mut self, delta: i16, x: i32, y: i32) {
        let delta = 1.0 + delta as f32 / 120.0 / 10.0;

        let screen_space = self.pixel_to_screen_coords(vec2(x as f32, y as f32));
        let world_space = self.camera.screen_to_world_space_coords(screen_space);

        self.camera.zoom(delta, world_space);
    }

    pub fn render(&mut self) {
        let client_aspect_ratio = self.client_width as f32 / self.client_height as f32;
        let screenshot = self.screenshot.as_ref().unwrap();
        let screenshot_aspect_ratio = screenshot.width() as f32 / screenshot.height() as f32;
        let view_matrix = self.camera.to_homogenous()
            * Mat4::new_nonuniform_scaling(&vec3(
                1.0,
                client_aspect_ratio / screenshot_aspect_ratio,
                1.0,
            ));

        unsafe {
            glClear(GL_COLOR_BUFFER_BIT);

            glActiveTexture(GL_TEXTURE0);
            glBindTexture(GL_TEXTURE_2D, self.texture_id);
            glUseProgram(self.shader_program_id);

            {
                glUniformMatrix4fv(self.view_matrix_uniform, 1, false, view_matrix.as_ptr());

                glBindVertexArray(self.vao_id);
                glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, self.index_buffer_id);
                {
                    glDrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_BYTE, std::ptr::null());
                }
            }

            glUseProgram(0);
            glBindVertexArray(0);
            glBindTexture(GL_TEXTURE_2D, 0);
        }

        self.render_imgui();

        unsafe {
            SwapBuffers(self.hdc.unwrap());
        }
    }

    pub fn render_imgui(&mut self) {
        unsafe {
            ImGui_ImplOpenGL3_NewFrame();
            ImGui_ImplWin32_NewFrame();
        }

        let imgui = self.imgui.as_mut().unwrap();
        let ui = imgui.frame();

        ui.show_demo_window(&mut true);

        let draw_data = imgui.render();

        unsafe {
            ImGui_ImplOpenGL3_RenderDrawData(draw_data as *const _ as *mut _);
        }
    }
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
            debug_type_to_str(type_),
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

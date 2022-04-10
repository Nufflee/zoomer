use std::backtrace::Backtrace;
use std::ffi::c_void;
use std::{
    ffi::{CStr, CString},
    mem::{size_of, size_of_val},
};
use std::{fs, ptr};

use crate::camera::Camera;
use crate::ffi::c_str_ptr;
use crate::highlighter::Highlighter;
use crate::imgui_impl::*;
use crate::screenshot::take_screenshot;
use crate::{console, screenshot::Screenshot};
use crate::{gl::*, monitors};

use imgui::{Condition, FontConfig, FontSource};
use nalgebra_glm::{vec2, vec3, vec4, Mat4, Vec2, Vec3};
use winapi::um::winuser::{SetForegroundWindow, ShowWindow, SW_HIDE, SW_SHOW, VK_ESCAPE};
use winapi::{
    shared::windef::{HDC, HWND},
    um::{
        wingdi::*,
        winuser::{GetDC, VK_F2},
    },
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

uniform bool u_HighlighterOn;
uniform vec2 u_MousePosition;
uniform vec2 u_HighlighterRadius;

void main() {
    color = texture(u_Texture, v_TexCoord);

    // NOTE: This branch is statically uniform hence no divergence should happen and performance should be identical to 2 separate shaders
    if (u_HighlighterOn) {
        // Use the ellipse formula to create the highlighter circle due to varying aspect ratio (x^2/a^2 + y^2/b^2 = 1)
        vec2 distance = pow(v_TexCoord - u_MousePosition, vec2(2.0)) / pow(u_HighlighterRadius, vec2(2.0));

        // Use .rgb so we don't touch the alpha component.
        if (distance.x + distance.y < 1.0) {
            color.rgb = mix(color.rgb, vec3(1.0, 1.0, 1.0), 0.035);
        } else {
            color.rgb = mix(color.rgb, vec3(0.0, 0.0, 0.0), 0.55);
        }
    }
}
"#;

const DEBUG_GL_ERROR_BACKTRACE: bool = true;

pub struct Zoomer {
    pub client_width: u32,
    pub client_height: u32,

    window: Option<HWND>,
    hdc: Option<HDC>,
    imgui: Option<imgui::Context>,
    screenshot: Option<Screenshot>,
    /// Whether the zoomer window is currently open and showing.
    is_open: bool,

    vao_id: GLuint,
    texture_id: GLuint,
    index_buffer_id: GLuint,
    shader_program_id: GLuint,

    view_matrix_uniform: GLint,
    highlighter_radius_uniform: GLint,
    highlighter_on_uniform: GLint,
    mouse_position_uniform: GLint,

    debug_window_is_open: bool,

    highlighter: Highlighter,

    /// Current mouse position in pixel coordinate space.
    mouse_pos: Vec2,
    /// Last mouse position in screen coordinate space.
    last_mouse_screen_pos: Vec2,

    camera: Option<Camera>,
}

impl Zoomer {
    pub fn new() -> Self {
        Self {
            client_width: 0,
            client_height: 0,

            window: None,
            hdc: None,
            imgui: None,
            screenshot: None,
            is_open: false,

            vao_id: 0,
            texture_id: 0,
            index_buffer_id: 0,
            shader_program_id: 0,

            view_matrix_uniform: -1,
            highlighter_radius_uniform: -1,
            highlighter_on_uniform: -1,
            mouse_position_uniform: -1,

            debug_window_is_open: false,

            highlighter: Highlighter::new(),

            mouse_pos: Vec2::zeros(),
            last_mouse_screen_pos: Vec2::zeros(),

            camera: None,
        }
    }

    pub fn init(&mut self, window: HWND, client_width: i32, client_height: i32) {
        self.screenshot = Some(self.take_screenshot());

        self.client_width = client_width as u32;
        self.client_height = client_height as u32;

        self.window = Some(window);
        self.hdc = Some(unsafe { GetDC(window) });

        self.camera = Some(Camera::new(
            0.25..=500.0,
            vec2(1.0, self.aspect_ratio_ratio()),
        ));
        self.is_open = true;

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

        // Create and bind a dummy OpenGL context so we can load extension functions.
        // Reference: https://github.com/glfw/glfw/blob/4cb36872a5fe448c205d0b46f0e8c8b57530cfe0/src/wgl_context.c#L535
        let dummy_context = unsafe {
            let dummy_context = wglCreateContext(hdc);
            wglMakeCurrent(hdc, dummy_context);

            dummy_context
        };

        assert!(
            is_wgl_extension_supported(hdc, "WGL_ARB_create_context_profile"),
            "`WGL_ARB_create_context_profile` extension not supported"
        );

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
            CStr::from_ptr(version.cast()).to_str().unwrap()
        });

        unsafe {
            if DEBUG_GL_ERROR_BACKTRACE {
                // Debug output needs to be synchronized in order to obtain backtraces.
                glEnable(GL_DEBUG_OUTPUT_SYNCHRONOUS);
            }

            glDebugMessageCallback(gl_message_callback, std::ptr::null_mut());
        }
    }

    // TODO: clippy: this function has too many lines (211/100)
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
                        vertices.as_ptr().cast(),
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
                        colors.as_ptr().cast(),
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
                        uvs.as_ptr().cast(),
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
                        indices.as_ptr().cast(),
                        GL_STATIC_DRAW,
                    );
                }

                glBindBuffer(GL_ARRAY_BUFFER, 0);
            }
            glBindVertexArray(0);
        }

        fn compile_shader_source(source: &CString, type_: GLenum) -> GLuint {
            unsafe {
                let shader = glCreateShader(type_);

                glShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
                glCompileShader(shader);

                let mut success = true;
                glGetShaderiv(shader, GL_COMPILE_STATUS, ptr::addr_of_mut!(success).cast());

                if !success {
                    let mut info_log = vec![0; 512];

                    glGetShaderInfoLog(
                        shader,
                        512,
                        std::ptr::null_mut(),
                        info_log.as_mut_ptr().cast(),
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
                compile_shader_source(&CString::new(VERTEX_SHADER).unwrap(), GL_VERTEX_SHADER);
            let fragment_shader =
                compile_shader_source(&CString::new(FRAGMENT_SHADER).unwrap(), GL_FRAGMENT_SHADER);

            unsafe {
                // NOTE: This is an `i32` for alignment purposes. Using a `bool` with alignment of 1 could lead to an unaligned write as `glGetProgramiv` expects an `i32*`.
                let mut success: i32 = 0;

                let shader_program = glCreateProgram();

                glAttachShader(shader_program, vertex_shader);
                glAttachShader(shader_program, fragment_shader);
                glLinkProgram(shader_program);

                glGetProgramiv(
                    shader_program,
                    GL_LINK_STATUS,
                    ptr::addr_of_mut!(success).cast(),
                );

                if success == 0 {
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

        self.mouse_position_uniform =
            unsafe { glGetUniformLocation(shader_program, c_str_ptr!("u_MousePosition")) };
        assert!(self.mouse_position_uniform != -1);

        self.highlighter_radius_uniform =
            unsafe { glGetUniformLocation(shader_program, c_str_ptr!("u_HighlighterRadius")) };
        assert!(self.highlighter_radius_uniform != -1);

        self.highlighter_on_uniform =
            unsafe { glGetUniformLocation(shader_program, c_str_ptr!("u_HighlighterOn")) };
        assert!(self.highlighter_on_uniform != -1);

        let texture = unsafe {
            let mut texture = 0;

            glGenTextures(1, &mut texture);

            texture
        };

        self.texture_id = texture;

        self.upload_screenshot_to_gpu();

        unsafe {
            glEnable(GL_BLEND);

            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        }
    }

    fn init_imgui(&mut self, window: HWND) {
        let imgui = imgui::Context::create();

        unsafe {
            ImGui_ImplWin32_Init(window as *const c_void);
            ImGui_ImplOpenGL3_Init(c_str_ptr!("#version 330 core"));
        }

        self.imgui = Some(imgui);
        let imgui = self.imgui.as_mut().unwrap();

        let maybe_font_data = fs::read("C:\\Windows\\Fonts\\FiraCode-Regular.ttf").ok();
        let font = maybe_font_data.as_ref().map_or_else(
            || FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: 19.0,
                    ..Default::default()
                }),
            },
            |font_data| FontSource::TtfData {
                data: font_data,
                size_pixels: 19.0,
                config: None,
            },
        );

        imgui.fonts().add_font(&[font]);
        imgui.set_ini_filename(None);

        let style = imgui.style_mut();
        style.item_spacing = [15.0, 7.5];
        style.window_rounding = 5.0;

        self.debug_window_is_open = true;
    }

    fn take_screenshot(&mut self) -> Screenshot {
        let monitors = monitors::enumerate();

        assert!(!monitors.is_empty(), "no monitors found");

        let (start_x, start_y) = monitors.iter().fold((0, 0), |min_start, monitor| {
            (monitor.x.min(min_start.0), monitor.y.min(min_start.1))
        });

        let width: u32 = monitors.iter().map(|monitor| monitor.width).sum();
        let height = monitors.iter().map(|monitor| monitor.height).max().unwrap();

        let timer = std::time::Instant::now();

        let screenshot = take_screenshot(
            std::ptr::null_mut(),
            start_x,
            start_y,
            width as u32,
            height as u32,
        );

        println!(
            "Screenshot taken in {} seconds",
            timer.elapsed().as_secs_f32()
        );

        screenshot
    }

    fn upload_screenshot_to_gpu(&mut self) {
        let screenshot = self.screenshot.as_mut().unwrap();

        unsafe {
            glBindTexture(GL_TEXTURE_2D, self.texture_id);

            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);

            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA,
                screenshot.width(),
                screenshot.height(),
                0,
                GL_RGBA as GLenum,
                GL_UNSIGNED_BYTE,
                screenshot.take_pixel_bytes().as_ptr().cast(),
            );

            glBindTexture(GL_TEXTURE_2D, 0);
        }
    }

    pub fn on_resize(&mut self, new_client_width: u16, new_client_height: u16) {
        self.client_width = new_client_width as u32;
        self.client_height = new_client_height as u32;

        unsafe {
            glViewport(0, 0, self.client_width, self.client_height);
        }
    }

    /// Converts from screen pixel space ([0, `client_width`] x [0, `client_height`]) to normalized screen coordinates or NDC ([-1, 1] x [-1, 1])
    pub fn pixel_to_screen_space(&self, pixel_coords: Vec2) -> Vec2 {
        vec2(
            pixel_coords.x / self.client_width as f32 * 2.0 - 1.0,
            -1.0 * (pixel_coords.y / self.client_height as f32 * 2.0 - 1.0),
        )
    }

    pub fn pixel_to_uv_space(&self, pixel_coords: Vec2) -> Vec2 {
        let mut mouse_uv_pos = self
            .camera
            .as_ref()
            .unwrap()
            .screen_to_world_space(self.pixel_to_screen_space(pixel_coords));

        mouse_uv_pos.y *= -1.0 / self.aspect_ratio_ratio();
        mouse_uv_pos += vec2(1.0, 1.0);
        mouse_uv_pos /= 2.0;

        mouse_uv_pos
    }

    pub fn on_left_mouse_down(&mut self, x: i32, y: i32) {
        self.last_mouse_screen_pos = self.pixel_to_screen_space(vec2(x as f32, y as f32));
    }

    pub fn on_left_mouse_up(&mut self) {
        self.camera.as_mut().unwrap().clamp_me_daddy();
    }

    pub fn on_mouse_move(&mut self, x: i32, y: i32, left_mouse_down: bool) {
        self.mouse_pos = vec2(x as f32, y as f32);

        if !left_mouse_down {
            return;
        }

        let mouse_screen_pos = self.pixel_to_screen_space(self.mouse_pos);
        let delta = mouse_screen_pos - self.last_mouse_screen_pos;

        self.camera.as_mut().unwrap().translate(delta);

        self.last_mouse_screen_pos = mouse_screen_pos;
    }

    pub fn on_mouse_wheel(&mut self, delta: i16, x: i32, y: i32, ctrl_is_down: bool) {
        let delta = delta as f32 / 120.0 / 10.0;

        if ctrl_is_down && self.highlighter.is_enabled() {
            self.highlighter
                .set_radius(self.highlighter.radius() * (1.0 + delta * 2.0));

            return;
        }

        let screen_point = self.pixel_to_screen_space(vec2(x as f32, y as f32));

        let camera = self.camera.as_mut().unwrap();

        camera.zoom(1.0 + delta, screen_point);
    }

    pub fn on_key_down(&mut self, key: u8) {
        if key == VK_F2 as u8 {
            self.debug_window_is_open = !self.debug_window_is_open;
        }

        if key == b'C' {
            self.highlighter.set_enabled(!self.highlighter.is_enabled());

            unsafe {
                glUseProgram(self.shader_program_id);
                glUniform1i(
                    self.highlighter_on_uniform,
                    self.highlighter.is_enabled() as i32,
                );
                glUseProgram(0);
            }
        }

        if key == VK_ESCAPE as u8 {
            self.is_open = false;

            unsafe { ShowWindow(self.window.unwrap(), SW_HIDE) };
        }
    }

    pub fn on_hotkey(&mut self) {
        if self.is_open {
            return;
        }

        self.screenshot = Some(self.take_screenshot());
        self.upload_screenshot_to_gpu();

        let window = self.window.unwrap();

        unsafe {
            ShowWindow(window, SW_SHOW);
            // NOTE: This is not strictly required, but just in case.
            SetForegroundWindow(window);
        }

        self.is_open = true;
    }

    pub fn screenshot_aspect_ratio(&self) -> f32 {
        let screenshot = self.screenshot.as_ref().unwrap();

        screenshot.width() as f32 / screenshot.height() as f32
    }

    /// Returns the ratio of the client aspect ratio to the screenshot aspect ratio
    pub fn aspect_ratio_ratio(&self) -> f32 {
        let client_aspect_ratio = self.client_width as f32 / self.client_height as f32;
        let screenshot_aspect_ratio = self.screenshot_aspect_ratio();

        client_aspect_ratio / screenshot_aspect_ratio
    }

    pub fn update(&mut self, dt: f32) {
        self.camera.as_mut().unwrap().update(dt);
        self.highlighter.update(dt);

        let mouse_uv_pos = self.pixel_to_uv_space(self.mouse_pos);

        unsafe {
            glUseProgram(self.shader_program_id);
            glUniform2fv(
                self.mouse_position_uniform,
                1,
                vec4(mouse_uv_pos.x, mouse_uv_pos.y, 0.0, 1.0).as_ptr(),
            );
            glUseProgram(0);
        }

        let radius_uv =
            vec2(self.highlighter.radius(), self.highlighter.radius()).component_mul(&vec2(
                1.0 / self.client_width as f32,
                1.0 / self.client_height as f32,
            ));

        let highlighter_radius_uv = vec2(radius_uv.x, radius_uv.y / self.aspect_ratio_ratio());

        unsafe {
            glUseProgram(self.shader_program_id);
            glUniform2fv(
                self.highlighter_radius_uniform,
                1,
                highlighter_radius_uv.as_ptr(),
            );
            glUseProgram(0);
        }
    }

    pub fn render(&mut self) {
        let view_matrix = self.camera.as_ref().unwrap().to_homogenous()
            * Mat4::new_nonuniform_scaling(&vec3(1.0, self.aspect_ratio_ratio(), 1.0));

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

        let screen_space = self.pixel_to_screen_space(self.mouse_pos);
        let uv_space = self.pixel_to_uv_space(self.mouse_pos);

        let camera = self.camera.as_mut().unwrap();

        let camera_space = camera.screen_to_camera_space(screen_space);
        let world_space = camera.screen_to_world_space(screen_space);

        let imgui = self.imgui.as_mut().unwrap();
        let ui = imgui.frame();

        if self.debug_window_is_open {
            ui.window("Debug")
                .size([650.0, 0.0], Condition::FirstUseEver)
                .resizable(false)
                .build(|| {
                    ui.text(format!(
                        "Mouse pixel space position = ({}, {})",
                        self.mouse_pos.x, self.mouse_pos.y,
                    ));
                    ui.text(format!(
                        "Mouse screen space position = ({:.4}, {:.4})",
                        screen_space.x, screen_space.y,
                    ));
                    ui.text(format!(
                        "Mouse world space position = ({:.4}, {:.4})",
                        world_space.x, world_space.y,
                    ));
                    ui.text(format!(
                        "Mouse camera space position = ({:.4}, {:.4})",
                        camera_space.x, camera_space.y,
                    ));
                    ui.text(format!(
                        "Mouse UV space position = ({:.4}, {:.4})",
                        uv_space.x, uv_space.y
                    ));

                    ui.separator();

                    ui.text(format!(
                        "Camera position = ({:.4}, {:.4})",
                        camera.position().x,
                        camera.position().y
                    ));
                });
        }

        let draw_data = imgui.render();

        unsafe {
            ImGui_ImplOpenGL3_RenderDrawData(draw_data as *const _ as *mut _);
        }
    }

    /// Whether ImGui wants to receive mouse events instead of the application (ie. mouse is over an ImGui window)
    pub fn imgui_wants_mouse_events(&self) -> bool {
        self.imgui.as_ref().unwrap().io().want_capture_mouse
    }

    /// Whether ImGui wants to receive keyboard events instead of the application
    pub fn imgui_wants_keyboard_events(&self) -> bool {
        self.imgui.as_ref().unwrap().io().want_capture_keyboard
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
    use console::{Color, SimpleColor};

    if severity == GL_DEBUG_SEVERITY_NOTIFICATION {
        return;
    }

    let message = CStr::from_ptr(message);
    let message = message.to_string_lossy();

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

    if DEBUG_GL_ERROR_BACKTRACE && type_ == GL_DEBUG_TYPE_ERROR {
        eprintln!("{}", Backtrace::force_capture());
    }

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

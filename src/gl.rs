#![allow(non_upper_case_globals, non_snake_case, dead_code)]

use crate::ffi::c_str_ptr;
use std::ffi::c_void;
use std::mem::transmute;
use std::os::raw::c_char;
use winapi::shared::windef::{HDC, HGLRC};
use winapi::um::wingdi::wglGetProcAddress;

pub type GLuint = u32;
pub type GLsizei = u32;
pub type GLenum = u32;
pub type GLubyte = u8;
pub type GLfloat = f32;
pub type GLint = i32;
pub type GLboolean = bool;
pub type GLclampf = GLfloat;
pub type GLbitfield = u32;
pub type GLvoid = c_void;
pub type GLchar = c_char;

// glGetString
pub const GL_VERSION: GLenum = 0x1F02;

// glGetError
pub const GL_NO_ERROR: GLenum = 0;

// glClear
pub const GL_COLOR_BUFFER_BIT: GLbitfield = 0x00004000;
pub const GL_DEPTH_BUFFER_BIT: GLbitfield = 0x00000100;
pub const GL_STENCIL_BUFFER_BIT: GLbitfield = 0x00000400;

// glBindBuffer
pub const GL_ARRAY_BUFFER: GLenum = 0x8892;
pub const GL_ELEMENT_ARRAY_BUFFER: GLenum = 0x8893;

// glBufferData
pub const GL_STATIC_DRAW: GLenum = 0x88E4;

// glDrawArrays
pub const GL_TRIANGLES: GLenum = 0x0004;

pub const GL_FLOAT: GLenum = 0x1406;

// glCreateShader
pub const GL_FRAGMENT_SHADER: GLenum = 0x8B30;
pub const GL_VERTEX_SHADER: GLenum = 0x8B31;

// glGetShaderiv
pub const GL_COMPILE_STATUS: GLenum = 0x8B81;

// glGetProgramiv
pub const GL_LINK_STATUS: GLenum = 0x8B82;

// glBindTexture
pub const GL_TEXTURE_2D: GLenum = 0x0DE1;

// glTexImage2D
pub const GL_UNSIGNED_BYTE: GLenum = 0x1401;

pub const GL_RGB: GLint = 0x1907;
pub const GL_RGBA: GLint = 0x1908;

// glTextureParameteri
pub const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;
pub const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
pub const GL_TEXTURE_WRAP_S: GLenum = 0x2802;
pub const GL_TEXTURE_WRAP_T: GLenum = 0x2803;

pub const GL_NEAREST: GLint = 0x2600;
pub const GL_LINEAR: GLint = 0x2601;
pub const GL_LINEAR_MIPMAP_LINEAR: GLint = 0x2703;

pub const GL_CLAMP_TO_EDGE: GLint = 0x812F;

// glActiveTexture
pub const GL_TEXTURE0: GLenum = 0x84C0;

// wglCreateContextAttribsARB
pub const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
pub const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;

pub const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;
pub const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x00000001;

pub const WGL_CONTEXT_FLAGS_ARB: i32 = 0x2094;
pub const WGL_CONTEXT_DEBUG_BIT_ARB: i32 = 0x0001;

// glDebugMessageCallback
pub const GL_DEBUG_TYPE_ERROR: GLenum = 0x824C;
pub const GL_DEBUG_TYPE_DEPRECATED_BEHAVIOR: GLenum = 0x824D;
pub const GL_DEBUG_TYPE_UNDEFINED_BEHAVIOR: GLenum = 0x824E;
pub const GL_DEBUG_TYPE_PORTABILITY: GLenum = 0x824F;
pub const GL_DEBUG_TYPE_PERFORMANCE: GLenum = 0x8250;
pub const GL_DEBUG_TYPE_OTHER: GLenum = 0x8251;

pub const GL_DEBUG_SEVERITY_HIGH: GLenum = 0x9146;
pub const GL_DEBUG_SEVERITY_MEDIUM: GLenum = 0x9147;
pub const GL_DEBUG_SEVERITY_LOW: GLenum = 0x9148;
pub const GL_DEBUG_SEVERITY_NOTIFICATION: GLenum = 0x826B;

pub fn shader_type_to_str(type_: GLenum) -> &'static str {
    match type_ {
        GL_VERTEX_SHADER => "vertex",
        GL_FRAGMENT_SHADER => "fragment",
        _ => unreachable!(),
    }
}

pub fn debug_type_to_str(type_: GLenum) -> &'static str {
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

// https://www.khronos.org/registry/OpenGL/api/GL/glcorearb.h
extern "C" {
    pub fn glGetString(name: GLenum) -> *const GLubyte;
    pub fn glGetError() -> GLenum;

    pub fn glClearColor(red: GLclampf, green: GLclampf, blue: GLclampf, alpha: GLclampf);
    pub fn glClear(mask: GLbitfield);

    pub fn glViewport(x: GLint, y: GLint, width: GLsizei, height: GLsizei);

    pub fn glTexImage2D(
        target: GLenum,
        level: GLint,
        internalFormat: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    );
    pub fn glTexParameteri(target: GLenum, pname: GLenum, param: GLint);
}

macro_rules! declare_opengl_function {
    (fn $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) $(,)?) => {
        declare_opengl_function!(fn $name($($arg: $arg_ty),*) -> ());
    };
    (fn $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $return_type:ty $(,)?) => {
        #[inline(always)]
        pub unsafe fn $name($($arg: $arg_ty),*) -> $return_type {
            use std::sync::Once;

            static INIT: Once = Once::new();
            static mut FPTR: Option<extern "C" fn ($($arg: $arg_ty),*) -> $return_type> = None;

            INIT.call_once(|| {
                let func = wglGetProcAddress(c_str_ptr!(stringify!($name)));
                assert!(!func.is_null(), "unable to load OpenGL/WGL function `{}`", stringify!($name));
                FPTR = transmute::<_, _>(func)
            });

            FPTR.unwrap()($($arg),*)
        }
    };
}

declare_opengl_function!(fn glGenBuffers(n: GLsizei, buffers: *mut GLuint));
declare_opengl_function!(fn glBindBuffer(target: GLenum, buffer: GLuint));
declare_opengl_function!(fn glBufferData(target: GLenum,size: GLsizei,data: *const GLvoid, usage: GLenum));
declare_opengl_function!(
    fn glVertexAttribPointer(
        index: GLuint,
        size: GLint,
        type_: GLenum,
        normalized: GLboolean,
        stride: GLsizei,
        pointer: *const GLvoid,
    )
);

declare_opengl_function!(fn glEnableVertexAttribArray(index: GLuint));
declare_opengl_function!(fn glGenVertexArrays(n: GLsizei, arrays: *mut GLuint));
declare_opengl_function!(fn glBindVertexArray(array: GLuint));

declare_opengl_function!(fn glDrawArrays(mode: GLenum, first: GLint, count: GLsizei));
declare_opengl_function!(
    fn glDrawElements(
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    )
);

declare_opengl_function!(fn glCreateShader(shaderType: GLenum) -> GLuint);
declare_opengl_function!(
    fn glShaderSource(
        shader: GLuint,
        count: GLsizei,
        string: *const *const GLchar,
        length: *const GLint,
    )
);
declare_opengl_function!(fn glCompileShader(shader: GLuint));
declare_opengl_function!(fn glGetShaderiv(shader: GLuint, pname: GLenum, params: *mut GLint));
declare_opengl_function!(fn glCreateProgram() -> GLuint);
declare_opengl_function!(fn glAttachShader(program: GLuint, shader: GLuint));
declare_opengl_function!(fn glLinkProgram(program: GLuint));
declare_opengl_function!(fn glGetProgramiv(program: GLuint, pname: GLenum, params: *mut GLint));
declare_opengl_function!(fn glUseProgram(program: GLuint));
declare_opengl_function!(fn glGetUniformLocation(program: GLuint, name: *const GLchar) -> GLint);
declare_opengl_function!(fn glUniform1i(location: GLint, v0: GLint));
declare_opengl_function!(
    fn glUniformMatrix4fv(
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    )
);
declare_opengl_function!(
    fn glGetShaderInfoLog(
        shader: GLuint,
        maxLength: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    )
);

declare_opengl_function!(fn glEnable(cap: GLenum));

declare_opengl_function!(fn glGenTextures(n: GLsizei, textures: *mut GLuint));
declare_opengl_function!(fn glBindTexture(target: GLenum, texture: GLuint));
declare_opengl_function!(fn glActiveTexture(texture: GLenum));
declare_opengl_function!(fn glGenerateMipmap(target: GLenum));

#[allow(clippy::upper_case_acronyms)]
type DEBUGPROC = unsafe extern "C" fn(
    source: GLenum,
    type_: GLenum,
    id: GLuint,
    severity: GLenum,
    length: GLsizei,
    message: *const GLchar,
    userParam: *mut GLvoid,
);
declare_opengl_function!(fn glDebugMessageCallback(callback: DEBUGPROC, userParam: *mut c_void));

// https://www.khronos.org/registry/OpenGL/extensions/ARB/WGL_ARB_create_context.txt
declare_opengl_function!(
    fn wglCreateContextAttribsARB(hdc: HDC, shareContext: HGLRC, attribList: *const i32) -> HGLRC
);
// https://www.khronos.org/registry/OpenGL/extensions/ARB/WGL_ARB_extensions_string.txt
declare_opengl_function!(
    fn wglGetExtensionsStringARB(hdc: HDC) -> *const GLchar,
);
// https://www.khronos.org/registry/OpenGL/extensions/EXT/WGL_EXT_swap_control.txt
declare_opengl_function!(fn wglSwapIntervalEXT(interval: i32) -> i32);
declare_opengl_function!(fn wglGetSwapIntervalEXT() -> i32);

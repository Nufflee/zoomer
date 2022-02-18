#![allow(non_upper_case_globals)]

use crate::ffi::c_str_ptr;
use lazy_static::lazy_static;
use std::ffi::c_void;
use std::mem::transmute;
use std::os::raw::c_char;
use winapi::shared::windef::{HDC, HGLRC};
use winapi::um::wingdi::wglGetProcAddress;

macro_rules! load_opengl_function {
    ($type:ty, $name:ident) => {
        load_opengl_function!(
            $type,
            $name,
            "unable to load OpenGL/WGL function `{}`",
            stringify!($name)
        );
    };
    ($type:ty, $name:ident, $($arg: expr),+) => {
        lazy_static! {
            pub static ref $name: $type = unsafe {
                let func = wglGetProcAddress(c_str_ptr!(stringify!($name)));
                assert!(!func.is_null(), $($arg),+);
                transmute::<_, _>(func)
            };
        }
    };
}

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

// glClear
pub const GL_COLOR_BUFFER_BIT: GLbitfield = 0x00004000;
pub const GL_DEPTH_BUFFER_BIT: GLbitfield = 0x00000100;
pub const GL_STENCIL_BUFFER_BIT: GLbitfield = 0x00000400;

// glBindBuffer
pub const GL_ARRAY_BUFFER: GLenum = 0x8892;

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

// wglCreateContextAttribsARB
pub const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
pub const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;

pub const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;
pub const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x00000001;

// https://www.khronos.org/registry/OpenGL/api/GL/glcorearb.h
extern "C" {
    pub fn glGetString(name: GLenum) -> *const GLubyte;

    pub fn glClearColor(red: GLclampf, green: GLclampf, blue: GLclampf, alpha: GLclampf);
    pub fn glClear(mask: GLbitfield);

    pub fn glViewport(x: GLint, y: GLint, width: GLsizei, height: GLsizei);
}

load_opengl_function!(
    unsafe extern "C" fn(n: GLsizei, buffers: *mut GLuint),
    glGenBuffers
);
load_opengl_function!(
    unsafe extern "C" fn(target: GLenum, buffer: GLuint),
    glBindBuffer
);
load_opengl_function!(
    unsafe extern "C" fn(target: GLenum, size: GLsizei, data: *const GLvoid, usage: GLenum),
    glBufferData
);
load_opengl_function!(
    unsafe extern "C" fn(
        index: GLuint,
        size: GLint,
        type_: GLenum,
        normalized: GLboolean,
        stride: GLsizei,
        pointer: *const GLvoid,
    ),
    glVertexAttribPointer
);
load_opengl_function!(
    unsafe extern "C" fn(index: GLuint),
    glEnableVertexAttribArray
);
load_opengl_function!(
    unsafe extern "C" fn(n: GLsizei, arrays: *const GLuint),
    glGenVertexArrays
);
load_opengl_function!(unsafe extern "C" fn(array: GLuint), glBindVertexArray);
load_opengl_function!(
    unsafe extern "C" fn(mode: GLenum, first: GLint, count: GLsizei),
    glDrawArrays
);
load_opengl_function!(
    unsafe extern "C" fn(shaderType: GLenum) -> GLuint,
    glCreateShader
);
load_opengl_function!(
    unsafe extern "C" fn(
        shader: GLuint,
        count: GLsizei,
        string: *const *const GLchar,
        length: *const GLint,
    ),
    glShaderSource
);
load_opengl_function!(unsafe extern "C" fn(shader: GLuint), glCompileShader);
load_opengl_function!(
    unsafe extern "C" fn(shader: GLuint, pname: GLenum, params: *mut GLint),
    glGetShaderiv
);
load_opengl_function!(unsafe extern "C" fn() -> GLuint, glCreateProgram);
load_opengl_function!(
    unsafe extern "C" fn(program: GLuint, shader: GLuint),
    glAttachShader
);
load_opengl_function!(unsafe extern "C" fn(program: GLuint), glLinkProgram);
load_opengl_function!(
    unsafe extern "C" fn(program: GLuint, pname: GLenum, params: *mut GLint),
    glGetProgramiv
);
load_opengl_function!(unsafe extern "C" fn(program: GLuint), glUseProgram);
load_opengl_function!(
    unsafe extern "C" fn(program: GLuint, name: *const GLchar) -> GLint,
    glGetUniformLocation
);
load_opengl_function!(
    unsafe extern "C" fn(
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ),
    glUniformMatrix4fv
);
load_opengl_function!(extern "C" fn(cap: GLenum), glEanble);

// https://www.khronos.org/registry/OpenGL/extensions/ARB/WGL_ARB_create_context.txt
load_opengl_function!(
    unsafe extern "C" fn(hdc: HDC, shareContext: HGLRC, attribList: *const i32) -> HGLRC,
    wglCreateContextAttribsARB
);
// https://www.khronos.org/registry/OpenGL/extensions/ARB/WGL_ARB_extensions_string.txt
load_opengl_function!(
    unsafe extern "C" fn(hdc: HDC) -> *const GLchar,
    wglGetExtensionsStringARB,
    "`WGL_ARB_extensions_string` extension not supported"
);

use std::mem::size_of;

use num_traits::PrimInt;
use winapi::{
    ctypes::c_void,
    shared::windef::{HGDIOBJ, HWND},
    um::{
        wingdi::*,
        winnt::HANDLE,
        winuser::{GetDC, ReleaseDC},
    },
};

// An RGBA screenshot.
pub struct Screenshot {
    width: u32,
    height: u32,
    /// Pixel bytes are taken out of the screenshot and deallocated after being transferred to the GPU
    pixel_bytes: Option<Vec<u8>>,
    /// Width stride in *bytes*.
    stride: u32,
}

impl Screenshot {
    pub const BYTES_PER_PIXEL: u32 = 4;

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn take_pixel_bytes(&mut self) -> Vec<u8> {
        self.pixel_bytes
            .take()
            .expect("screenshot pixel bytes were already taken")
    }

    pub fn stride(&self) -> u32 {
        self.stride
    }
}

/// Take a screenshot of the given window handle constrained by the given rectangle. Returns a slice of BGR pixels and its width stride as a tuple.
pub fn take_screenshot(
    handle: HWND,
    start_x: i32,
    start_y: i32,
    width: u32,
    height: u32,
) -> Screenshot {
    unsafe {
        let window_dc = GetDC(handle);
        assert!(!window_dc.is_null());

        let memory_dc = CreateCompatibleDC(window_dc);
        assert!(!memory_dc.is_null());

        let bitmap_handle = CreateCompatibleBitmap(window_dc, width as i32, height as i32);
        assert!(!bitmap_handle.is_null());

        let ret = SelectObject(memory_dc, bitmap_handle as HGDIOBJ);
        assert!(!ret.is_null() && ret != HGDI_ERROR);

        let ret = BitBlt(
            memory_dc,
            0,
            0,
            width as i32,
            height as i32,
            window_dc,
            start_x,
            start_y,
            SRCCOPY,
        );
        assert!(ret != 0);

        let mut bitmap = BITMAP::default();

        let ret = GetObjectA(
            bitmap_handle as HANDLE,
            size_of::<BITMAP>() as i32,
            &mut bitmap as *mut _ as HANDLE,
        );
        assert!(ret != 0);

        let stride = round_up_to_power_of_2(width as u32 * Screenshot::BYTES_PER_PIXEL, 4);
        let bitmap_size = stride * height as u32;

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: bitmap.bmPlanes,
                biBitCount: (Screenshot::BYTES_PER_PIXEL * 8) as u16,
                biCompression: BI_RGB,
                biSizeImage: bitmap_size,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixel_bytes = vec![0u8; bitmap_size as usize];

        let ret = GetDIBits(
            memory_dc,
            bitmap_handle,
            0,
            height as u32,
            pixel_bytes.as_mut_ptr() as *mut c_void,
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );
        assert!(ret != 0);

        ReleaseDC(handle, window_dc);
        DeleteDC(memory_dc);
        DeleteObject(bitmap_handle as *mut c_void);

        assert!(pixel_bytes.len() % 3 == 0);

        // Convert from BGRA to RGBA.
        let padding_per_row = stride - width * Screenshot::BYTES_PER_PIXEL;

        for y in 0..height {
            for x in 0..width {
                let padding = padding_per_row * y;
                let pixel_index =
                    ((x + y * width) * Screenshot::BYTES_PER_PIXEL + padding) as usize;

                let b = pixel_bytes[pixel_index];
                let g = pixel_bytes[pixel_index + 1];
                let r = pixel_bytes[pixel_index + 2];
                let a = pixel_bytes[pixel_index + 3];

                pixel_bytes[pixel_index] = r;
                pixel_bytes[pixel_index + 1] = g;
                pixel_bytes[pixel_index + 2] = b;
                pixel_bytes[pixel_index + 3] = a;
            }
        }

        Screenshot {
            width,
            height,
            pixel_bytes: Some(pixel_bytes),
            stride,
        }
    }
}

fn round_up_to_power_of_2<T: PrimInt>(value: T, power: T) -> T {
    // Make sure `value` is a power of 2
    debug_assert!(power.count_ones() == 1);

    (value + (power - T::one())) & (!(power - T::one()))
}

#[cfg(test)]
mod test {
    use super::round_up_to_power_of_2;

    #[test]
    fn test_round_up_to_power_of_2() {
        assert_eq!(round_up_to_power_of_2(0, 4), 0);
        assert_eq!(round_up_to_power_of_2(3, 4), 4);
        assert_eq!(round_up_to_power_of_2(1, 4), 4);
        assert_eq!(round_up_to_power_of_2(69, 4), 72);
    }

    #[test]
    #[should_panic]
    fn test_round_up_to_power_of_2_panic() {
        round_up_to_power_of_2(1, 3);
    }
}

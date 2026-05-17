use std::mem::size_of;
use winapi::{
    shared::windef::{HBITMAP, HDC},
    um::wingdi::{
        BitBlt,
        CreateCompatibleBitmap,
        CreateCompatibleDC,
        CreateDCW,
        DeleteDC,
        DeleteObject,
        GetDIBits,
        SelectObject,
        BITMAPINFO,
        BITMAPINFOHEADER,
        BI_RGB,
        CAPTUREBLT,
        DIB_RGB_COLORS, //CAPTUREBLT,
        HGDI_ERROR,
        RGBQUAD,
        SRCCOPY,
    },
};

const PIXEL_WIDTH: i32 = 4;

pub struct CapturerGDI {
    screen_dc: HDC,
    dc: HDC,
    bmp: HBITMAP,
    width: i32,
    height: i32,
    pub capture_scale: f32,
    scaled_buffer: Vec<u8>,
}

impl CapturerGDI {
    pub fn new_with_scale(
        name: &[u16],
        width: i32,
        height: i32,
        capture_scale: f32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            if name.is_empty() {
                return Err("Empty display name".into());
            }
            let screen_dc = CreateDCW(&name[0], 0 as _, 0 as _, 0 as _);
            if screen_dc.is_null() {
                return Err("Failed to create dc from monitor name".into());
            }

            // Create a Windows Bitmap, and copy the bits into it
            let dc = CreateCompatibleDC(screen_dc);
            if dc.is_null() {
                DeleteDC(screen_dc);
                return Err("Can't get a Windows display".into());
            }

            let bmp = CreateCompatibleBitmap(screen_dc, width, height);
            if bmp.is_null() {
                DeleteDC(screen_dc);
                DeleteDC(dc);
                return Err("Can't create a Windows buffer".into());
            }

            let res = SelectObject(dc, bmp as _);
            if res.is_null() || res == HGDI_ERROR {
                DeleteDC(screen_dc);
                DeleteDC(dc);
                DeleteObject(bmp as _);
                return Err("Can't select Windows buffer".into());
            }
            Ok(Self {
                screen_dc,
                dc,
                bmp,
                width,
                height,
                capture_scale,
                scaled_buffer: Vec::new(),
            })
        }
    }

    pub fn new(name: &[u16], width: i32, height: i32) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_scale(name, width, height, 1.0)
    }

    pub fn frame(&mut self, data: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let res = BitBlt(
                self.dc,
                0,
                0,
                self.width,
                self.height,
                self.screen_dc,
                0,
                0,
                SRCCOPY | CAPTUREBLT, // CAPTUREBLT enable layered window but also make cursor blinking
            );
            if res == 0 {
                return Err("Failed to copy screen to Windows buffer".into());
            }

            let stride = self.width * PIXEL_WIDTH;
            let size: usize = (stride * self.height) as usize;
            let mut data1: Vec<u8> = Vec::with_capacity(size);
            data1.set_len(size);
            data.resize(size, 0);

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as _,
                    biWidth: self.width as _,
                    biHeight: self.height as _,
                    biPlanes: 1,
                    biBitCount: (8 * PIXEL_WIDTH) as _,
                    biCompression: BI_RGB,
                    biSizeImage: (self.width * self.height * PIXEL_WIDTH) as _,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };

            // copy bits into Vec
            let res = GetDIBits(
                self.dc,
                self.bmp,
                0,
                self.height as _,
                &mut data[0] as *mut u8 as _,
                &mut bmi as _,
                DIB_RGB_COLORS,
            );
            if res == 0 {
                return Err("GetDIBits failed".into());
            }
            crate::common::ARGBMirror(
                data.as_ptr(),
                stride as _,
                data1.as_mut_ptr(),
                stride as _,
                self.width as _,
                self.height as _,
            );
            crate::common::ARGBRotate(
                data1.as_ptr(),
                stride as _,
                data.as_mut_ptr(),
                stride as _,
                self.width as _,
                self.height as _,
                crate::RotationMode::kRotate180,
            );

            if (self.capture_scale - 1.0).abs() > 0.001 {
                let scaled_width = (self.width as f32 * self.capture_scale).round() as i32;
                let scaled_height = (self.height as f32 * self.capture_scale).round() as i32;
                self.scaled_buffer
                    .resize((scaled_width * PIXEL_WIDTH * scaled_height) as usize, 0);
                crate::common::ARGBScale(
                    data.as_ptr(),
                    stride as _,
                    self.width as _,
                    self.height as _,
                    self.scaled_buffer.as_mut_ptr(),
                    (scaled_width * PIXEL_WIDTH) as _,
                    scaled_width as _,
                    scaled_height as _,
                    crate::common::FilterMode::kFilterLinear,
                );
                data.resize(self.scaled_buffer.len(), 0);
                data.copy_from_slice(&self.scaled_buffer);
            }

            Ok(())
        }
    }
}

impl Drop for CapturerGDI {
    fn drop(&mut self) {
        unsafe {
            DeleteDC(self.screen_dc);
            DeleteDC(self.dc);
            DeleteObject(self.bmp as _);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    #[test]
    fn test() {
        match Displays::new().unwrap().next() {
            Some(d) => {
                let w = d.width();
                let h = d.height();
                let c = CapturerGDI::new(d.name(), w, h).unwrap();
                let mut data = Vec::new();
                c.frame(&mut data).unwrap();
                let mut bitflipped = Vec::with_capacity((w * h * 4) as usize);
                for y in 0..h {
                    for x in 0..w {
                        let i = (w * 4 * y + 4 * x) as usize;
                        bitflipped.extend_from_slice(&[data[i + 2], data[i + 1], data[i], 255]);
                    }
                }
                repng::encode(
                    std::fs::File::create("gdi_screen.png").unwrap(),
                    d.width() as u32,
                    d.height() as u32,
                    &bitflipped,
                )
                .unwrap();
            }
            _ => {
                assert!(false);
            }
        }
    }
}

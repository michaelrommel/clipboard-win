//!Image clipboard support for various formats
//!
//!## General information
//!
use super::{EnumFormats, format_name, get_vec, size, get_clipboard_data };

use winapi::um::wingdi::{BITMAPFILEHEADER, BITMAPV5HEADER};
use winapi::um::winbase::{GlobalLock, GlobalUnlock};
use winapi::ctypes::{c_void};

use image::{load_from_memory_with_format, ImageFormat, DynamicImage};

use error_code::SystemError;

use core::{mem};

use alloc::string::String;
use alloc::vec::Vec;

use crate::{SysResult, formats};

extern crate std;
use std::eprintln;

/// An image from the clipboard
#[cfg(feature = "image")]
pub enum ClipboardImage {
    /// A binary-image variant for BMP, PNG, JPEG etc.
    ImageBinary(ImageFormat, Option<DynamicImage>),
    /// A text-image variant, e.g. SVG or plain unicode
    ImageString(String, alloc::vec::Vec<u8>)
}

#[cfg(feature = "image")]
struct MappedFormat<'a> {
     kind: &'a str,
     names: [&'a str; 2],
     format: Option<ImageFormat>,
}

#[cfg(feature = "image")]
impl ClipboardImage {
    // creates mapping table from user facing image types to
    // ImageFormats
    fn get_mappedformat_for(imgtype: &str) -> Option<MappedFormat> {
        Some(match imgtype {
            "Bmp" => MappedFormat {
                kind: "binary",
                names: ["CF_DIBV5", ""],
                format: Some(ImageFormat::Bmp)
            },
            "Png" => MappedFormat {
                kind: "binary",
                names: ["PNG", "image/png"],
                format: Some(ImageFormat::Png)
            },
            "Svg" => MappedFormat {
                kind: "string",
                names: ["image/svg+xml", ""],
                format: None
            },
            _ => return None
        })
    }

    /// Fetches an image from the clipboard, returns the enum
    pub fn new<T>(args: T) -> ClipboardImage
        where T: Into<ClipboardImage>
    {
        args.into()
    }

    fn get_id_for_format(requested: String) -> u32 {
        let mut enmfmts = EnumFormats::new();
        let mut found_format_id : u32 = 0;
        for no in &mut enmfmts {
            let available: String = String::from(format_name(no).as_ref().unwrap().as_str());
            eprintln!(
                "cmp format no: {:?} = {:?} with {:?}",
                no,
                available,
                requested,
            );
            if available == requested {
                found_format_id = no;
                break;
            }
        };
        return found_format_id;
    }

    /// Fetches the desired image representation from the clipboard
    /// and stores it in the enum variant
    pub fn get_from_clipboard(mut self) -> SysResult<ClipboardImage> {
        match self {
            ClipboardImage::ImageString(ref format_name, ref mut store) => {
                let id = ClipboardImage::get_id_for_format(String::from(format_name));
                // did we find an id for the desired format string?
                if id != 0 {
                    // append to store
                    match get_vec(id, store) {
                        Ok(_) => return Ok(self),
                        Err(_) => return Err(SystemError::last()),
                    }
                } else {
                    return Err(SystemError::last());
                }
            },
            ClipboardImage::ImageBinary(ref _format, ref mut _img) => {
                return Ok(self);
            }
        };
    }

    /// Writes the image representation to a buffer
    pub fn write_to_buffer(self, out: &mut alloc::vec::Vec<u8>) -> SysResult<usize> {
        eprintln!("write_to_buffer");
        use ClipboardImage::*;
        match self {
            ImageString(_f, mut s) => {
                eprintln!("string arm: {:?}",
                    String::from(std::str::from_utf8(&s).unwrap())
                );
                out.append(&mut s);
                Ok(out.len())
            },
            ImageBinary(_f, _i) => {
                eprintln!("binary arm");
                Ok(0)
            },
        }
    }
}

#[cfg(feature = "image")]
impl From<&str> for ClipboardImage {
    /// Fetches the text representation of an image from the clipboard
    /// for instance a SVG image or drawio base64 encoded format
    fn from(format_name: &str) -> Self {
        // map format_name to available format on the clipboard
        let mf: MappedFormat = ClipboardImage::get_mappedformat_for(format_name).unwrap();
        match mf.kind {
            "binary" => {
                // return the actual data
                return ClipboardImage::ImageBinary(mf.format.unwrap(), None)
            },
            "string" => {
                // check for existing data on the clipboard
                let found_format = mf.names[0];
                // return the actual data
                let store: Vec<u8> = alloc::vec::Vec::new();
                return ClipboardImage::ImageString(String::from(found_format), store)
            },
            _ => {
                panic!("unknown format");
            }
        }
    }
}

#[cfg(feature = "image")]
impl From<Option<String>> for ClipboardImage {
    /// Fetches the text representation of an image from the clipboard
    /// for instance a SVG image or drawio base64 encoded format
    fn from(format_name: Option<String>) -> Self {
        let store = alloc::vec::Vec::new();
        ClipboardImage::ImageString(format_name.unwrap(), store)
    }
}

#[cfg(feature = "image")]
impl From<ImageFormat> for ClipboardImage {
    /// Fetches a binary image from the clipboard and stores it in an
    /// DynamicImage
    fn from(fmt: ImageFormat) -> Self {
        ClipboardImage::ImageBinary(fmt, None)
    }
}

/// Reads PNG image, appending image to the `out` vector and returning number
/// of bytes read on success.
#[cfg(feature = "image")]
pub fn get_png(out: &mut alloc::vec::Vec<u8>, id: u32) -> SysResult<usize> {

    // get the raw size of the image in memory
    let rawsize = size(id).unwrap().get();

    // get the specified format
    let clipboard_data = get_clipboard_data(id)?;
    let lockptr: *mut c_void;
    unsafe {
        // Windows recommends to obtain a locked pointer and use that
        lockptr = GlobalLock(clipboard_data.as_ptr());
        if lockptr.is_null() {
            return Err(SystemError::new(1309));
        }
    }

    let mut buffer = alloc::vec::Vec::new();
    unsafe {
        let imagebuffer: &mut [u8] = core::slice::from_raw_parts_mut(lockptr as *mut u8, rawsize as usize);
        buffer.extend_from_slice(&imagebuffer);

        // now we can release the lock
        GlobalUnlock(clipboard_data.as_ptr());
    }

    let dynimg: DynamicImage = match load_from_memory_with_format(buffer.as_mut_slice(), ImageFormat::Png) {
        Ok(di) => di,
        Err(err) => panic!("DynamicImage from memory failed: {:?}", err),
    };

    match dynimg.write_to(out, ImageFormat::Png) {
        Ok(_) => Ok(rawsize),
        Err(_) => return Err(SystemError::new(1308))
    }
}

/// Reads DIBV5 image, appending image to the `out` vector and returning number
/// of bytes read on success.
#[cfg(feature = "image")]
pub fn get_dibv5(out: &mut alloc::vec::Vec<u8>) -> SysResult<usize> {

    // get the raw size of the image in memory
    let rawsize = size(formats::CF_DIBV5).unwrap().get();

    let clipboard_data = get_clipboard_data(formats::CF_DIBV5)?;
    let lockptr: *mut c_void;
    unsafe {
        // Windows recommends to obtain a locked pointer and use that
        lockptr = GlobalLock(clipboard_data.as_ptr());
        if lockptr.is_null() {
            return Err(SystemError::new(1309));
        }
    }

    // this allows us to refer to individual elements of the header
    // information in below calculations
    let dibv5: BITMAPV5HEADER;
    unsafe {
        // get a pointer to the memory segment where the header starts
        let clipref = lockptr as *mut BITMAPV5HEADER;
        // clone that into our strcuture, GetObjectW did not work for me
        // because the CF_DIBV5 is not supported
        dibv5 = *clipref.clone();
    }

    /*
    eprintln!("bV5Size            {:?}", dibv5.bV5Size);
    eprintln!("bV5Width           {:?}", dibv5.bV5Width);
    eprintln!("bV5Height          {:?}", dibv5.bV5Height);
    eprintln!("bV5Planes          {:?}", dibv5.bV5Planes);
    eprintln!("bV5BitCount        {:?}", dibv5.bV5BitCount);
    eprintln!("bV5Compression     {:?}", dibv5.bV5Compression);
    eprintln!("bV5SizeImage       {:?}", dibv5.bV5SizeImage);
    eprintln!("bV5XPelsPerMeter   {:?}", dibv5.bV5XPelsPerMeter);
    eprintln!("bV5YPelsPerMeter   {:?}", dibv5.bV5YPelsPerMeter);
    eprintln!("bV5ClrUsed         {:?}", dibv5.bV5ClrUsed);
    eprintln!("bV5ClrImportant    {:?}", dibv5.bV5ClrImportant);
    eprintln!("bV5RedMask         {:?}", dibv5.bV5RedMask);
    eprintln!("bV5GreenMask       {:?}", dibv5.bV5GreenMask);
    eprintln!("bV5BlueMask        {:?}", dibv5.bV5BlueMask);
    eprintln!("bV5AlphaMask       {:?}", dibv5.bV5AlphaMask);
    eprintln!("bV5CSType          {:?}", dibv5.bV5CSType);
    eprintln!("bV5GammaRed        {:?}", dibv5.bV5CSType);
    eprintln!("bV5CSType          {:?}", dibv5.bV5CSType);
    eprintln!("bV5GammaRed        {:?}", dibv5.bV5GammaRed);
    eprintln!("bV5GammaGreen      {:?}", dibv5.bV5GammaGreen);
    eprintln!("bV5GammaBlue       {:?}", dibv5.bV5GammaBlue);
    eprintln!("bV5Intent          {:?}", dibv5.bV5Intent);
    eprintln!("bV5ProfileData     {:?}", dibv5.bV5ProfileData);
    eprintln!("bV5ProfileSize     {:?}", dibv5.bV5ProfileSize);
    eprintln!("bV5Reserved        {:?}", dibv5.bV5Reserved);
    */

    let mut filebuffer = alloc::vec::Vec::new();

    // create the BMP file header
    // the 'BM' signature
    filebuffer.extend_from_slice(&u16::to_le_bytes(0x4d42));
    // the file size in total including the file header
    filebuffer.extend_from_slice(&u32::to_le_bytes(mem::size_of::<BITMAPFILEHEADER>() as u32 + rawsize as u32));
    // 2 reserved WORDs
    filebuffer.extend_from_slice(&u32::to_le_bytes(0));
    // offset to pixel array from start of file header
    if dibv5.bV5SizeImage == 0 {
        // BI_RGB images may have set this to zero, then
        filebuffer.extend_from_slice(&u32::to_le_bytes(mem::size_of::<BITMAPFILEHEADER>() as u32 + mem::size_of::<BITMAPV5HEADER>() as u32));
    } else {
        filebuffer.extend_from_slice(&u32::to_le_bytes(mem::size_of::<BITMAPFILEHEADER>() as u32 + (rawsize as u32 - dibv5.bV5SizeImage)));
    }

    // append the whole image structure including the fileinfoheader and bitmap
    unsafe {
        let imagebuffer: &mut [u8] = core::slice::from_raw_parts_mut(lockptr as *mut u8, rawsize as usize);
        filebuffer.extend_from_slice(&imagebuffer);

        // now we can release the lock
        GlobalUnlock(clipboard_data.as_ptr());
    }

    let dynimg: DynamicImage = match load_from_memory_with_format(filebuffer.as_mut_slice(), ImageFormat::Bmp) {
        Ok(di) => di,
        Err(_) => return Err(SystemError::new(1310))
    };

    match dynimg.write_to(out, ImageFormat::Png) {
        Ok(_) => Ok(rawsize),
        Err(_) => return Err(SystemError::new(1311))
    }
}


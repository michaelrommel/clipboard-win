//!Image clipboard support for various formats
//!
//!TODO: Describe module
//!
use super::{EnumFormats, format_name, get_vec, size };

use winapi::um::wingdi::{BITMAPFILEHEADER, BITMAPV5HEADER};

use image::{load_from_memory_with_format, ImageFormat, DynamicImage};

use error_code::SystemError;

use core::{mem};

use alloc::string::String;
use alloc::vec::Vec;

use crate::{SysResult};

extern crate std;
use std::eprintln;

/// An image from the clipboard
#[cfg(feature = "image")]
pub enum ClipboardImage {
    /// A binary-image variant for BMP, PNG, JPEG etc.
    ImageBinary(ImageFormat, Option<DynamicImage>),
    /// A text-image variant, e.g. SVG or plain unicode
    ImageString(u32, String, alloc::vec::Vec<u8>),
    /// Not-Found Variant
    NotFound,
}

/// This struct holds a mapping table from simple format strs 
/// to more information how this image is obtained and stored
#[cfg(feature = "image")]
struct MappedFormat<'a> {
     kind: &'a str,
     names: [&'a str; 2],
     format: Option<ImageFormat>,
}

#[cfg(feature = "image")]
impl ClipboardImage {
    /// Fetches an image from the clipboard, returns the constructed 
    /// ClipboardImage
    pub fn new<T>(args: T) -> ClipboardImage
        where T: Into<ClipboardImage>
    {
        args.into()
    }

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
            "Jpeg" => MappedFormat {
                kind: "binary",
                names: ["JFIF", "image/jpeg"],
                format: Some(ImageFormat::Jpeg)
            },
            "Gif" => MappedFormat {
                kind: "binary",
                names: ["GIF", "image/gif"],
                format: Some(ImageFormat::Gif)
            },
            "Ico" => MappedFormat {
                kind: "binary",
                names: ["image/ico", ""],
                format: Some(ImageFormat::Ico)
            },
            "WebP" => MappedFormat {
                kind: "binary",
                names: ["image/webp", ""],
                format: Some(ImageFormat::WebP)
            },
            "Svg" => MappedFormat {
                kind: "string",
                names: ["image/svg+xml", ""],
                format: None
            },
            _ => return None
        })
    }

    // iterates over the list of provided format names and returns
    // the id if it could be found
    fn get_id_for_format(requested: [&str; 2]) -> (u32, String) {
        let mut enmfmts = EnumFormats::new();
        let mut found_format_id : u32 = 0;
        let mut found_format_name: String = String::from("");
        for no in &mut enmfmts {
            let avail_name = format_name(no); // temporary: Option<str_buf::StrBuf<[u8; 52]>>
            let available: &str = avail_name.as_ref().unwrap().as_str();
            eprintln!(
                "cmp format no: {:?} = {:?} with {:?}",
                no,
                available,
                requested,
            );
            if requested.contains(&available) {
                found_format_id = no;
                found_format_name = String::from(available);
                break;
            }
        };
        return (found_format_id, found_format_name);
    }

    /// Writes the image representation to a user supplied buffer
    pub fn write_to_buffer(self, out: &mut alloc::vec::Vec<u8>) -> SysResult<usize> {
        eprintln!("write_to_buffer");
        use ClipboardImage::*;
        match self {
            ImageString(_id, _f, mut s) => {
                eprintln!("string arm");
                out.append(&mut s);
                Ok(out.len())
            },
            ImageBinary(f, di) => {
                eprintln!("binary arm");
                let out_before = out.len();
                // TODO: let user specify the output format
                //match di.unwrap().write_to(out, ImageFormat::Png) {
                match di.unwrap().write_to(out, f) {
                    Ok(_) => Ok(out.len() - out_before),
                    Err(_) => return Err(SystemError::last())
                }
            },
            NotFound => {
                Ok(0)
            }
        }
    }
}

#[cfg(feature = "image")]
impl From<&[&str]> for ClipboardImage {
    /// Fetches the text representation of an image from the clipboard
    /// for instance a SVG image or drawio base64 encoded format
    fn from(preferred_formats: &[&str]) -> Self {
        let mut result: ClipboardImage = ClipboardImage::NotFound;
        // iterate over given formats
        for preferred_format in preferred_formats {
            eprintln!("Checking for: {:?}", preferred_format);
            // map format_name to available format on the clipboard
            let mf: MappedFormat = ClipboardImage::get_mappedformat_for(preferred_format).unwrap();
            match mf.kind {
                "binary" => {
                    // Fetches a binary representation of an image from the clipboard
                    // for instance a PNG or BMP image
                    // check for existing data on the clipboard
                    let (id, name) = ClipboardImage::get_id_for_format(mf.names);
                    // did we find an id for the desired format string?
                    eprintln!("Binary Got: {:?}: {:?}", id, name);
                    if id != 0 {
                        // special handling for BMP format, as a file header needs
                        // to be prepended so that the image crate can recognize
                        // the format, see
                        // https://github.com/image-rs/image/issues/1569#issuecomment-933028400
                        match name.as_str() {
                            "CF_DIBV5" => {
                                // get the raw size of the image in memory
                                let rawsize = size(id).unwrap().get();
                                eprintln!("Raw size: {:?}", rawsize);

                                let mut store = alloc::vec::Vec::new();
                                // create the BMP file header
                                // the 'BM' signature
                                store.extend_from_slice(&u16::to_le_bytes(0x4d42));
                                // the file size in total including the file header
                                store.extend_from_slice(&u32::to_le_bytes(mem::size_of::<BITMAPFILEHEADER>() as u32 + rawsize as u32));
                                // 2 reserved WORDs
                                store.extend_from_slice(&u16::to_le_bytes(0));
                                store.extend_from_slice(&u16::to_le_bytes(0));
                                // 1 DWORD for the offset, to be filled later
                                store.extend_from_slice(&u32::to_le_bytes(0));

                                let copiedsize = match get_vec(id, &mut store) {
                                    Ok(len) => len,
                                    Err(_) => 0
                                };
                                eprintln!("Copied size: {:?}", copiedsize);

                                if copiedsize > 0 {

                                    eprintln!("Fileheader size: {:?}", mem::size_of::<BITMAPFILEHEADER>() as u32);
                                    // this allows us to refer to individual elements of the headers
                                    let fh: &BITMAPFILEHEADER = unsafe {
                                        &*(store[0..mem::size_of::<BITMAPFILEHEADER>() as usize]
                                            .as_ptr() as *const BITMAPFILEHEADER)
                                    };
                                    let bh: &BITMAPV5HEADER = unsafe {
                                        &*(store[
                                            (mem::size_of::<BITMAPFILEHEADER>() as usize)..
                                            (mem::size_of::<BITMAPFILEHEADER>() as usize +
                                             mem::size_of::<BITMAPV5HEADER>() as usize)]
                                            .as_ptr() as *const BITMAPV5HEADER)
                                    };
                                    // now set correct offset to pixel array from start of file header
                                    if bh.bV5SizeImage == 0 {
                                        // BI_RGB images may have set this to zero, then assume no
                                        // color table
                                        store[10..14].copy_from_slice(
                                            &u32::to_le_bytes(
                                                mem::size_of::<BITMAPFILEHEADER>() as u32 +
                                                mem::size_of::<BITMAPV5HEADER>() as u32)
                                            );
                                    } else {
                                        store[10..14].copy_from_slice(
                                            &u32::to_le_bytes(
                                                mem::size_of::<BITMAPFILEHEADER>() as u32 +
                                                (rawsize as u32 - bh.bV5SizeImage)));
                                    }

                                    eprintln!("bfType             {:?}", {fh.bfType});
                                    eprintln!("bfSize             {:?}", {fh.bfSize});
                                    eprintln!("bfReserved1        {:?}", {fh.bfReserved1});
                                    eprintln!("bfReserved2        {:?}", {fh.bfReserved2});
                                    eprintln!("bfOffBits          {:?}", {fh.bfOffBits});
                                    eprintln!("bV5Size            {:?}", bh.bV5Size);
                                    eprintln!("bV5Width           {:?}", bh.bV5Width);
                                    eprintln!("bV5Height          {:?}", bh.bV5Height);
                                    eprintln!("bV5Planes          {:?}", bh.bV5Planes);
                                    eprintln!("bV5BitCount        {:?}", bh.bV5BitCount);
                                    eprintln!("bV5Compression     {:?}", bh.bV5Compression);
                                    eprintln!("bV5SizeImage       {:?}", bh.bV5SizeImage);
                                    eprintln!("bV5XPelsPerMeter   {:?}", bh.bV5XPelsPerMeter);
                                    eprintln!("bV5YPelsPerMeter   {:?}", bh.bV5YPelsPerMeter);
                                    eprintln!("bV5ClrUsed         {:?}", bh.bV5ClrUsed);
                                    eprintln!("bV5ClrImportant    {:?}", bh.bV5ClrImportant);
                                    eprintln!("bV5RedMask         {:?}", bh.bV5RedMask);
                                    eprintln!("bV5GreenMask       {:?}", bh.bV5GreenMask);
                                    eprintln!("bV5BlueMask        {:?}", bh.bV5BlueMask);
                                    eprintln!("bV5AlphaMask       {:?}", bh.bV5AlphaMask);
                                    eprintln!("bV5CSType          {:?}", bh.bV5CSType);
                                    eprintln!("bV5GammaRed        {:?}", bh.bV5CSType);
                                    eprintln!("bV5CSType          {:?}", bh.bV5CSType);
                                    eprintln!("bV5GammaRed        {:?}", bh.bV5GammaRed);
                                    eprintln!("bV5GammaGreen      {:?}", bh.bV5GammaGreen);
                                    eprintln!("bV5GammaBlue       {:?}", bh.bV5GammaBlue);
                                    eprintln!("bV5Intent          {:?}", bh.bV5Intent);
                                    eprintln!("bV5ProfileData     {:?}", bh.bV5ProfileData);
                                    eprintln!("bV5ProfileSize     {:?}", bh.bV5ProfileSize);
                                    eprintln!("bV5Reserved        {:?}", bh.bV5Reserved);
                                    eprintln!("store len          {:?}", store.len());

                                    match load_from_memory_with_format(store.as_mut_slice(), mf.format.unwrap()) {
                                        Ok(di) => {
                                            // successfully created the DynamicImage
                                            eprintln!("Binary created image");
                                            result = ClipboardImage::ImageBinary(mf.format.unwrap(), Some(di));
                                        },
                                        Err(_) => {
                                            eprintln!("image creation failed: {:?}", mf.format.unwrap());
                                        }
                                    }
                                }
                            },
                            _ => {
                                // all other formats should be parseable by the image crate
                                // allocate temporary buffer for data from the clipboard
                                let mut store = alloc::vec::Vec::new();
                                match get_vec(id, &mut store) {
                                    Ok(_) => {
                                        // the buffer was returned from the clipboard
                                        eprintln!("Binary retrieved buffer: {:?}", store.len());
                                        match load_from_memory_with_format(store.as_mut_slice(), mf.format.unwrap()) {
                                            Ok(di) => {
                                                // successfully created the DynamicImage
                                                eprintln!("Binary created image");
                                                result = ClipboardImage::ImageBinary(mf.format.unwrap(), Some(di));
                                            },
                                            Err(err) => {
                                                eprintln!("image creation failed: {:?}: {:?}", mf.format.unwrap(), err);
                                            }
                                        }
                                    },
                                    // silently ignore the error here
                                    Err(_) => {}
                                }
                            }
                        }
                        break;
                    }
                },
                "string" => {
                    // Fetches the text representation of an image from the clipboard
                    // for instance a SVG image or drawio base64 encoded format
                    // check for existing data on the clipboard
                    let (id, name) = ClipboardImage::get_id_for_format(mf.names);
                    // did we find an id for the desired format string?
                    eprintln!("String Got: {:?}: {:?}", id, name);
                    if id != 0 {
                        // return the actual data
                        let mut store: Vec<u8> = alloc::vec::Vec::new();
                        match get_vec(id, &mut store) {
                            Ok(_) => {
                                result = ClipboardImage::ImageString(id, name, store);
                            },
                            Err(_) => {}
                        }
                        break;
                    }
                },
                _ => {
                    panic!("Format kind not implemented");
                }
            }
        }
        return result;
    }
}

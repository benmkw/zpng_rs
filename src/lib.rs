#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(incomplete_features)]
#![allow(clippy::identity_op)]
#![allow(clippy::many_single_char_names)]
#![feature(const_generics)]

// TODO performance? https://godbolt.org/z/Ezhnh_

/// kept this for typesafety
/// Image data returned by the library
#[derive(Debug)]
pub struct ZPNG_Buffer {
    pub Data: Vec<u8>,
}

/// Image data returned by the library
#[derive(Debug)]
pub struct ZPNG_ImageData {
    /// Pixel data
    pub Buffer: ZPNG_Buffer,

    /// Number of bytes for each color channel (1-2)
    pub BytesPerChannel: u8,

    /// Number of channels for each pixel (1-4)
    pub Channels: u8,

    /// Width in pixels of image
    pub WidthPixels: u16,

    /// Height in pixels of image
    pub HeightPixels: u16,
}

/// Compress image into a buffer.
pub fn ZPNG_Compress(imageData: &ZPNG_ImageData) -> Option<ZPNG_Buffer> {
    let pixelCount: u32 = imageData.WidthPixels as u32 * imageData.HeightPixels as u32;
    let pixelBytes: u32 = imageData.BytesPerChannel as u32 * imageData.Channels as u32;
    let byteCount = pixelBytes as usize * pixelCount as usize;

    // FIXME: One day add support for other formats
    if pixelBytes > 8 {
        return None;
    };

    // Pass 1: Pack and filter data.
    let width = imageData.WidthPixels;
    let height = imageData.HeightPixels;
    let packing = match pixelBytes {
        1 => PackAndFilter::<1>(&imageData.Buffer.Data, width, height, byteCount),
        2 => PackAndFilter::<2>(&imageData.Buffer.Data, width, height, byteCount),
        3 => {
            if cfg!(feature = "ENABLE_RGB_COLOR_FILTER") {
                PackAndFilter_3(&imageData.Buffer.Data, width, height, byteCount)
            } else {
                PackAndFilter::<3>(&imageData.Buffer.Data, width, height, byteCount)
            }
        }
        4 => {
            if cfg!(feature = "ENABLE_RGB_COLOR_FILTER") {
                PackAndFilter_4(&imageData.Buffer.Data, width, height, byteCount)
            } else {
                PackAndFilter::<4>(&imageData.Buffer.Data, width, height, byteCount)
            }
        }
        5 => PackAndFilter::<5>(&imageData.Buffer.Data, width, height, byteCount),
        6 => PackAndFilter::<6>(&imageData.Buffer.Data, width, height, byteCount),
        7 => PackAndFilter::<7>(&imageData.Buffer.Data, width, height, byteCount),
        8 => PackAndFilter::<8>(&imageData.Buffer.Data, width, height, byteCount),
        _ => unreachable!(),
    };

    // Pass 2: Compress the packed/filtered data.
    let mut output = if cfg!(not(feature = "WASM")) {
        zstd::block::compress(&packing, kCompressionLevel).ok()?
    } else {
        todo!()
    };

    // Write header
    let mut header = Vec::<u8>::with_capacity(ZPNG_HEADER_OVERHEAD_BYTES as usize);

    header.extend_from_slice(&u16::to_le_bytes(ZPNG_HEADER_MAGIC));
    header.extend_from_slice(&u16::to_le_bytes(imageData.WidthPixels));
    header.extend_from_slice(&u16::to_le_bytes(imageData.HeightPixels));
    header.extend_from_slice(&u8::to_le_bytes(imageData.Channels));
    header.extend_from_slice(&u8::to_le_bytes(imageData.BytesPerChannel));

    debug_assert_eq!(&header.len(), &(ZPNG_HEADER_OVERHEAD_BYTES as usize));
    header.append(&mut output);

    Some(ZPNG_Buffer { Data: header })
}

/// Decompress image from a buffer
pub fn ZPNG_Decompress(buffer: &ZPNG_Buffer) -> Option<ZPNG_ImageData> {
    let mut buffer = buffer.Data.as_slice();

    debug_assert!(buffer.len() >= ZPNG_HEADER_OVERHEAD_BYTES as usize);
    debug_assert_eq!(
        u16::from_le_bytes([buffer[0], buffer[1]]),
        ZPNG_HEADER_MAGIC
    );

    // parse the header
    let width = u16::from_le_bytes([buffer[2], buffer[3]]);
    let height = u16::from_le_bytes([buffer[4], buffer[5]]);
    let channels = buffer[6];
    let bytesPerChannel = buffer[7];

    // skip the header
    buffer = &buffer[ZPNG_HEADER_OVERHEAD_BYTES as usize..];

    let pixelCount = width as u32 * height as u32;
    let pixelBytes = bytesPerChannel as u32 * channels as u32;
    let byteCount = pixelBytes as usize * pixelCount as usize;

    // Stage 1: Decompress back to packing buffer
    let mut packing = vec![0; byteCount];
    if cfg!(not(feature = "WASM")) {
        let size = zstd::block::decompress_to_buffer(&buffer, &mut packing).ok()?;
        debug_assert_eq!(size, byteCount);
    } else {
        todo!()
    };

    // Stage 2: Unpack/Unfilter
    let zpngBuffer = match pixelBytes {
        1 => UnpackAndUnfilter::<1>(&packing, width, height, byteCount),
        2 => UnpackAndUnfilter::<2>(&packing, width, height, byteCount),
        3 => {
            if cfg!(feature = "ENABLE_RGB_COLOR_FILTER") {
                UnpackAndUnfilter_3(&packing, width, height, byteCount)
            } else {
                UnpackAndUnfilter::<3>(&packing, width, height, byteCount)
            }
        }
        4 => {
            if cfg!(feature = "ENABLE_RGB_COLOR_FILTER") {
                UnpackAndUnfilter_4(&packing, width, height, byteCount)
            } else {
                UnpackAndUnfilter::<4>(&packing, width, height, byteCount)
            }
        }
        5 => UnpackAndUnfilter::<5>(&packing, width, height, byteCount),
        6 => UnpackAndUnfilter::<6>(&packing, width, height, byteCount),
        7 => UnpackAndUnfilter::<7>(&packing, width, height, byteCount),
        8 => UnpackAndUnfilter::<8>(&packing, width, height, byteCount),
        _ => unreachable!(),
    };

    Some(ZPNG_ImageData {
        Buffer: zpngBuffer,
        WidthPixels: width,
        HeightPixels: height,
        Channels: channels,
        BytesPerChannel: bytesPerChannel,
    })
}

const kCompressionLevel: i32 = 1;
const ZPNG_HEADER_MAGIC: u16 = 0xFBF8;
const ZPNG_HEADER_OVERHEAD_BYTES: u8 = std::mem::size_of::<ZPNG_Header>() as u8;

// File format header
// TODO zerocopy or rather less dependencies?
struct ZPNG_Header {
    _Magic: u16,
    _Width: u16,
    _Height: u16,
    _Channels: u8,
    _BytesPerChannel: u8,
}

//------------------------------------------------------------------------------
// Image Processing

// Interleaving is a 1% compression win, and a 0.3% performance win: Not used.
// Splitting the data into blocks of 4 at a time actually reduces compression.

fn PackAndFilter<const kChannels: usize>(
    mut input: &[u8],
    width: u16,
    height: u16,
    byteCount: usize,
) -> Vec<u8> {
    let mut output = vec![0; byteCount];
    let mut output_offset = 0;

    for _y in 0..height {
        let mut prev = [0; kChannels];

        for _x in 0..width {
            // For each channel:
            for i in 0..kChannels {
                let a: u8 = input[i];
                let d: u8 = a.wrapping_sub(prev[i]);
                output[i + output_offset] = d;
                prev[i] = a;
            }
            input = &input[kChannels..];
            output_offset += kChannels;
        }
    }

    debug_assert_eq!(output_offset, output.len());
    output
}

/// #ifdef ENABLE_RGB_COLOR_FILTER
fn PackAndFilter_3(mut input: &[u8], width: u16, height: u16, byteCount: usize) -> Vec<u8> {
    let mut output = vec![0; byteCount];
    const kChannels: usize = 3;

    // Color plane split
    let planeBytes = width as usize * height as usize;
    let mut output_y_offset = 0;
    let mut output_u_offset = planeBytes;
    let mut output_v_offset = planeBytes * 2;

    for _row in 0..height {
        let mut prev = [0; kChannels];

        for _x in 0..width {
            let mut r: u8 = input[0];
            let mut g: u8 = input[1];
            let mut b: u8 = input[2];

            r = r.wrapping_sub(prev[0]);
            g = g.wrapping_sub(prev[1]);
            b = b.wrapping_sub(prev[2]);

            prev[0] = input[0];
            prev[1] = input[1];
            prev[2] = input[2];

            // GB-RG filter from BCIF
            let y: u8 = b;
            let u: u8 = g.wrapping_sub(b);
            let v: u8 = g.wrapping_sub(r);

            output[output_y_offset] = y;
            output_y_offset += 1;

            output[output_u_offset] = u;
            output_u_offset += 1;

            output[output_v_offset] = v;
            output_v_offset += 1;

            input = &input[kChannels..];
        }
    }
    output
}

/// #ifdef ENABLE_RGB_COLOR_FILTER
fn PackAndFilter_4(mut input: &[u8], width: u16, height: u16, byteCount: usize) -> Vec<u8> {
    let mut output = vec![0; byteCount];
    const kChannels: usize = 4;

    // Color plane split
    let planeBytes = width as usize * height as usize;
    let mut output_y_offset = 0;
    let mut output_u_offset = planeBytes;
    let mut output_v_offset = planeBytes * 2;
    let mut output_a_offset = planeBytes * 3;

    for _row in 0..height {
        let mut prev = [0; kChannels];

        for _x in 0..width {
            let mut r: u8 = input[0];
            let mut g: u8 = input[1];
            let mut b: u8 = input[2];
            let mut a: u8 = input[3];

            r = r.wrapping_sub(prev[0]);
            g = g.wrapping_sub(prev[1]);
            b = b.wrapping_sub(prev[2]);
            a = a.wrapping_sub(prev[3]);

            prev[0] = input[0];
            prev[1] = input[1];
            prev[2] = input[2];
            prev[3] = input[3];

            // GB-RG filter from BCIF
            let y: u8 = b;
            let u: u8 = g.wrapping_sub(b);
            let v: u8 = g.wrapping_sub(r);

            output[output_y_offset] = y;
            output_y_offset += 1;

            output[output_u_offset] = u;
            output_u_offset += 1;

            output[output_v_offset] = v;
            output_v_offset += 1;

            output[output_a_offset] = a;
            output_a_offset += 1;

            input = &input[kChannels..];
        }
    }
    output
}

fn UnpackAndUnfilter<const kChannels: usize>(
    mut input: &[u8],
    width: u16,
    height: u16,
    byteCount: usize,
) -> ZPNG_Buffer {
    let mut output = vec![0; byteCount];
    let mut output_offset = 0;

    for _y in 0..height {
        let mut prev = [0; kChannels];

        for _x in 0..width {
            // For each channel:
            for i in 0..kChannels {
                let d: u8 = input[i];
                let a: u8 = d.wrapping_add(prev[i]);
                output[i + output_offset] = a;
                prev[i] = a;
            }

            input = &input[kChannels..];
            output_offset += kChannels;
        }
    }
    debug_assert_eq!(output_offset, output.len());
    ZPNG_Buffer { Data: output }
}

/// #ifdef ENABLE_RGB_COLOR_FILTER
fn UnpackAndUnfilter_3(input: &[u8], width: u16, height: u16, byteCount: usize) -> ZPNG_Buffer {
    let mut output = vec![0; byteCount];
    const kChannels: usize = 3;

    // Color plane split
    let planeBytes = width as usize * height as usize;
    let mut input_y_offset = 0;
    let mut input_u_offset = planeBytes;
    let mut input_v_offset = planeBytes * 2;

    let mut output_offset = 0;

    for _row in 0..height {
        let mut prev = [0; kChannels];

        for _x in 0..width {
            let y: u8 = input[input_y_offset];
            input_y_offset += 1;

            let u: u8 = input[input_u_offset];
            input_u_offset += 1;

            let v: u8 = input[input_v_offset];
            input_v_offset += 1;

            // GB-RG filter from BCIF
            let B: u8 = y;
            let G: u8 = u.wrapping_add(B);
            let mut r: u8 = G.wrapping_sub(v);
            let mut g: u8 = G;
            let mut b: u8 = B;

            r = r.wrapping_add(prev[0]);
            g = g.wrapping_add(prev[1]);
            b = b.wrapping_add(prev[2]);

            output[output_offset + 0] = r;
            output[output_offset + 1] = g;
            output[output_offset + 2] = b;

            prev[0] = r;
            prev[1] = g;
            prev[2] = b;

            output_offset += kChannels;
        }
    }
    debug_assert_eq!(output.len(), output_offset);
    ZPNG_Buffer { Data: output }
}

/// #ifdef ENABLE_RGB_COLOR_FILTER
fn UnpackAndUnfilter_4(input: &[u8], width: u16, height: u16, byteCount: usize) -> ZPNG_Buffer {
    let mut output = vec![0; byteCount];
    const kChannels: usize = 4;

    // Color plane split
    let planeBytes = width as usize * height as usize;
    let mut input_y_offset = 0;
    let mut input_u_offset = planeBytes;
    let mut input_v_offset = planeBytes * 2;
    let mut input_a_offset = planeBytes * 3;

    let mut output_offset = 0;

    for _row in 0..height {
        let mut prev = [0; kChannels];

        for _x in 0..width {
            let y: u8 = input[input_y_offset];
            input_y_offset += 1;

            let u: u8 = input[input_u_offset];
            input_u_offset += 1;

            let v: u8 = input[input_v_offset];
            input_v_offset += 1;

            let mut a: u8 = input[input_a_offset];
            input_a_offset += 1;

            // GB-RG filter from BCIF
            let B: u8 = y;
            let G: u8 = u.wrapping_add(B);
            let mut r: u8 = G.wrapping_sub(v);
            let mut g: u8 = G;
            let mut b: u8 = B;

            r = r.wrapping_add(prev[0]);
            g = g.wrapping_add(prev[1]);
            b = b.wrapping_add(prev[2]);
            a = a.wrapping_add(prev[3]);

            output[output_offset + 0] = r;
            output[output_offset + 1] = g;
            output[output_offset + 2] = b;
            output[output_offset + 3] = a;

            prev[0] = r;
            prev[1] = g;
            prev[2] = b;
            prev[3] = a;

            output_offset += kChannels;
        }
    }
    debug_assert_eq!(output.len(), output_offset);
    ZPNG_Buffer { Data: output }
}

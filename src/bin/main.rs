#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(incomplete_features)]
#![feature(const_generics)]

use argh::FromArgs;
use std::fs;
use std::path::Path;
use std::process::Command;
use zpng_rs_lib::{ZPNG_Buffer, ZPNG_Compress, ZPNG_Decompress, ZPNG_ImageData};

// TODO it would be nice to use a crate that would allow mutually exclusive options (-c -d)
// clap has this feature (and thus structopt probably as well but structopt/ clap gave me linker errors at the time of writing)
#[derive(FromArgs)]
/// Zpng_rs - Experimental Lossless Image Compressor
struct Opt {
    /// compress an image (jpeg, webp, tga, bmp, png, gif, ico), saves as .zpng
    #[argh(switch, short = 'c')]
    compress: bool,

    /// decompress a .zpng image, saves as .png
    #[argh(switch, short = 'd')]
    decompress: bool,

    /// test the compressor for compatibility with input file.
    ///
    /// 1st: Makes sure that it can decompress the image without writing it to disc.
    /// 2nd: Outputs zpng by itself and by calling the original zpng tool.
    /// 3rd: Decompresses the foreign output and lets the original zpng tool decompress its own output.
    ///
    /// You can adapt the path in the source to test your implementation.
    #[argh(switch)]
    test: bool,

    /// input file
    #[argh(option, short = 'i')]
    inpath: String,

    /// output file, deduced to be the input filename with .png
    #[argh(option, short = 'o')]
    outpath: Option<String>,
}

fn main() {
    if cfg!(feature = "color_backtrace") {
        std::env::set_var("RUST_BACKTRACE", "full");
        color_backtrace::install();
    }

    let opt: Opt = argh::from_env();
    let inpath = opt.inpath;
    let mut outpath = opt.outpath;

    if opt.compress as u8 + opt.decompress as u8 + opt.test as u8 > 1 {
        println!("ERROR: --compress, --decompress and --test are mutually exclusive");
        return;
    }

    if opt.compress {
        if outpath.is_none() {
            outpath = Some(
                Path::new(&inpath)
                    .with_extension("zpng")
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        }

        // TODO don't always convert to rgba, if this is rgb this is not needed
        let image = image::open(&inpath).unwrap().to_rgba();
        let (width, height) = image.dimensions();

        let image_data = ZPNG_ImageData {
            Buffer: ZPNG_Buffer {
                Data: image.into_raw(),
            },
            // TODO deduce this from image open metadata
            BytesPerChannel: 1,
            // TODO deduce this from image open metadata
            Channels: 4,
            WidthPixels: width as u16,
            HeightPixels: height as u16,
        };
        let comp = ZPNG_Compress(&image_data).unwrap();

        fs::write(outpath.unwrap(), &comp.Data).unwrap();
    } else if opt.decompress {
        if outpath.is_none() {
            outpath = Some(
                Path::new(&inpath)
                    .with_extension("png")
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        }

        let buffer = fs::read(&inpath).unwrap();
        let dec = ZPNG_Decompress(&ZPNG_Buffer { Data: buffer }).unwrap();

        let format = match dec.Channels {
            3 => image::ColorType::Rgb8,
            4 => image::ColorType::Rgba8,
            _ => unimplemented!(),
        };

        image::save_buffer(
            outpath.unwrap(),
            &dec.Buffer.Data,
            dec.WidthPixels as u32,
            dec.HeightPixels as u32,
            format,
        )
        .unwrap();
    } else if opt.test {
        let buffer = image::open(inpath.clone()).unwrap().to_rgba();
        let (width, height) = buffer.dimensions();

        let image_data = ZPNG_ImageData {
            Buffer: ZPNG_Buffer {
                Data: buffer.into_raw(),
            },
            BytesPerChannel: 1,
            Channels: 4,
            WidthPixels: width as u16,
            HeightPixels: height as u16,
        };

        // makes sure that it can decompress the image without writing it to disc
        let comp = ZPNG_Compress(&image_data).expect("compression failed");
        let _dec = ZPNG_Decompress(&comp).expect("decompression failed");

        // produces and outputs itself and by calling the external zpng tool
        let rust_zpng = Path::new(&inpath).with_file_name("zpng_rs.zpng");
        let orig_zpng = Path::new(&inpath).with_file_name("zpng.zpng");
        assert!(Command::new("sh")
            .arg("-c")
            .arg(format!(
                "./Zpng/build/zpng -c {} {}",
                inpath,
                orig_zpng.to_str().unwrap()
            ))
            .status()
            .unwrap()
            .success());

        // write own output
        let image = image::open(&inpath).unwrap().to_rgba();
        let (width, height) = image.dimensions();

        let image_data = ZPNG_ImageData {
            Buffer: ZPNG_Buffer {
                Data: image.into_raw(),
            },
            BytesPerChannel: 1,
            Channels: 4,
            WidthPixels: width as u16,
            HeightPixels: height as u16,
        };
        let comp = ZPNG_Compress(&image_data).unwrap();

        fs::write(rust_zpng.clone(), &comp.Data).unwrap();

        // decompresses the foreign output and lets the other zpng tool decompress its own output
        let rust_png = Path::new(&inpath).with_file_name("zpng_rs.png");
        let orig_png = Path::new(&inpath).with_file_name("zpng.png");
        assert!(Command::new("sh")
            .arg("-c")
            .arg(format!(
                "./Zpng/build/zpng -d {} {}",
                rust_zpng.to_str().unwrap().to_string(),
                orig_png.to_str().unwrap().to_string()
            ))
            .status()
            .unwrap()
            .success());

        // read foreign file and decompress
        let buffer = fs::read(orig_zpng).unwrap();

        let dec = ZPNG_Decompress(&ZPNG_Buffer { Data: buffer }).unwrap();

        let format = match dec.Channels {
            3 => image::ColorType::Rgb8,
            4 => image::ColorType::Rgba8,
            _ => unimplemented!(),
        };

        image::save_buffer(
            rust_png,
            &dec.Buffer.Data,
            dec.WidthPixels as u32,
            dec.HeightPixels as u32,
            format,
        )
        .unwrap();
    }
}

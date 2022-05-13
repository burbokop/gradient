extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::subtitle::Bitmap;
use ffmpeg::util::frame::video::Video;
use gradient::bitmap::{BitmapRef, from_argb, to_argb, RgbU24Layout};
use image::{ImageBuffer, GenericImageView};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{prelude::*, Cursor};
use std::path::Path;
use std::time::Instant;

#[derive(Serialize, Deserialize, Clone)]
struct Cfg {
    pub index: u32,
    pub rate_n: i32,
    pub rate_d: i32
}


impl PartialEq for Cfg {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for Cfg {
    
}

impl PartialOrd for Cfg {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

impl Ord for Cfg {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

fn load_cfg(path: &str) -> BTreeMap<String, Cfg> {
    std::fs::read_to_string(path).map(|f|toml::from_str(f.as_str()).unwrap()).unwrap_or_default()
}

fn save_cfg(path: &str, map: &BTreeMap<String, Cfg>) {
    std::fs::write(path, toml::to_string(map).unwrap()).unwrap()
}


fn mul_pixel_u32(p: u32, m: f64) -> u32 {
    from_argb(to_argb(p).map(|x| ((x as f64) * m) as u8))
}

fn invert_rgb(p: u32) -> u32 {
    let mut argb = to_argb(p);
    argb[1] = 0xff - argb[1];
    argb[2] = 0xff - argb[2];
    argb[3] = 0xff - argb[3];
    from_argb(argb)
}

fn discard_alpha(p: u32) -> u32 {
    let mut argb = to_argb(p);
    argb[0] = 0;
    from_argb(argb)
}

fn avr_rgb(p: u32) -> u8 {
    let mut argb = to_argb(p);
    ((argb[1] as u32 + argb[2] as u32 + argb[3] as u32) / 3) as u8    
}


fn dif_mul(p1: u32, p0: u32, m: f64) -> u32 {
    let mut argb1 = to_argb(p1);
    let argb0 = to_argb(p0);
    argb1[1] = ((argb1[1] as f64 - argb0[1] as f64).abs() * m) as u8;
    argb1[2] = ((argb1[2] as f64 - argb0[2] as f64).abs() * m) as u8;
    argb1[3] = ((argb1[3] as f64 - argb0[3] as f64).abs() * m) as u8;
    from_argb(argb1)
}

fn dif(p1: u32, p0: u32) -> u32 {
    let mut argb1 = to_argb(p1);
    let argb0 = to_argb(p0);
    argb1[1] = (argb1[1] as i16 - argb0[1] as i16).abs() as u8;
    argb1[2] = (argb1[2] as i16 - argb0[2] as i16).abs() as u8;
    argb1[3] = (argb1[3] as i16 - argb0[3] as i16).abs() as u8;
    from_argb(argb1)
}


fn mul_pixel_u32_rgb(p: u32, m: f64) -> u32 {
    let mut argb = to_argb(p);
    argb[1] = ((argb[1] as f64) * m) as u8;
    argb[2] = ((argb[2] as f64) * m) as u8;
    argb[3] = ((argb[3] as f64) * m) as u8;
    from_argb(argb)
}


pub fn each_btmp_pair<'a>(prev: &mut BitmapRef<'a, RgbU24Layout<0, 1, 2>>, current: &mut BitmapRef<'a, RgbU24Layout<0, 1, 2>>, integrator: Integrator, frame_index: usize) {
    prev.clone_by(current, |mut c, p| {
        c.set_argb_u32(dif(c.get_argb_u32(), p.get_argb_u32()))
        //*c = *p
    });

    integrator.next()

    //let avr = current.iter_mut().map(|x| avr_rgb(x.get_argb_u32()) as u32).sum::<u32>() / current.width() as u32 / current.height() as u32;
//
    //if avr == 0 {
    //    current.for_each_mut(|mut x| x.set_argb_u32(invert_rgb(x.get_argb_u32())));
    //}
}


fn main() -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let input_path = env::args().nth(1).expect("Cannot open file.");

    let cfg_path = "./out/videos.toml";


    if let Ok(mut ictx) = input(&input_path) {
        let input = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;


        let mut cfg = load_cfg(cfg_path);
        let video_index = cfg
            .get(&input_path)
            .map(|x| x.clone())
            .unwrap_or(
                cfg.iter()
                    .map(|(_, b)| b).max().map(|x| Cfg { index: x.index + 1, rate_n: input.rate().numerator(), rate_d: input.rate().denominator() })
                    .unwrap_or(Cfg { index: 0, rate_n: input.rate().numerator(), rate_d: input.rate().denominator() })
                );

        cfg.insert(input_path.clone(), video_index.clone());
        
        save_cfg(cfg_path, &cfg);
        
        println!("starting processing video stream #{:0>4} (path: {})", video_index.index, input_path);
        println!("frame rate: {} ({})", input.rate(), input.avg_frame_rate());

        let video_stream_index = input.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        let mut decoder = context_decoder.decoder().video()?;

        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        )?;

        let mut frame_index = 0;

        //let mut last_frame_data: &[u8]
        

        let mut each_frame = |prev: &mut Option<Video>, current: &mut Video, frame_index: usize| {
            let cw = current.width() as usize;
            let ch = current.height() as usize;

            let mut btmp = BitmapRef::from_bytes(
                current.data_mut(0), 
                cw,
                ch
            ).unwrap();

            if let Some(v) = prev {
                let w = v.width() as usize;
                let h = v.height() as usize;
    
                let mut prev_btmp = BitmapRef::from_bytes(
                    v.data_mut(0), 
                    w,
                    h
                ).unwrap();


                each_btmp_pair(&mut prev_btmp, &mut btmp, frame_index)
            };
        };

        let mut prev_frame: Option<Video> = None;


        let mut receive_and_process_decoded_frames =
            |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                let mut decoded = Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {


                        let mut rgb_frame = Video::empty();
                        scaler.run(&decoded, &mut rgb_frame)?;

                        let begin_inst = Instant::now();
                        let copy = rgb_frame.clone();
                        let after_clone_inst = Instant::now();

                    if let Some(path) = prepare_file(frame_index, video_index.index) {
                        each_frame(&mut prev_frame, &mut rgb_frame, frame_index);
                        let after_transform_inst = Instant::now();

                        save_file_u24(&rgb_frame, path).unwrap();
                        let after_save_inst = Instant::now();
                        println!("frame{} produced (time {{ clone: {:?}, transform: {:?}, save: {:?} }})", frame_index, after_clone_inst - begin_inst, after_transform_inst - after_clone_inst, after_save_inst - after_transform_inst);
                    } else {
                        println!("frame{} skiped", frame_index);
                    }
                    prev_frame = Some(copy);

                    frame_index += 1;
                }
                Ok(())
            };

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet)?;
                receive_and_process_decoded_frames(&mut decoder)?;
            }
        }
        decoder.send_eof()?;
        receive_and_process_decoded_frames(&mut decoder)?;
    }

    Ok(())
}

fn prepare_file(index: usize, video_index: u32) -> Option<String> {
    let dir = format!("./out/video_{:0>4}", video_index);
    std::fs::create_dir_all(dir.as_str()).unwrap();
    let file = format!("{}/frame{}.ppm", dir, index);
    if Path::new(file.as_str()).exists() {
        None
    } else {
        Some(file)
    }
}

fn save_only_ppm(frame: &Video, path: String) -> std::result::Result<(), std::io::Error> {
    let data = frame.data(0);
    let mut ppm24 = Vec::with_capacity(data.len());

    for i in (0..data.len()).step_by(4) {
        let x = &data[i..(i + 4)];
        let r = x[1];
        let g = x[2];
        let b = x[3];
        ppm24.push(r);
        ppm24.push(g);
        ppm24.push(b);
    }

    let mut file = File::create(path)?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(&ppm24)?;
    Ok(())
}

fn save_file_u24(frame: &Video, path: String) -> std::result::Result<(), std::io::Error> {

    let mut file = File::create(path)?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(frame.data(0))?;
    Ok(())
}
/*
fn save_file(frame: &Video, path: String) -> std::result::Result<(), std::io::Error> {


    let dir = format!("./out/video_{:0>4}", video_index);


    std::fs::create_dir_all(dir.as_str()).unwrap();

    let mut vec = Vec::from(frame.data(0));

    let mut ppm24 = Vec::with_capacity(vec.len());

    for i in (0..vec.len()).step_by(4) {
        let x = &mut vec[i..(i + 4)];
        let a = x[0];
        let r = x[1];
        let g = x[2];
        let b = x[3];
        x[0] = r;
        x[1] = g;
        x[2] = b;
        x[3] = a;

        ppm24.push(r);
        ppm24.push(g);
        ppm24.push(b);
    }

    let img = image::DynamicImage::ImageRgba8(ImageBuffer::from_raw(frame.width(), frame.height(), vec).unwrap());

    //let img = image::io::Reader::new(Cursor::new(frame.data(0))).with_guessed_format()?.decode().unwrap();


    img.save(format!("{}/frame{}.png", dir, index)).unwrap();


    let mut file = File::create(format!("{}/frame{}.ppm", dir, index))?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(&ppm24)?;
    Ok(())
}*/
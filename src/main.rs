use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;

use v4l::prelude::*;
use v4l::video::Capture;
use v4l::buffer::Type;
use v4l::FourCC;
use v4l::io::traits::{CaptureStream, Stream};

use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

mod gui;
mod render;

pub fn to_bytes(input: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 * input.len());

    for value in input {
        bytes.extend(&value.to_be_bytes());
    }

    bytes
}

fn main() {

    let id = 0_usize;
    //let fwidth = 640;
    //let fheight = 480;
    let fcc = b"YUYV";

    let dev = Device::new(id).expect("Failed to open device");
    
    let mut fmt = dev.format().expect("Failed to get Device format");
    //fmt.width = fwidth;
    //fmt.height = fheight;
    fmt.fourcc = FourCC::new(fcc);

    
    let id_mtx = Arc::new(Mutex::new(id));
    let frate_mtx = Arc::new(Mutex::new((1, 30)));
    let framesize_mtx = Arc::new(Mutex::new((fmt.width, fmt.height)));
    let fourcc_mtx : Arc<Mutex<[u8; 4]>> = Arc::new(Mutex::new(*fcc));

    let id_mtx_clone = id_mtx.clone();
    let frate_mtx_clone = frate_mtx.clone();
    let framesize_mtx_clone = framesize_mtx.clone();
    let fourcc_mtx_clone = fourcc_mtx.clone();

    let (tx, rx) = mpsc::channel();

    //v4l capture thread
    thread::spawn(move || {
        
        let mut fmt = dev.set_format(&fmt).expect("Failed to write format");
        // The actual format chosen by the device driver may differ from what we
        // requested! Print it out to get an idea of what is actually used now.
        println!("Format in use:\n{}", fmt);

        let mut parms = dev.params().expect("Failed to load device parameters");

        println!("Parameters in use:\n{:?}", parms);
 
        // Create the stream, which will internally 'allocate' (as in map) the
        // number of requested buffers for us.
        let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4)
            .unwrap();

        // At this point, the stream is ready and all buffers are setup.
        // We can now read frames (represented as buffers) by iterating through
        // the stream. Once an error condition occurs, the iterator will return
        // None.
        loop {

            let fcc = *fourcc_mtx.lock().unwrap();
            let (frate_num, frate_denom) = *frate_mtx.lock().unwrap(); 
            let (width, height) = *framesize_mtx.lock().unwrap();

            if parms.interval.denominator != frate_denom || parms.interval.numerator != frate_num {

                parms.interval.denominator = frate_denom;
                parms.interval.numerator = frate_num;
                
                match stream.stop() {
            
                    Ok(_) => { 
                        match dev.set_params(&parms) {
                            Ok(parms) => {println!("Parameters set to:\n{}", parms);},
                            Err(er) => {println!("Failed to set Parameters: {}", er);}
                        }
                        if let Err(er) = stream.start() {println!("Failed to start video stream: {}", er);}
                    },
                    Err(er) => {println!("Failed to stop video stream: {}", er);}
                }
            }

            if fmt.width != width || fmt.height != height || !fmt.fourcc.repr.eq(&fcc) {
                
                    fmt.width = width;
                    fmt.height = height;
                    fmt.fourcc = v4l::FourCC::new(&fcc);

                    stream.stop().expect("Couldn't stop video stream");
                    //must drop the old stream to set the new frame format 
                    drop(stream);

                    fmt = dev.set_format(&fmt).expect("Failed to write format");
                    // The actual format chosen by the device driver may differ from what we
                    // requested! Print it out to get an idea of what is actually used now.
                    println!("Format in use:\n{}", fmt);
                    
                    stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4)
            .unwrap();

            } 
            
            let (buf, _meta) = stream.next().unwrap();

           // println!(
           //     "Buffer size: {}, seq: {}, timestamp: {}",
           //     buf.len(),
           //     meta.sequence,
           //     meta.timestamp
           // );

           // To process the captured data, you can pass it somewhere else.
           // If you want to modify the data or extend its lifetime, you have to
           // copy it. This is a best-effort tradeoff solution that allows for
           // zero-copy readers while enforcing a full clone of the data for
           // writers.

           let data = match &fmt.fourcc.repr {
               b"YUYV" => buf.to_vec(),
               b"MJPG" => {
                    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
                    // Decode the JPEG frame to RGBA
                    let mut decoder = JpegDecoder::new_with_options(buf, options);
                    //let info = decoder.info().unwrap();
                    //eprintln!("{:?}", info);
                    //rgb vec
                   
                    decoder.decode().expect("failed to decode JPEG")
               }
               _ => panic!("invalid buffer pixelformat"),
           };

           tx.send(fmt.fourcc.repr.to_vec()).unwrap();
           
           let vsize: Vec<u32> = vec![fmt.width, fmt.height];
           let frame_size = to_bytes(&vsize);

           tx.send(frame_size).unwrap();

           tx.send(data).unwrap();
        }

    });

    //render thread 
    let _th_join_handle = thread::spawn( move|| {
        let mut rend = render::Render::new(
            fmt.width,
            fmt.height, 
            &fmt.fourcc.repr);

        let _ = rend.render_data(rx);
    });

    //gui window
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("rustycamera",
        native_options, 
        Box::new(|cc| {
            Ok(Box::new(
                gui::GuiApp::new(cc, id_mtx_clone, frate_mtx_clone, framesize_mtx_clone, fourcc_mtx_clone)))
            }
        )
    );
    
}

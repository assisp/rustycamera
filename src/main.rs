use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;

use v4l::{prelude::*, v4l2};
use v4l::v4l_sys::v4l2_colorfx_V4L2_COLORFX_SOLARIZATION;
use v4l::video::Capture;
use v4l::buffer::Type;
use v4l::FourCC;
use v4l::io::traits::{CaptureStream, Stream};

use jpeg_decoder as jpeg;

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

    let mut dev = Device::new(id).expect("Failed to open device");
    
    let mut fmt = dev.format().expect("Failed to get Device format");
    //fmt.width = fwidth;
    //fmt.height = fheight;
    fmt.fourcc = FourCC::new(fcc);

    
    let id_mtx = Arc::new(Mutex::new(id));
    let framesize_mtx = Arc::new(Mutex::new((fmt.width, fmt.height)));
    let fourcc_mtx : Arc<Mutex<[u8; 4]>> = Arc::new(Mutex::new(*fcc));

    let id_mtx_clone = id_mtx.clone();
    let framesize_mtx_clone = framesize_mtx.clone();
    let fourcc_mtx_clone = fourcc_mtx.clone();

    let (tx, rx) = mpsc::channel();

    //v4l capture thread
    thread::spawn(move || {
        
        let mut fmt = dev.set_format(&fmt).expect("Failed to write format");
        // The actual format chosen by the device driver may differ from what we
        // requested! Print it out to get an idea of what is actually used now.
        println!("Format in use:\n{}", fmt);        
        
        // Create the stream, which will internally 'allocate' (as in map) the
        // number of requested buffers for us.
        let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4)
            .unwrap();

        // At this point, the stream is ready and all buffers are setup.
        // We can now read frames (represented as buffers) by iterating through
        // the stream. Once an error condition occurs, the iterator will return
        // None.
        loop {

            let (width, height) = *framesize_mtx.lock().unwrap();

            if fmt.width != width || 
                fmt.height != height {
                
                    fmt.width = width;
                    fmt.height = height;

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
            
            let (buf, meta) = stream.next().unwrap();

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
                   // Decode the JPEG frame to RGB
                   let mut decoder = jpeg::Decoder::new(buf);
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
    let th_join_handle = thread::spawn( move|| {
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
                gui::GuiApp::new(cc, id_mtx_clone, framesize_mtx_clone, fourcc_mtx_clone)))
            }
        )
    );
    
}

fn get_v4l_device(device_id: usize, width: u32, height: u32) -> Device {
    // Create a new capture device with a few extra parameters
    let dev = Device::new(device_id).expect("Failed to open device");

    // Let's say we want to explicitly request another format
    let mut fmt = dev.format().expect("Failed to read format");
    fmt.width = width;
    fmt.height = height;
    fmt.fourcc = FourCC::new(b"YUYV");
    let fmt = dev.set_format(&fmt).expect("Failed to write format");

    // The actual format chosen by the device driver may differ from what we
    // requested! Print it out to get an idea of what is actually used now.
    println!("Format in use:\n{}", fmt);

    // Now we'd like to capture some frames!
    // First, we need to create a stream to read buffers from. We choose a
    // mapped buffer stream, which uses mmap to directly access the device
    // frame buffer. No buffers are copied nor allocated, so this is actually
    // a zero-copy operation.

    // To achieve the best possible performance, you may want to use a
    // UserBufferStream instance, but this is not supported on all devices,
    // so we stick to the mapped case for this example.
    // Please refer to the rustdoc docs for a more detailed explanation about
    // buffer transfers.

    dev
}

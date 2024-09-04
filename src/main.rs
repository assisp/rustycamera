use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use v4l::prelude::*;
use v4l::video::Capture;
use v4l::buffer::Type;
use v4l::FourCC;
use v4l::io::traits::CaptureStream;

use jpeg_decoder as jpeg;

mod gui;
mod render;

fn main() {

    let mut id = AtomicUsize::new(0);

    let width = 1280;
    let height = 720;

    let (tx, rx) = mpsc::channel();

    let dev = Device::new(id.load(Ordering::Relaxed)).expect("Failed to open device");
    
    let mut fmt = dev.format().expect("Failed to get Device format");
    fmt.width = width;
    fmt.height = height;
    fmt.fourcc = FourCC::new(b"YUYV");
    let fmt = dev.set_format(&fmt).expect("Failed to write format");

    //v4l capture thread
    thread::spawn(move || {
        // Create the stream, which will internally 'allocate' (as in map) the
        // number of requested buffers for us.
        let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4)
            .unwrap();

        // At this point, the stream is ready and all buffers are setup.
        // We can now read frames (represented as buffers) by iterating through
        // the stream. Once an error condition occurs, the iterator will return
        // None.
        loop {
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

            tx.send(data).unwrap();
        }

    });

    //render thread 
    let th_join_handle = thread::spawn( move|| {
        let rend = render::Render::new(width, height, &fmt.fourcc.repr);
        let _ = rend.render_data(rx);
    });

    //gui window
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("rustycamera",
        native_options, 
        Box::new(|cc| {
            Ok(Box::new(
                gui::GuiApp::new(cc, id)))
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

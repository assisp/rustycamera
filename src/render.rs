use std::sync::mpsc;
use std::time::SystemTime;

use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

pub fn from_bytes(input:&mut &[u8]) -> Vec<u32> {
    
    let mut len = input.len();

    let mut output : Vec<u32> = Vec::new();

    while len >= 4 {
        let (int_bytes, rest) = input.split_at(std::mem::size_of::<u32>());
        *input = rest;
        output.push(u32::from_be_bytes(int_bytes.try_into().unwrap()));
        len -= 4;
    }

    //println!("size: {:?}", output);

    output
}

pub struct Render {
    width: u32,
    height: u32,
    fourcc: [u8; 4],
}

impl Render {
    pub fn new(width: u32, height: u32, fourcc: &[u8; 4]) -> Self {
        Self{
            width,
            height,
            fourcc: *fourcc,
        }
    }

    pub fn render_data(&mut self, rx : mpsc::Receiver<Vec<u8>>) -> Result<(), String> {
        
        let mut fps_count : f64 = 0.;
        // We init systems.
        let sdl_context = sdl2::init().expect("failed to init SDL");
        let video_subsystem = sdl_context.video().expect("failed to get video context");
        //we create a window
        let window = video_subsystem
            .window("Rustycamera", 800, 600)
            .position_centered()
            .resizable()
            .opengl()
            .build()
            .expect("failed to build window");
        
        // We get the canvas from which we can get the `TextureCreator`.
        let mut canvas: Canvas<Window> = window.into_canvas()
            .build()
            .expect("failed to build window's canvas");
        let texture_creator = canvas.texture_creator();
        
        let mut pix_fmt = match &self.fourcc {
            b"YUYV" => PixelFormatEnum::YUY2,
            b"MJPG" => PixelFormatEnum::RGBA32,
            _ => panic!("invalid buffer pixelformat"),
        };

        let _ = canvas.set_logical_size(self.width, self.height);
        
        
        let mut texture = texture_creator.create_texture_streaming(pix_fmt, self.width, self.height).unwrap();
        
        let mut running = true;
        let mut event_pump = sdl_context.event_pump().unwrap();
        
        let mut now = SystemTime::now();
            
        while running {
             
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => {
                        running = false;
                    }
                    _ => {}
                }
            }

            let fourcc_data = rx.recv().unwrap();
            
            let size_data = rx.recv().unwrap();
            let b: &mut &[u8] = &mut &size_data[..];
            let size = from_bytes(b);
        
            let data = rx.recv().unwrap();

            if size[0] != self.width || size[1] != self.height || !self.fourcc.eq(&fourcc_data[..]) {
                self.width = size[0];
                self.height = size[1];
                self.fourcc.clone_from_slice(&fourcc_data);

                println!("new render texture format: {} -> {}x{}", std::str::from_utf8(&self.fourcc).expect("Failed to convert fourcc to string"), self.width, self.height);

                pix_fmt = match &self.fourcc {
                    b"YUYV" => PixelFormatEnum::YUY2,
                    b"MJPG" => PixelFormatEnum::RGBA32,
                    _ => panic!("invalid buffer pixelformat"),
                };

                texture = texture_creator.create_texture_streaming(pix_fmt, self.width, self.height).unwrap();
                let _ = canvas.set_logical_size(self.width, self.height);
            }

            texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                buffer[..].clone_from_slice(&data);
            }).expect("Failed texture data copy");
        
            canvas.clear();
            canvas.copy(&texture, None, None).expect("copy texture");
            canvas.present();

            fps_count += 1.;

            match now.elapsed() {
                Ok(elapsed) => {
                    if elapsed.as_secs_f64() >= 2.0 {
                        let fps = fps_count / elapsed.as_secs_f64();
                        let window = canvas.window_mut();
                        let title = format!("rustycamera  - {:.2} fps", fps);
                        let _ = window.set_title(&title);
                        fps_count = 0.;
                        now = SystemTime::now();
                    };
                }
                Err(e) => {
                    // an error occurred!
                    println!("Error: {e:?}");
                }
            }
        };
    
        Ok(())
    }
}

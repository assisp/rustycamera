use std::sync::mpsc;

use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

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

    pub fn render_data(&self, rx : mpsc::Receiver<Vec<u8>>) -> Result<(), String> {
    
        // We init systems.
        let sdl_context = sdl2::init().expect("failed to init SDL");
        let video_subsystem = sdl_context.video().expect("failed to get video context");
        //we create a window
        let window = video_subsystem
            .window("SDL2 Render", self.width, self.height)
            .position_centered()
            .opengl()
            .build()
            .expect("failed to build window");
        
        // We get the canvas from which we can get the `TextureCreator`.
        let mut canvas: Canvas<Window> = window.into_canvas()
            .build()
            .expect("failed to build window's canvas");
        let texture_creator = canvas.texture_creator();
        
        let pix_fmt = match &self.fourcc {
            b"YUYV" => PixelFormatEnum::YUY2,
            b"MJPG" => PixelFormatEnum::RGB24,
            _ => panic!("invalid buffer pixelformat"),
        };
        
        
        let mut texture = texture_creator.create_texture_streaming(pix_fmt, self.width, self.height).unwrap();
        
        let mut running = true;
        let mut event_pump = sdl_context.event_pump().unwrap();
        
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
        
            let data = rx.recv().unwrap();
            texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                buffer[..].clone_from_slice(&data);
            }).expect("texture data copy");
        
            canvas.clear();
            canvas.copy(&texture, None, None).expect("copy texture");
            canvas.present();
        };
    
        Ok(())
    }
}

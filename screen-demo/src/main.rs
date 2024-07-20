use crc::CRC_4_G_704;

use speedy2d::{
    color::Color,
    dimen::Vec2,
    image::ImageHandle,
    shape::Rectangle,
    window::{WindowHandler, WindowHelper},
    Graphics2D, Window,
};

const SCREEN_X: usize = 1920;
const SCREEN_Y: usize = 1080;

const TOT_X: usize = 10;
const TOT_Y: usize = 8;
const COLOR_SHIFT_PERCENT: f32 = 0.05;

const X_BITS: usize = 4;
const Y_BITS: usize = 3;
const CRC_BITS: usize = CRC_4_G_704.width as usize;
const TOT_BITS: usize = X_BITS + Y_BITS + CRC_BITS;

const PIXELS_PER_X_STEP: usize = SCREEN_X / TOT_X;
const PIXELS_PER_Y_STEP: usize = SCREEN_Y / TOT_Y;

struct MyWindowHandler {
    cur_frame: i8,
    base_interval: i8,
    cur_bit: i8,
    background: Option<ImageHandle>,
    send_coordinates: bool,
    mouse_loc: Vec2,
    color_baseline: (u8, u8, u8),
    transmitted_val: usize,
    start_time: std::time::Instant,
}

impl WindowHandler for MyWindowHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        self.cur_frame += 1;
        // println!("cur_frame: {}", self.cur_frame);
        if self.background.is_some() && self.mouse_loc.x != 0.0 && self.mouse_loc.y != 0.0 {
            let screen = graphics.capture(speedy2d::image::ImageDataType::RGB);
            let offset = (self.mouse_loc.y as usize * SCREEN_X + self.mouse_loc.x as usize) * 3;
            let r = screen.data()[offset];
            let g = screen.data()[offset + 1];
            let b = screen.data()[offset + 2];
            if (self.cur_frame - 1) % (self.base_interval + 1) != 0 {
                // println!("\t\t{r}, {g}, {b}");
                self.transmitted_val <<= 1;
                if r + g + b > self.color_baseline.0 + self.color_baseline.1 + self.color_baseline.2
                {
                    self.transmitted_val |= 1;
                }
            } else {
                // println!("baseline:\t{r}, {g}, {b}");
                self.color_baseline = (r, g, b);
            }
            if self.cur_bit as usize == TOT_BITS {
                let tot_time = std::time::Instant::now().duration_since(self.start_time);
                let recv_x = self.transmitted_val >> (Y_BITS + CRC_BITS);
                let recv_y = self.transmitted_val >> CRC_BITS & (2_usize.pow(Y_BITS as u32) - 1);
                let recv_crc = self.transmitted_val & (2_usize.pow(CRC_BITS as u32) - 1);

                let crc_val =
                    crc::Crc::<u8>::new(&CRC_4_G_704).checksum(&[recv_x as u8, recv_y as u8]);

                println!(
                    "received: ({}, {}) in {} ms, crc valid: {}",
                    recv_x,
                    recv_y,
                    tot_time.as_millis(),
                    crc_val as usize == recv_crc
                );
            }
        }
        graphics.clear_screen(Color::from_rgb(0.5, 0.5, 0.5));

        if self.background.is_none() {
            self.background = Some(
                graphics
                    .create_image_from_file_path(
                        None,
                        speedy2d::image::ImageSmoothingMode::Linear,
                        "map.jpg",
                    )
                    .unwrap(),
            )
        }
        graphics.draw_image((0.0, 0.0), self.background.as_ref().unwrap());

        // draw grid lines
        for x in 1..TOT_X {
            let pixel_x = (x * PIXELS_PER_X_STEP) as f32;
            graphics.draw_line(
                (pixel_x, 0.0),
                (pixel_x, SCREEN_Y as f32),
                1.0,
                Color::BLACK,
            );
        }
        for y in 1..TOT_Y {
            let pixel_y = (y * PIXELS_PER_Y_STEP) as f32;
            graphics.draw_line(
                (0.0, pixel_y),
                (SCREEN_X as f32, pixel_y),
                1.0,
                Color::BLACK,
            );
        }

        if self.send_coordinates {
            graphics.draw_rectangle(
                Rectangle::from_tuples((0.0, 0.0), (SCREEN_X as f32, SCREEN_Y as f32)),
                Color::from_rgba(0.0, 0.0, 0.0, COLOR_SHIFT_PERCENT),
            );

            if self.cur_frame % (self.base_interval + 1) != 0 {
                // println!("Sending bit: {}", self.cur_bit);
                // draw grid
                // draw y
                for y in 0..TOT_Y {
                    let pixel_y = (y * PIXELS_PER_Y_STEP) as f32;
                    for x in 0..TOT_X {
                        let pixel_x = (x * PIXELS_PER_X_STEP) as f32;

                        let crc_val =
                            crc::Crc::<u8>::new(&CRC_4_G_704).checksum(&[x as u8, y as u8]);

                        let transmission = (x << (Y_BITS + CRC_BITS))
                            | (y << CRC_BITS)
                            | (crc_val as usize & (2_usize.pow(CRC_BITS as u32) - 1));
                        if (transmission >> (TOT_BITS - 1 - self.cur_bit as usize)) & 1 > 0 {
                            graphics.draw_rectangle(
                                Rectangle::from_tuples(
                                    (pixel_x, pixel_y),
                                    (
                                        pixel_x + PIXELS_PER_X_STEP as f32,
                                        pixel_y + PIXELS_PER_Y_STEP as f32,
                                    ),
                                ),
                                Color::from_rgba(1.0, 1.0, 1.0, COLOR_SHIFT_PERCENT),
                            );
                        }
                    }
                }

                self.cur_bit += 1;
                if self.cur_bit as usize >= TOT_BITS {
                    self.send_coordinates = false;
                }
            }

            helper.request_redraw();
        }
        // std::thread::sleep(std::time::Duration::from_millis(500))
    }

    fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        _button: speedy2d::window::MouseButton,
    ) {
        let x = self.mouse_loc.x as usize / PIXELS_PER_X_STEP;
        let y = self.mouse_loc.y as usize / PIXELS_PER_Y_STEP;
        println!(
            "({}, {}) : ({x}, {y})",
            self.mouse_loc.x as usize, self.mouse_loc.y as usize
        );
        if !self.send_coordinates {
            self.send_coordinates = true;
            self.cur_bit = 0;
            self.cur_frame = 0;
            self.transmitted_val = 0;
            self.start_time = std::time::Instant::now();
            helper.request_redraw();
        }
    }

    fn on_mouse_move(&mut self, _helper: &mut WindowHelper<()>, position: Vec2) {
        self.mouse_loc = position;
    }
}

fn main() {
    let window = Window::new_centered("Demo", (SCREEN_X as u32, SCREEN_Y as u32)).unwrap();
    window.run_loop(MyWindowHandler {
        cur_frame: 0,
        base_interval: 1,
        cur_bit: 0,
        background: None,
        send_coordinates: false,
        mouse_loc: Vec2::new(0.0, 0.0),
        color_baseline: (0, 0, 0),
        transmitted_val: 0,
        start_time: std::time::Instant::now(),
    });
}

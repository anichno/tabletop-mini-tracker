use std::{
    io::{Read, Write},
    net::TcpStream,
};

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
const COLOR_SHIFT_PERCENT: f32 = 0.02;

const X_BITS: usize = 4;
const Y_BITS: usize = 3;
const CRC_BITS: usize = CRC_4_G_704.width as usize;
const TOT_BITS: usize = X_BITS + Y_BITS + CRC_BITS;

const PIXELS_PER_X_STEP: usize = SCREEN_X / TOT_X;
const PIXELS_PER_Y_STEP: usize = SCREEN_Y / TOT_Y;

#[derive(Debug, Clone, Copy)]
enum TransmissionState {
    NotTransmitting,
    StartTransmitting,
    CalibrateLow,
    CalibrateHigh,
    BitSent(u8),
    DoneTransmitting,
}

struct MiniBase {
    conn: TcpStream,
}

impl MiniBase {
    fn sync(&mut self) {
        self.conn.write_all(&[b'I']).unwrap();

        let mut buf = [0; 1];
        let bytes_read = self.conn.read(&mut buf).unwrap();
        if bytes_read == 0 {
            panic!("rpi disconnected");
        }

        if buf[0] != b'A' {
            panic!("Invalid ack sent from rpi");
        }
    }
    fn notify_transmission(&mut self, state: TransmissionState) {
        match state {
            TransmissionState::NotTransmitting => return,
            TransmissionState::StartTransmitting => self
                .conn
                .write_all(&[b'S', X_BITS as u8, Y_BITS as u8, CRC_BITS as u8])
                .unwrap(),
            TransmissionState::CalibrateLow => self.conn.write_all(&[b'L']).unwrap(),
            TransmissionState::CalibrateHigh => self.conn.write_all(&[b'H']).unwrap(),
            TransmissionState::BitSent(bit_num) => self.conn.write_all(&[b'D', bit_num]).unwrap(),
            TransmissionState::DoneTransmitting => self.conn.write_all(&[b'F']).unwrap(),
        }

        let mut buf = [0; 1];
        let bytes_read = self.conn.read(&mut buf).unwrap();
        if bytes_read == 0 {
            panic!("rpi disconnected");
        }

        if buf[0] != b'A' {
            panic!("Invalid ack sent from rpi");
        }
    }
}

struct MyWindowHandler {
    background: Option<ImageHandle>,
    // send_coordinates: bool,
    mouse_loc: Vec2,
    start_time: std::time::Instant,
    transmission_state: TransmissionState,
    paired_mini: MiniBase,
}

fn draw_bits_to_screen(bit_num: u8, graphics: &mut Graphics2D) {
    // draw grid
    // draw y
    for y in 0..TOT_Y {
        let pixel_y = (y * PIXELS_PER_Y_STEP) as f32;
        for x in 0..TOT_X {
            let pixel_x = (x * PIXELS_PER_X_STEP) as f32;

            let crc_val = crc::Crc::<u8>::new(&CRC_4_G_704).checksum(&[x as u8, y as u8]);

            let transmission = (x << (Y_BITS + CRC_BITS))
                | (y << CRC_BITS)
                | (crc_val as usize & (2_usize.pow(CRC_BITS as u32) - 1));
            let color = if (transmission >> (TOT_BITS - 1 - bit_num as usize)) & 1 > 0 {
                Color::from_rgba(1.0, 1.0, 1.0, COLOR_SHIFT_PERCENT)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, COLOR_SHIFT_PERCENT)
            };

            graphics.draw_rectangle(
                Rectangle::from_tuples(
                    (pixel_x, pixel_y),
                    (
                        pixel_x + PIXELS_PER_X_STEP as f32,
                        pixel_y + PIXELS_PER_Y_STEP as f32,
                    ),
                ),
                color,
            );
        }
    }
}

impl WindowHandler for MyWindowHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        println!("transmission state: {:?}", self.transmission_state);
        self.paired_mini
            .notify_transmission(self.transmission_state);

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

        match self.transmission_state {
            TransmissionState::NotTransmitting => (),
            TransmissionState::StartTransmitting => {
                graphics.draw_rectangle(
                    Rectangle::from_tuples((0.0, 0.0), (SCREEN_X as f32, SCREEN_Y as f32)),
                    Color::from_rgba(0.0, 0.0, 0.0, COLOR_SHIFT_PERCENT),
                );
                self.transmission_state = TransmissionState::CalibrateLow;
                helper.request_redraw();
            }
            TransmissionState::CalibrateLow => {
                graphics.draw_rectangle(
                    Rectangle::from_tuples((0.0, 0.0), (SCREEN_X as f32, SCREEN_Y as f32)),
                    Color::from_rgba(1.0, 1.0, 1.0, COLOR_SHIFT_PERCENT),
                );
                self.transmission_state = TransmissionState::CalibrateHigh;
                helper.request_redraw();
            }
            TransmissionState::CalibrateHigh => {
                draw_bits_to_screen(0, graphics);
                self.transmission_state = TransmissionState::BitSent(0);
                helper.request_redraw();
            }
            TransmissionState::BitSent(bit_num) => {
                let next_bit = bit_num + 1;
                if (next_bit as usize) < TOT_BITS {
                    draw_bits_to_screen(next_bit, graphics);
                    self.transmission_state = TransmissionState::BitSent(next_bit);
                    helper.request_redraw();
                } else {
                    self.transmission_state = TransmissionState::DoneTransmitting;
                    helper.request_redraw();
                }
            }
            TransmissionState::DoneTransmitting => {
                self.transmission_state = TransmissionState::NotTransmitting
            }
        }
    }

    fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        _button: speedy2d::window::MouseButton,
    ) {
        let x = self.mouse_loc.x as usize / PIXELS_PER_X_STEP;
        let y = self.mouse_loc.y as usize / PIXELS_PER_Y_STEP;
        println!("Preparing to send: ({x}, {y})");
        if matches!(self.transmission_state, TransmissionState::NotTransmitting) {
            // !self.send_coordinates {
            // self.send_coordinates = true;
            self.transmission_state = TransmissionState::StartTransmitting;
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

    let rpi = TcpStream::connect("hardware-lab:9000").unwrap();
    let mut paired_mini = MiniBase { conn: rpi };
    paired_mini.sync();

    window.run_loop(MyWindowHandler {
        background: None,
        // send_coordinates: false,
        mouse_loc: Vec2::new(0.0, 0.0),
        start_time: std::time::Instant::now(),
        transmission_state: TransmissionState::NotTransmitting,
        paired_mini,
    });
}

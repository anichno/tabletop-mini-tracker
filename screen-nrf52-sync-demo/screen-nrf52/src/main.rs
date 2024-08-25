use crc::CRC_4_G_704;

use log::{debug, info};
use mini::{MiniManager, MiniMessage};
use speedy2d::{
    color::Color,
    image::ImageHandle,
    shape::Rectangle,
    window::{WindowCreationOptions, WindowHandler, WindowHelper, WindowSize},
    Graphics2D, Window,
};
use tokio::runtime::Runtime;

mod mini;

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

struct MyWindowHandler {
    background: Option<ImageHandle>,
    start_time: std::time::Instant,
    transmission_state: TransmissionState,
    paired_minis: MiniManager,
    runtime: Runtime,
    mini_location: Option<(usize, usize)>,
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

impl WindowHandler<MiniMessage> for MyWindowHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper<MiniMessage>, graphics: &mut Graphics2D) {
        println!("transmission state: {:?}", self.transmission_state);
        match self.transmission_state {
            TransmissionState::NotTransmitting => (),
            TransmissionState::StartTransmitting => self.runtime.block_on(async {
                self.mini_location = None;
                self.paired_minis
                    .start_transmission(X_BITS as u8, Y_BITS as u8, CRC_BITS as u8)
                    .await;
            }),
            TransmissionState::CalibrateLow => {
                self.runtime.block_on(self.paired_minis.calibrate_low())
            }
            TransmissionState::CalibrateHigh => {
                self.runtime.block_on(self.paired_minis.calibrate_high())
            }
            TransmissionState::BitSent(_) => self.runtime.block_on(self.paired_minis.send_bit()),
            TransmissionState::DoneTransmitting => self.paired_minis.end_transmission(),
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

        if let Some((mini_x, mini_y)) = self.mini_location {
            // draw box around mini
            let start_x = (mini_x * PIXELS_PER_X_STEP) as f32;
            let end_x = ((mini_x + 1) * PIXELS_PER_X_STEP) as f32;
            let start_y = (mini_y * PIXELS_PER_Y_STEP) as f32;
            let end_y = ((mini_y + 1) * PIXELS_PER_Y_STEP) as f32;

            const THICKNESS: f32 = 6.0;
            const COLOR: Color = Color::GREEN;

            graphics.draw_line((start_x, start_y), (end_x, start_y), THICKNESS, COLOR);
            graphics.draw_line((start_x, end_y), (end_x, end_y), THICKNESS, COLOR);
            graphics.draw_line((start_x, start_y), (start_x, end_y), THICKNESS, COLOR);
            graphics.draw_line((end_x, start_y), (end_x, end_y), THICKNESS, COLOR);
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

    fn on_user_event(&mut self, helper: &mut WindowHelper<MiniMessage>, user_event: MiniMessage) {
        debug!("on_user_event: {:?}", user_event);
        match user_event {
            MiniMessage::Command(cmd) => match cmd {
                common::ScreenCommands::TriggerTransmission => {
                    if matches!(self.transmission_state, TransmissionState::NotTransmitting) {
                        self.transmission_state = TransmissionState::StartTransmitting;
                        self.start_time = std::time::Instant::now();
                        helper.request_redraw();
                    }
                }
            },
            MiniMessage::Position(location) => {
                self.mini_location = Some((location.x as usize, location.y as usize));
            }
        }
    }
}

fn main() {
    env_logger::init();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    info!("Attempting to find Minis");
    let mut paired_minis = runtime.block_on(mini::MiniManager::new()).unwrap();
    info!("Connected to Minis");

    let window = Window::new_with_user_events(
        "Demo",
        WindowCreationOptions::new_windowed(
            WindowSize::PhysicalPixels((SCREEN_X as u32, SCREEN_Y as u32).into()),
            None,
        ),
    )
    .unwrap();
    let event_sender = window.create_user_event_sender();

    runtime.block_on(paired_minis.register_screen_channel(event_sender));

    window.run_loop(MyWindowHandler {
        background: None,
        start_time: std::time::Instant::now(),
        transmission_state: TransmissionState::NotTransmitting,
        paired_minis,
        runtime,
        mini_location: None,
    });
}

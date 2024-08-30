use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver},
    thread::{self, sleep},
    time::Duration,
};

use serial_common::Commands;
use serialport::SerialPort;
use speedy2d::{
    color::Color,
    window::{UserEventSender, WindowHandler, WindowHelper},
    Graphics2D, Window,
};

const SCREEN_X: usize = 300;
const SCREEN_Y: usize = 300;

enum ScreenCommands {
    Draw(Vec<DrawCommands>),
    Quit,
}
enum DrawCommands {
    // Line((f32, f32), (f32, f32)),
    Circle(Color, (f32, f32), f32),

    Background(Color),
}

struct MyWindowHandler {
    screen_drawn: mpsc::SyncSender<bool>,
    screen_ready: mpsc::SyncSender<bool>,
    draw_commands: Option<Vec<DrawCommands>>,
    first_draw: bool,
}

impl WindowHandler<ScreenCommands> for MyWindowHandler {
    fn on_draw(&mut self, _helper: &mut WindowHelper<ScreenCommands>, graphics: &mut Graphics2D) {
        if self.first_draw {
            graphics.clear_screen(Color::GREEN);
            self.first_draw = false;
        }

        if let Some(cmds) = self.draw_commands.take() {
            for cmd in cmds {
                match cmd {
                    // DrawCommands::Line(_, _) => todo!(),
                    DrawCommands::Background(color) => graphics.clear_screen(color),
                    DrawCommands::Circle(color, center, radius) => {
                        graphics.draw_circle(center, radius, color)
                    }
                }
            }

            self.screen_drawn
                .try_send(true)
                .expect("Failed to report screen drawn");
        }
    }

    fn on_mouse_button_down(
        &mut self,
        _helper: &mut WindowHelper<ScreenCommands>,
        _button: speedy2d::window::MouseButton,
    ) {
        println!("Got click");
        let _ = self.screen_ready.try_send(true);
    }

    fn on_user_event(
        &mut self,
        helper: &mut WindowHelper<ScreenCommands>,
        user_event: ScreenCommands,
    ) {
        match user_event {
            ScreenCommands::Draw(draw_cmds) => {
                self.draw_commands = Some(draw_cmds);
                helper.request_redraw();
            }
            ScreenCommands::Quit => helper.terminate_loop(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct CalibrationData {
    delay: Duration,
    // dark_val: u32,
    // light_val: u32,
    pixels_per_mm: f64,
}

struct Calibrator {
    sensors: SerialSensors,
    calibration_data: CalibrationData,
    screen_ready: Receiver<bool>,
    screen_drawn: Receiver<bool>,
    screen_event_sender: UserEventSender<ScreenCommands>,
}

impl Calibrator {
    fn run(&mut self) {
        self.sensors
            .set_light_conversion_time(opt4048::ConversionTime::Time600us);
        self.screen_ready.recv().unwrap();

        self.calibrate_brightness();

        // calc delay
        self.calibrate_delay();

        self.calibrate_pixels_per_mm();

        self.find_center();
    }

    fn calibrate_brightness(&mut self) {
        // get dark val
        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::BLACK,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));
        self.sensors.calibrate_dark();

        // get light val
        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::WHITE,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));
        self.sensors.calibrate_light();

        // get middle val
        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::GRAY,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));
        self.sensors.calibrate_middle();
    }

    fn calibrate_delay(&mut self) {
        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::BLACK,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));

        let mut min = u32::MAX;
        let mut max = 0;
        let mut avg = 0;
        let delay = 0;
        let num_trials = 25;

        for _trial in 0..num_trials {
            // dark to light
            self.screen_event_sender
                .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                    Color::WHITE,
                )]))
                .unwrap();
            self.screen_drawn.recv().unwrap();
            let val = self.sensors.get_delay(true, Duration::from_millis(delay));
            // print!("{val} : ");

            min = min.min(val);
            max = max.max(val);
            avg += val;

            // light to dark
            self.screen_event_sender
                .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                    Color::BLACK,
                )]))
                .unwrap();
            self.screen_drawn.recv().unwrap();
            let val = self.sensors.get_delay(false, Duration::from_millis(delay));
            // println!("{val}");

            min = min.min(val);
            max = max.max(val);
            avg += val;
        }

        avg /= num_trials * 2;

        self.calibration_data.delay = Duration::from_millis(max as u64);

        println!("min: {min}, max: {max}, avg: {avg}");
    }

    fn calibrate_pixels_per_mm(&mut self) {
        self.sensors
            .set_light_conversion_time(opt4048::ConversionTime::Time600us);

        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::WHITE,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));
        let mut bright_val = 0;
        for _ in 0..10 {
            bright_val += self.sensors.get_light_val();
        }
        bright_val /= 10;

        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::BLACK,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));
        let mut dark_val = 0;
        for _ in 0..10 {
            dark_val += self.sensors.get_light_val();
        }
        dark_val /= 10;

        let center = (SCREEN_X as f32 / 2.0, SCREEN_Y as f32 / 2.0);
        let mut min_radius = 0;
        let mut max_radius = 0;
        let brightness_diff = bright_val - dark_val;
        let start_brightness = (brightness_diff as f32 * 0.025) as u32 + dark_val;
        let end_brightness = bright_val - (brightness_diff as f32 * 0.025) as u32;
        println!("dark: {dark_val}, bright: {bright_val}");

        for radius in 1..(SCREEN_X / 2) {
            self.screen_event_sender
                .send_event(ScreenCommands::Draw(vec![
                    DrawCommands::Background(Color::BLACK),
                    DrawCommands::Circle(Color::WHITE, center, radius as f32),
                ]))
                .unwrap();
            self.screen_drawn.recv().unwrap();
            sleep(self.calibration_data.delay);

            let mut val = 0;
            for _ in 0..10 {
                val += self.sensors.get_light_val();
            }
            val /= 10;

            if min_radius == 0 && val > start_brightness {
                min_radius = radius;
            } else if max_radius == 0 && val > end_brightness {
                max_radius = radius;
                break;
            }
        }

        let hole_diameter = max_radius - min_radius;
        dbg!(hole_diameter);
        self.calibration_data.pixels_per_mm = hole_diameter as f64 / 3.0;
        dbg!(self.calibration_data.pixels_per_mm);
    }

    fn find_center(&mut self) {
        self.sensors
            .set_light_conversion_time(opt4048::ConversionTime::Time600us);

        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::WHITE,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));
        let mut bright_val = 0;
        for _ in 0..10 {
            bright_val += self.sensors.get_light_val();
        }
        bright_val /= 10;

        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![DrawCommands::Background(
                Color::BLACK,
            )]))
            .unwrap();
        self.screen_drawn.recv().unwrap();
        sleep(Duration::from_secs(1));
        let mut dark_val = 0;
        for _ in 0..10 {
            dark_val += self.sensors.get_light_val();
        }
        dark_val /= 10;

        // find the smallest circle which completely encompasses the mini aperature
        let center = (SCREEN_X as f32 / 2.0, SCREEN_Y as f32 / 2.0);
        let brightness_diff = bright_val - dark_val;
        let end_brightness = bright_val - (brightness_diff as f32 * 0.1) as u32;

        let mut containing_radius = None;
        for radius in 1..(SCREEN_X / 2) {
            self.screen_event_sender
                .send_event(ScreenCommands::Draw(vec![
                    DrawCommands::Background(Color::BLACK),
                    DrawCommands::Circle(Color::WHITE, center, radius as f32),
                ]))
                .unwrap();
            self.screen_drawn.recv().unwrap();
            sleep(self.calibration_data.delay);

            let val = self.sensors.get_light_val();

            if val > end_brightness {
                containing_radius = Some(radius);
                break;
            }
        }

        dbg!(containing_radius);

        let Some(radius) = containing_radius else {
            panic!("Couldnt find mini");
        };

        // find angle from estimated center
        let radius = (radius as f64 - (self.calibration_data.pixels_per_mm * 1.5)) as f32;
        let mut max_brightness = 0;
        let mut max_brightness_angle = 0.0;
        let mut real_center = None;
        for angle in 0..360 {
            let angle_radians = (angle as f32).to_radians();
            let x = radius * angle_radians.cos() + center.0;
            let y = radius * angle_radians.sin() + center.1;

            self.screen_event_sender
                .send_event(ScreenCommands::Draw(vec![
                    DrawCommands::Background(Color::BLACK),
                    DrawCommands::Circle(
                        Color::WHITE,
                        (x, y),
                        self.calibration_data.pixels_per_mm as f32 * 1.5,
                    ),
                ]))
                .unwrap();
            self.screen_drawn.recv().unwrap();
            sleep(self.calibration_data.delay);

            let val = self.sensors.get_light_val();

            if val > bright_val / 4 && val < max_brightness - (max_brightness as f32 * 0.05) as u32
            {
                real_center = Some((x, y));
                break;
            } else if val > max_brightness {
                max_brightness = val;
                max_brightness_angle = angle as f64;
            }
        }

        dbg!(max_brightness_angle);
        dbg!(real_center);

        self.screen_event_sender
            .send_event(ScreenCommands::Draw(vec![
                DrawCommands::Background(Color::BLACK),
                DrawCommands::Circle(
                    Color::GREEN,
                    real_center.unwrap(),
                    self.calibration_data.pixels_per_mm as f32 * (40.0 / 2.0),
                ),
            ]))
            .unwrap();
        self.screen_drawn.recv().unwrap();

        sleep(Duration::from_secs(1000));
    }
}

impl Drop for Calibrator {
    fn drop(&mut self) {
        self.screen_event_sender
            .send_event(ScreenCommands::Quit)
            .unwrap();
    }
}

struct SerialSensors {
    conn: Box<dyn SerialPort>,
}

impl SerialSensors {
    fn ping_pong(&mut self) {
        let mut output_buf = [0; 4];

        self.conn.write_all(&[Commands::Ping.into()]).unwrap();
        self.conn.read_exact(&mut output_buf).unwrap();

        assert_eq!(&output_buf, &[b'P', b'O', b'N', b'G'])
    }

    fn set_light_conversion_time(&mut self, time: opt4048::ConversionTime) {
        let time_val = match time {
            opt4048::ConversionTime::Time600us => 1,
            opt4048::ConversionTime::Time1ms => 2,
            opt4048::ConversionTime::Time1ms8 => 3,
            opt4048::ConversionTime::Time3ms4 => 4,
            opt4048::ConversionTime::Time6ms5 => 5,
            opt4048::ConversionTime::Time12ms7 => 6,
            opt4048::ConversionTime::Time25ms => 7,
            opt4048::ConversionTime::Time50ms => 8,
            opt4048::ConversionTime::Time100ms => 9,
            opt4048::ConversionTime::Time200ms => 10,
            opt4048::ConversionTime::Time400ms => 11,
            opt4048::ConversionTime::Time800ms => 12,
        };

        let mut ack_buf = [0; 1];

        self.conn
            .write_all(&[Commands::SetLightConversionTime.into(), time_val])
            .unwrap();
        self.conn.read_exact(&mut ack_buf).unwrap();

        assert_eq!(ack_buf[0], b'A')
    }

    fn get_light_val(&mut self) -> u32 {
        let mut output_buf = [0; 4];
        self.conn
            .write_all(&[Commands::GetLightValue.into()])
            .unwrap();
        self.conn.read_exact(&mut output_buf).unwrap();

        u32::from_le_bytes(output_buf)
    }

    fn calibrate_dark(&mut self) {
        let mut ack_buf = [0; 1];

        self.conn
            .write_all(&[Commands::CalibrateDark.into()])
            .unwrap();
        self.conn.read_exact(&mut ack_buf).unwrap();

        assert_eq!(ack_buf[0], b'A')
    }

    fn calibrate_light(&mut self) {
        let mut ack_buf = [0; 1];

        self.conn
            .write_all(&[Commands::CalibrateLight.into()])
            .unwrap();
        self.conn.read_exact(&mut ack_buf).unwrap();

        assert_eq!(ack_buf[0], b'A')
    }

    fn calibrate_middle(&mut self) {
        let mut ack_buf = [0; 1];

        self.conn
            .write_all(&[Commands::CalibrateMiddle.into()])
            .unwrap();
        self.conn.read_exact(&mut ack_buf).unwrap();

        assert_eq!(ack_buf[0], b'A')
    }

    fn get_delay(&mut self, dark_to_light: bool, wait: Duration) -> u32 {
        let mut output_buf = [0; 4];

        self.conn
            .write_all(&[
                Commands::GetFrameDelay.into(),
                dark_to_light.into(),
                wait.as_millis().try_into().unwrap(),
            ])
            .unwrap();
        self.conn.read_exact(&mut output_buf).unwrap();

        u32::from_le_bytes(output_buf)
    }
}

fn main() {
    let mut sensors = SerialSensors {
        conn: serialport::new(
            "/dev/serial/by-id/usb-SEGGER_J-Link_001050292515-if00",
            115_200,
        )
        .timeout(Duration::from_secs(5))
        .open()
        .expect("Unable to open sensor serial path"),
    };
    sensors.ping_pong();

    let window = Window::new_with_user_events(
        "Demo",
        speedy2d::window::WindowCreationOptions::new_windowed(
            speedy2d::window::WindowSize::PhysicalPixels((SCREEN_X as u32, SCREEN_Y as u32).into()),
            None,
        ),
    )
    .unwrap();
    let event_sender: UserEventSender<ScreenCommands> = window.create_user_event_sender();

    let (screen_drawn_sender, screen_drawn_receiver) = mpsc::sync_channel(1);
    let (screen_ready_sender, screen_ready_receiver) = mpsc::sync_channel(1);

    let mut calibrator = Calibrator {
        sensors,
        calibration_data: CalibrationData::default(),
        screen_ready: screen_ready_receiver,
        screen_drawn: screen_drawn_receiver,
        screen_event_sender: event_sender,
    };

    thread::spawn(move || {
        calibrator.run();
    });

    window.run_loop(MyWindowHandler {
        screen_drawn: screen_drawn_sender,
        screen_ready: screen_ready_sender,
        draw_commands: None,
        first_draw: true,
    });
}

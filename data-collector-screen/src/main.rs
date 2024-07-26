use std::{
    io::{Read, Write},
    net::TcpStream,
};

use speedy2d::{
    color::Color,
    shape::Rectangle,
    window::{WindowHandler, WindowHelper},
    Graphics2D, Window,
};

const BASE_BRIGHTNESS_STEP: f32 = 0.1;
const OVERLAY_BRIGHTNESS_STEP: f32 = 0.005;

const MAX_OVERLAY_BRIGHTNESS: f32 = 0.05;

const NUM_CONSISTENCY_TESTS: usize = 10;

const SCREEN_X: u32 = 200;
const SCREEN_Y: u32 = 200;

struct BrightnessWindowHandler {
    cur_base_brightness: f32,
    cur_overlay_brightness: f32,
    testing: bool,
    drawing_black: bool,
    hit_max_overlay: bool,
    rpi: TcpStream,
    cur_low: i32,
    results: Vec<Vec<i32>>,
    output_results: bool,
    results_new_row: bool,
}

impl WindowHandler for BrightnessWindowHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::from_rgba(
            self.cur_base_brightness,
            self.cur_base_brightness,
            self.cur_base_brightness,
            1.0,
        ));

        if self.testing {
            println!(
                "{} : {} : {}",
                self.cur_base_brightness, self.cur_overlay_brightness, self.drawing_black
            );
            // wait for sync from rpi
            let mut buf: [u8; 6] = [0; 6]; // S + u32 + <\n>?
            let bytes_read = self.rpi.read(&mut buf).unwrap();
            if bytes_read == 0 {
                helper.terminate_loop();
                return;
            }
            if buf[0] != b'S' && (buf[0] == b'B' && bytes_read < 5) {
                helper.request_redraw();
                return;
            }

            let brightness = u32::from_le_bytes(buf[1..5].try_into().unwrap()) as i32;
            println!("brightness: {}", brightness);

            if self.drawing_black {
                if buf[0] == b'B' {
                    self.results
                        .last_mut()
                        .unwrap()
                        .push(brightness - self.cur_low);
                }

                if self.output_results {
                    let mut x = OVERLAY_BRIGHTNESS_STEP;
                    print!(",");
                    for _ in 0..self.results[0].len() {
                        print!("{:.3},", x);
                        x += OVERLAY_BRIGHTNESS_STEP;
                    }
                    println!();

                    let mut x = 0.0;
                    for i in 0..self.results.len() {
                        print!("{:.2},", x);
                        for j in &self.results[i] {
                            print!("{},", j);
                        }
                        println!();
                        x += BASE_BRIGHTNESS_STEP;
                    }
                    helper.terminate_loop();
                    return;
                }

                if self.results_new_row {
                    self.results.push(Vec::new());
                    self.results_new_row = false;
                }

                graphics.draw_rectangle(
                    Rectangle::from_tuples((0.0, 0.0), (SCREEN_X as f32, SCREEN_Y as f32)),
                    Color::from_rgba(0.0, 0.0, 0.0, self.cur_overlay_brightness),
                );
                self.drawing_black = false;
                // send black to rpi
                self.rpi.write_all(&[b'B']).unwrap();
            } else {
                self.cur_low = brightness;
                graphics.draw_rectangle(
                    Rectangle::from_tuples((0.0, 0.0), (SCREEN_X as f32, SCREEN_Y as f32)),
                    Color::from_rgba(1.0, 1.0, 1.0, self.cur_overlay_brightness),
                );
                self.drawing_black = true;

                if !self.hit_max_overlay {
                    let next_overlay_brightness =
                        self.cur_overlay_brightness + OVERLAY_BRIGHTNESS_STEP;
                    if next_overlay_brightness >= MAX_OVERLAY_BRIGHTNESS {
                        self.hit_max_overlay = true;
                        self.cur_overlay_brightness = MAX_OVERLAY_BRIGHTNESS;
                    } else {
                        self.cur_overlay_brightness = next_overlay_brightness;
                    }
                } else {
                    self.hit_max_overlay = false;
                    self.cur_overlay_brightness = OVERLAY_BRIGHTNESS_STEP;
                    self.cur_base_brightness += BASE_BRIGHTNESS_STEP;

                    if self.cur_base_brightness >= 1.0 + BASE_BRIGHTNESS_STEP {
                        self.output_results = true;
                    } else {
                        self.results_new_row = true;
                    }
                }

                // send white to rpi
                self.rpi.write_all(&[b'W']).unwrap();
            }

            helper.request_redraw();
        }
    }

    fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        _button: speedy2d::window::MouseButton,
    ) {
        if !self.testing {
            self.testing = true;
            self.cur_base_brightness = 0.0;
            self.cur_overlay_brightness = OVERLAY_BRIGHTNESS_STEP;
            self.drawing_black = true;
            self.hit_max_overlay = false;
            helper.request_redraw();
        }
    }
}

struct ConsistencyWindowHandler {
    cur_base_brightness: f32,
    cur_overlay_brightness: f32,
    testing: bool,
    drawing_black: bool,
    hit_max_overlay: bool,
    rpi: TcpStream,
    cur_low: i32,
    results: Vec<((f32, f32), (i32, i32, i32))>,
    tmp_results: Vec<i32>,
    tmp_result_base_brightness: f32,
    tmp_result_overlay_brightness: f32,
    output_results: bool,
    results_new_row: bool,
    cur_tests: usize,
}

impl WindowHandler for ConsistencyWindowHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::from_rgba(
            self.cur_base_brightness,
            self.cur_base_brightness,
            self.cur_base_brightness,
            1.0,
        ));

        if self.testing {
            println!(
                "{} : {} : {}",
                self.cur_base_brightness, self.cur_overlay_brightness, self.drawing_black
            );
            // wait for sync from rpi
            let mut buf: [u8; 6] = [0; 6]; // S + u32 + <\n>?
            let bytes_read = self.rpi.read(&mut buf).unwrap();
            if bytes_read == 0 {
                helper.terminate_loop();
                return;
            }
            if buf[0] != b'S' && (buf[0] == b'B' && bytes_read < 5) {
                helper.request_redraw();
                return;
            }

            let brightness = u32::from_le_bytes(buf[1..5].try_into().unwrap()) as i32;
            // println!("brightness: {}", brightness);

            if self.drawing_black {
                if buf[0] == b'B' {
                    self.tmp_results.push(brightness - self.cur_low);
                }

                if self.results_new_row {
                    let min = *self.tmp_results.iter().min().unwrap();
                    let max = *self.tmp_results.iter().max().unwrap();
                    let avg = self.tmp_results.iter().sum::<i32>() / self.tmp_results.len() as i32;
                    self.results.push((
                        (
                            self.tmp_result_base_brightness,
                            self.tmp_result_overlay_brightness,
                        ),
                        (min, max, avg),
                    ));
                    self.results_new_row = false;
                    self.tmp_results.clear();
                }

                if self.output_results {
                    println!("Base Brightness,Overlay Brightness, Min Diff, Max Diff, Avg Diff");
                    for ((bb, ob), (mind, maxd, ad)) in self.results.iter() {
                        println!("{:.2},{:.3},{},{},{}", bb, ob, mind, maxd, ad);
                    }
                    helper.terminate_loop();
                    return;
                }

                graphics.draw_rectangle(
                    Rectangle::from_tuples((0.0, 0.0), (SCREEN_X as f32, SCREEN_Y as f32)),
                    Color::from_rgba(0.0, 0.0, 0.0, self.cur_overlay_brightness),
                );
                self.drawing_black = false;
                // send black to rpi
                self.rpi.write_all(&[b'B']).unwrap();
            } else {
                self.cur_low = brightness;
                graphics.draw_rectangle(
                    Rectangle::from_tuples((0.0, 0.0), (SCREEN_X as f32, SCREEN_Y as f32)),
                    Color::from_rgba(1.0, 1.0, 1.0, self.cur_overlay_brightness),
                );
                self.drawing_black = true;

                if self.cur_tests < NUM_CONSISTENCY_TESTS {
                    self.cur_tests += 1;
                } else {
                    self.cur_tests = 1;
                    self.results_new_row = true;
                    self.tmp_result_base_brightness = self.cur_base_brightness;
                    self.tmp_result_overlay_brightness = self.cur_overlay_brightness;

                    if !self.hit_max_overlay {
                        let next_overlay_brightness =
                            self.cur_overlay_brightness + OVERLAY_BRIGHTNESS_STEP;
                        if next_overlay_brightness >= MAX_OVERLAY_BRIGHTNESS {
                            self.hit_max_overlay = true;
                            self.cur_overlay_brightness = MAX_OVERLAY_BRIGHTNESS;
                        } else {
                            self.cur_overlay_brightness = next_overlay_brightness;
                        }
                    } else {
                        self.hit_max_overlay = false;
                        self.cur_overlay_brightness = OVERLAY_BRIGHTNESS_STEP;
                        self.cur_base_brightness += BASE_BRIGHTNESS_STEP;

                        if self.cur_base_brightness >= 1.0 + BASE_BRIGHTNESS_STEP {
                            // self.testing = false;
                            self.output_results = true;
                        }
                    }
                }

                // send white to rpi
                self.rpi.write_all(&[b'W']).unwrap();
            }

            helper.request_redraw();
        }
    }

    fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        _button: speedy2d::window::MouseButton,
    ) {
        if !self.testing {
            self.testing = true;
            self.cur_base_brightness = 0.0;
            self.cur_overlay_brightness = OVERLAY_BRIGHTNESS_STEP;
            self.drawing_black = true;
            self.hit_max_overlay = false;
            helper.request_redraw();
        }
    }
}

fn main() {
    let window = Window::new_centered("Brightness Data", (SCREEN_X, SCREEN_Y)).unwrap();

    let mut rpi = TcpStream::connect("hardware-lab:9000").unwrap();
    rpi.write_all(&[b'R']).unwrap();

    // let results = vec![Vec::new()];
    // // results.push(Vec::new());

    // window.run_loop(BrightnessWindowHandler {
    //     cur_base_brightness: 0.0,
    //     cur_overlay_brightness: OVERLAY_BRIGHTNESS_STEP,
    //     testing: false,
    //     drawing_black: true,
    //     hit_max_overlay: false,
    //     rpi,
    //     cur_low: 0,
    //     results,
    //     output_results: false,
    //     results_new_row: false,
    // });

    window.run_loop(ConsistencyWindowHandler {
        cur_base_brightness: 0.0,
        cur_overlay_brightness: OVERLAY_BRIGHTNESS_STEP,
        testing: false,
        drawing_black: true,
        hit_max_overlay: false,
        rpi,
        cur_low: 0,
        results: vec![],
        tmp_results: vec![],
        tmp_result_base_brightness: 0.0,
        tmp_result_overlay_brightness: 0.0,
        output_results: false,
        results_new_row: false,
        cur_tests: 1,
    })
}

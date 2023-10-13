use float_cmp::ApproxEq;
use mini_tracker::{self, Point, Receiver, Table};
use speedy2d::{shape::Rectangle, window::WindowHandler};

// 1 px == 1 mm
const PX_PER_MM: i32 = 3;
// 25mm == ~1 in
const MM_PER_INCH: i32 = 25;

// grid is
// |
// |
// |
// .--------

const TABLE_WIDTH: i32 = 930;
const TABLE_HEIGHT: i32 = 523;

const RECEIVER_SIZE: f32 = 2.5 * PX_PER_MM as f32;

fn convert_y(y: f64) -> f32 {
    ((TABLE_HEIGHT as f32) - y as f32) * PX_PER_MM as f32
}

fn convert_x(x: f64) -> f32 {
    x as f32 * PX_PER_MM as f32
}

fn place_receivers() -> Vec<Receiver> {
    let mut receivers = Vec::new();

    // top/bottom
    for x in (MM_PER_INCH / 2..TABLE_WIDTH).step_by((MM_PER_INCH / 1) as usize) {
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            20.0,
            Point {
                x: x as f64,
                y: 0.0,
            },
            mini_tracker::Direction::Up,
        ));
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            20.0,
            Point {
                x: x as f64,
                y: TABLE_HEIGHT as f64,
            },
            mini_tracker::Direction::Down,
        ));
    }

    // left/right
    for y in (MM_PER_INCH / 2..TABLE_HEIGHT).step_by((MM_PER_INCH / 1) as usize) {
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            40.0,
            Point {
                x: 0.0,
                y: y as f64,
            },
            mini_tracker::Direction::Right,
        ));
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            40.0,
            Point {
                x: TABLE_WIDTH as f64,
                y: y as f64,
            },
            mini_tracker::Direction::Left,
        ));
    }

    receivers
}

struct Visualizer {
    table: Table,
    mini_location: Point,
    visible_receivers: Vec<(Receiver, bool)>,
    active_receiver_idx: Option<usize>,
    positive_intersect_checkpoints: Vec<Vec<Point>>,
    negative_intersect_checkpoints: Vec<Vec<Point>>,
    intersects_visible: bool,
    showing_negative: bool,
}

impl WindowHandler for Visualizer {
    fn on_draw(
        &mut self,
        _helper: &mut speedy2d::window::WindowHelper<()>,
        graphics: &mut speedy2d::Graphics2D,
    ) {
        graphics.clear_screen(speedy2d::color::Color::WHITE);

        let (x, y) = (self.mini_location.x, self.mini_location.y);
        let (x, y) = (convert_x(x), convert_y(y));
        graphics.draw_circle(
            (x, y),
            (MM_PER_INCH * PX_PER_MM) as f32 / 2.0,
            speedy2d::color::Color::RED,
        );

        if self.intersects_visible {
            if !self.showing_negative {
                let idx = if let Some(idx) = self.active_receiver_idx {
                    idx + 1
                } else {
                    0
                };
                for point in self.positive_intersect_checkpoints[idx].iter() {
                    let (x, y) = (point.x, point.y);
                    let (x, y) = (convert_x(x), convert_y(y));
                    graphics.draw_circle(
                        (x, y),
                        (3 * PX_PER_MM) as f32,
                        speedy2d::color::Color::CYAN,
                    );
                }
            } else {
                let idx = self.active_receiver_idx.unwrap();
                for point in self.negative_intersect_checkpoints[idx].iter() {
                    let (x, y) = (point.x, point.y);
                    let (x, y) = (convert_x(x), convert_y(y));
                    graphics.draw_circle(
                        (x, y),
                        (3 * PX_PER_MM) as f32,
                        speedy2d::color::Color::CYAN,
                    );
                }
            }
        }

        let mut active_receiver;
        for (idx, receiver) in self.table.receivers.iter().enumerate() {
            let draw_color = if let Some(active_idx) = self.active_receiver_idx {
                if idx == active_idx {
                    active_receiver = true;
                    speedy2d::color::Color::GREEN
                } else {
                    active_receiver = false;
                    speedy2d::color::Color::BLUE
                }
            } else {
                active_receiver = false;
                speedy2d::color::Color::BLUE
            };

            let (x, y) = (receiver.location.x, receiver.location.y);
            let (x, y) = (convert_x(x), convert_y(y));

            graphics.draw_rectangle(
                Rectangle::from_tuples(
                    (x - RECEIVER_SIZE, y - RECEIVER_SIZE),
                    (x + RECEIVER_SIZE, y + RECEIVER_SIZE),
                ),
                draw_color,
            );

            if active_receiver {
                let color = if self.showing_negative {
                    speedy2d::color::Color::RED
                } else {
                    speedy2d::color::Color::BLACK
                };

                // draw view_bounds

                // line 1
                let line = receiver.view_bound1;
                let (x1, y1) = (line.point1.x, line.point1.y);
                let (x1, y1) = (convert_x(x1), convert_y(y1));

                let (x2, y2) = (line.point2.x, line.point2.y);
                let (x2, y2) = (convert_x(x2), convert_y(y2));
                graphics.draw_line((x1, y1), (x2, y2), 1.0, color);

                // line 2
                let line = receiver.view_bound2;
                let (x1, y1) = (line.point1.x, line.point1.y);
                let (x1, y1) = (convert_x(x1), convert_y(y1));
                let (x2, y2) = (line.point2.x, line.point2.y);
                let (x2, y2) = (convert_x(x2), convert_y(y2));
                graphics.draw_line((x1, y1), (x2, y2), 1.0, color);
            }
        }
    }

    fn on_key_up(
        &mut self,
        helper: &mut speedy2d::window::WindowHelper<()>,
        virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
        _scancode: speedy2d::window::KeyScancode,
    ) {
        match virtual_key_code {
            Some(speedy2d::window::VirtualKeyCode::Left) => {
                if let Some(mut idx) = self.active_receiver_idx {
                    'outer: loop {
                        if let Some(sub_idx) = idx.checked_sub(1) {
                            idx = sub_idx;
                            if (self.visible_receivers[idx].1 && !self.showing_negative)
                                || (!self.visible_receivers[idx].1 && self.showing_negative)
                            {
                                self.active_receiver_idx = Some(idx);
                                break;
                            }
                        } else {
                            if self.showing_negative {
                                self.showing_negative = false;
                                for (idx, (_, can_see)) in
                                    self.visible_receivers.iter().enumerate().rev()
                                {
                                    if *can_see {
                                        println!("{idx}");
                                        self.active_receiver_idx = Some(idx);
                                        break 'outer;
                                    }
                                }
                            } else {
                                self.active_receiver_idx = None;
                                break;
                            }
                        }
                    }
                }
            }
            Some(speedy2d::window::VirtualKeyCode::Right) => {
                if let Some(mut idx) = self.active_receiver_idx {
                    while idx < self.visible_receivers.len() {
                        idx += 1;

                        if idx == self.visible_receivers.len() {
                            if !self.showing_negative {
                                self.showing_negative = true;
                                for (idx, (_, can_see)) in self.visible_receivers.iter().enumerate()
                                {
                                    if !*can_see {
                                        self.active_receiver_idx = Some(idx);
                                        break;
                                    }
                                }
                            }
                        } else {
                            if (self.visible_receivers[idx].1 && !self.showing_negative)
                                || (!self.visible_receivers[idx].1 && self.showing_negative)
                            {
                                self.active_receiver_idx = Some(idx);
                                break;
                            }
                        }
                    }
                } else {
                    for (idx, (_, can_see)) in self.visible_receivers.iter().enumerate() {
                        if *can_see {
                            self.active_receiver_idx = Some(idx);
                            break;
                        }
                    }
                }
            }
            Some(speedy2d::window::VirtualKeyCode::Space) => {
                if let Some(idx) = self.active_receiver_idx {
                    println!("{:?}", self.table.receivers[idx]);
                }
            }
            Some(speedy2d::window::VirtualKeyCode::Return) => {
                self.intersects_visible = !self.intersects_visible;
            }
            Some(speedy2d::window::VirtualKeyCode::Escape) => {
                std::process::exit(0);
            }
            _ => (),
        }

        helper.request_redraw();
    }
}

fn get_point_checkpoints(
    table: &Table,
    visible_receivers: &Vec<(Receiver, bool)>,
) -> (Vec<Vec<Point>>, Vec<Vec<Point>>) {
    let mut positive_checkpoints = Vec::new();
    let mut negative_checkpoints = Vec::new();
    let point_margin = float_cmp::F64Margin::default().epsilon(0.0001);

    // for each receiver that can see the mini, add points for all intersections created by view lines, then remove any points which cannot be seen by this receiver
    let mut bounding_lines = vec![
        table.table_top,
        table.table_bottom,
        table.table_left,
        table.table_right,
    ];

    let mut intersections: Vec<Point> = Vec::new();

    for (receiver, _) in visible_receivers.iter().filter(|(_, v)| *v) {
        for line in &bounding_lines {
            if let Some(intersect) = line.intersection(&receiver.view_bound1) {
                if intersect.approx_ne(receiver.location, point_margin) {
                    intersections.push(intersect);
                }
            }
            if let Some(intersect) = line.intersection(&receiver.view_bound2) {
                if intersect.approx_ne(receiver.location, point_margin) {
                    intersections.push(intersect);
                }
            }
        }
        intersections.push(receiver.location);

        bounding_lines.push(receiver.view_bound1);
        bounding_lines.push(receiver.view_bound2);
    }

    positive_checkpoints.push(intersections.clone());

    // for each receiver, if receiver can see mini, keep points it CAN see. if CANNOT see mini, keep points it CANNOT see
    for (receiver, mini_visible) in visible_receivers {
        if *mini_visible {
            intersections.retain(|p| receiver.can_see(p));
        } else {
        }

        positive_checkpoints.push(intersections.clone());
    }

    for (receiver, mini_visible) in visible_receivers {
        if *mini_visible {
        } else {
            intersections.retain(|p| !receiver.can_see(p));
        }

        negative_checkpoints.push(intersections.clone());
    }

    (positive_checkpoints, negative_checkpoints)
}

fn main() {
    let receivers = place_receivers();

    let table = Table::new(TABLE_WIDTH, TABLE_HEIGHT, receivers);

    let mini_location = Point { x: 37.0, y: 362.0 };

    let mut visible_receivers = Vec::new();
    for receiver in table.receivers.iter() {
        let can_see = receiver.can_see(&mini_location);
        visible_receivers.push((receiver.clone(), can_see));
    }

    let (positive_intersect_checkpoints, negative_intersect_checkpoints) =
        get_point_checkpoints(&table, &visible_receivers);

    let window = speedy2d::Window::new_centered(
        "Mini Tracker Visualizer",
        (
            (TABLE_WIDTH * PX_PER_MM) as u32,
            (TABLE_HEIGHT * PX_PER_MM) as u32,
        ),
    )
    .unwrap();

    window.run_loop(Visualizer {
        table,
        mini_location,
        visible_receivers,
        active_receiver_idx: None,
        positive_intersect_checkpoints,
        negative_intersect_checkpoints,
        intersects_visible: true,
        showing_negative: false,
    });
}

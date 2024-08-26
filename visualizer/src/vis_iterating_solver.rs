use super::*;
use mini_tracker::{self, Direction, Line, Point, Polygon, Receiver, Table};
use speedy2d::{shape::Rectangle, window::WindowHandler};

fn draw_polygon(graphics: &mut speedy2d::Graphics2D, polygon: &Polygon) {
    for point in polygon.points.iter() {
        let (x, y) = (convert_x(point.x), convert_y(point.y));
        graphics.draw_circle((x, y), 3.0 * PX_PER_MM, speedy2d::color::Color::CYAN);
    }

    for line in polygon.lines.iter() {
        let (x1, y1) = (convert_x(line.point1.x), convert_y(line.point1.y));
        let (x2, y2) = (convert_x(line.point2.x), convert_y(line.point2.y));
        graphics.draw_line((x1, y1), (x2, y2), 1.0, speedy2d::color::Color::BLACK);
    }
}

fn draw_line(graphics: &mut speedy2d::Graphics2D, line: Line) {
    let (x1, y1) = (convert_x(line.point1.x), convert_y(line.point1.y));
    let (x2, y2) = (convert_x(line.point2.x), convert_y(line.point2.y));
    graphics.draw_line((x1, y1), (x2, y2), 1.0, speedy2d::color::Color::BLACK);
}

struct MiniBounds {
    table_bounds: Polygon,
    horiz_spacing: f32,
    vert_spacing: f32,
    top_left: Option<Receiver>,
    top_right: Option<Receiver>,
    bottom_left: Option<Receiver>,
    bottom_right: Option<Receiver>,
    left_top: Option<Receiver>,
    left_bottom: Option<Receiver>,
    right_top: Option<Receiver>,
    right_bottom: Option<Receiver>,
    last_negative_top_left: Option<Receiver>,
    last_negative_top_right: Option<Receiver>,
    last_negative_bottom_left: Option<Receiver>,
    last_negative_bottom_right: Option<Receiver>,
    last_negative_left_top: Option<Receiver>,
    last_negative_left_bottom: Option<Receiver>,
    last_negative_right_top: Option<Receiver>,
    last_negative_right_bottom: Option<Receiver>,
}

impl MiniBounds {
    fn new(
        table: &Table,
        top_receivers: Vec<(Receiver, bool)>,
        bottom_receivers: Vec<(Receiver, bool)>,
        left_receivers: Vec<(Receiver, bool)>,
        right_receivers: Vec<(Receiver, bool)>,
    ) -> Self {
        fn find_positive_and_negative(
            receivers: &[(Receiver, bool)],
            forward: bool,
        ) -> (Option<Receiver>, Option<Receiver>) {
            let idx = if forward {
                receivers
                    .iter()
                    .enumerate()
                    .find_map(|(i, (_, v))| if *v { Some(i) } else { None })
            } else {
                receivers
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(i, (_, v))| if *v { Some(i) } else { None })
            };

            let positive = if let Some(idx) = idx {
                Some(receivers[idx].0)
            } else {
                None
            };

            let negative = if let Some(idx) = idx {
                if forward && idx > 0 {
                    Some(receivers[idx - 1].0)
                } else if !forward && idx < receivers.len() - 1 {
                    Some(receivers[idx + 1].0)
                } else {
                    None
                }
            } else {
                None
            };

            (positive, negative)
        }

        let (top_left, last_negative_top_left) = find_positive_and_negative(&top_receivers, true);
        let (top_right, last_negative_top_right) =
            find_positive_and_negative(&top_receivers, false);
        let (bottom_left, last_negative_bottom_left) =
            find_positive_and_negative(&bottom_receivers, true);
        let (bottom_right, last_negative_bottom_right) =
            find_positive_and_negative(&bottom_receivers, false);
        let (left_top, last_negative_left_top) = find_positive_and_negative(&left_receivers, false);
        let (left_bottom, last_negative_left_bottom) =
            find_positive_and_negative(&left_receivers, true);
        let (right_top, last_negative_right_top) =
            find_positive_and_negative(&right_receivers, false);
        let (right_bottom, last_negative_right_bottom) =
            find_positive_and_negative(&right_receivers, true);

        let table_bounds = Polygon::new(&[
            table
                .table_top
                .intersection(&table.table_left, true)
                .unwrap(),
            table
                .table_top
                .intersection(&table.table_right, true)
                .unwrap(),
            table
                .table_bottom
                .intersection(&table.table_right, true)
                .unwrap(),
            table
                .table_bottom
                .intersection(&table.table_left, true)
                .unwrap(),
        ]);

        Self {
            table_bounds,
            vert_spacing: left_receivers[1].0.location.y - left_receivers[0].0.location.y,
            horiz_spacing: top_receivers[1].0.location.x - top_receivers[0].0.location.x,
            top_left,
            top_right,
            bottom_left,
            bottom_right,
            left_top,
            left_bottom,
            right_top,
            right_bottom,
            last_negative_top_left,
            last_negative_top_right,
            last_negative_bottom_left,
            last_negative_bottom_right,
            last_negative_left_top,
            last_negative_left_bottom,
            last_negative_right_top,
            last_negative_right_bottom,
        }
    }
    fn worst_case(&self, graphics: &mut speedy2d::Graphics2D) -> Polygon {
        let bounds = [
            self.top_left.map(|r| (r.expanded_view_bound1, false)),
            self.top_right.map(|r| (r.expanded_view_bound2, false)),
            self.bottom_left.map(|r| (r.expanded_view_bound2, true)),
            self.bottom_right.map(|r| (r.expanded_view_bound1, true)),
            self.left_top.map(|r| (r.expanded_view_bound2, true)),
            self.left_bottom.map(|r| (r.expanded_view_bound1, false)),
            self.right_top.map(|r| (r.expanded_view_bound1, true)),
            self.right_bottom.map(|r| (r.expanded_view_bound2, false)),
        ];

        let mut cur_polygon = self.table_bounds.clone();

        for bound in bounds.into_iter() {
            if let Some((bounding_line, above_cut)) = bound {
                draw_line(graphics, bounding_line);
                if let Some((poly1, poly2)) = cur_polygon.bisect(bounding_line) {
                    cur_polygon = if above_cut {
                        if poly1.above_line(&bounding_line) {
                            poly1
                        } else {
                            poly2
                        }
                    } else {
                        if !poly1.above_line(&bounding_line) {
                            poly1
                        } else {
                            poly2
                        }
                    }
                }
            }
        }

        cur_polygon
    }

    // assumes no shadowing
    fn best_case(&self, graphics: &mut speedy2d::Graphics2D) -> Polygon {
        let bounds = [
            self.last_negative_top_left.map(|r| (r.view_bound1, true)),
            self.last_negative_top_right.map(|r| (r.view_bound2, true)),
            self.last_negative_bottom_left
                .map(|r| (r.view_bound2, false)),
            self.last_negative_bottom_right
                .map(|r| (r.view_bound1, false)),
            self.last_negative_left_top.map(|r| (r.view_bound2, false)),
            self.last_negative_left_bottom
                .map(|r| (r.view_bound1, true)),
            self.last_negative_right_top.map(|r| (r.view_bound1, false)),
            self.last_negative_right_bottom
                .map(|r| (r.view_bound2, true)),
            self.last_negative_top_left.map(|r| {
                (
                    r.view_bound1
                        .parallel_line(self.horiz_spacing + 25.4 / 2.0, true),
                    false,
                )
            }),
            self.last_negative_top_right.map(|r| {
                (
                    r.view_bound2
                        .parallel_line(self.horiz_spacing + 25.4 / 2.0, false),
                    false,
                )
            }),
            self.last_negative_bottom_left.map(|r| {
                (
                    r.view_bound2
                        .parallel_line(self.horiz_spacing + 25.4 / 2.0, false),
                    true,
                )
            }),
            self.last_negative_bottom_right.map(|r| {
                (
                    r.view_bound1
                        .parallel_line(self.horiz_spacing + 25.4 / 2.0, true),
                    true,
                )
            }),
            self.last_negative_left_top.map(|r| {
                (
                    r.view_bound2
                        .parallel_line(self.vert_spacing + 25.4 / 2.0, false),
                    true,
                )
            }),
            self.last_negative_left_bottom.map(|r| {
                (
                    r.view_bound1
                        .parallel_line(self.vert_spacing + 25.4 / 2.0, true),
                    false,
                )
            }),
            self.last_negative_right_top.map(|r| {
                (
                    r.view_bound1
                        .parallel_line(self.vert_spacing + 25.4 / 2.0, true),
                    true,
                )
            }),
            self.last_negative_right_bottom.map(|r| {
                (
                    r.view_bound2
                        .parallel_line(self.vert_spacing + 25.4 / 2.0, false),
                    false,
                )
            }),
        ];

        let mut cur_polygon = self.table_bounds.clone();

        for bound in bounds.into_iter() {
            if let Some((bounding_line, above_cut)) = bound {
                if let Some((poly1, poly2)) = cur_polygon.bisect(bounding_line) {
                    cur_polygon = if above_cut {
                        if poly1.above_line(&bounding_line) {
                            poly1
                        } else {
                            poly2
                        }
                    } else {
                        if !poly1.above_line(&bounding_line) {
                            poly1
                        } else {
                            poly2
                        }
                    }
                }
            }
        }

        cur_polygon
    }
}

fn draw_receiver(graphics: &mut speedy2d::Graphics2D, receiver: &Receiver, can_see: bool) {
    let draw_color = if can_see {
        speedy2d::color::Color::BLUE
    } else {
        speedy2d::color::Color::RED
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

    // // draw view_bounds

    // // line 1
    // let line = receiver.view_bound1;
    // let (x1, y1) = (line.point1.x, line.point1.y);
    // let (x1, y1) = (convert_x(x1), convert_y(y1));

    // let (x2, y2) = (line.point2.x, line.point2.y);
    // let (x2, y2) = (convert_x(x2), convert_y(y2));
    // graphics.draw_line((x1, y1), (x2, y2), 1.0, speedy2d::color::Color::RED);

    // // line 2
    // let line = receiver.view_bound2;
    // let (x1, y1) = (line.point1.x, line.point1.y);
    // let (x1, y1) = (convert_x(x1), convert_y(y1));

    // let (x2, y2) = (line.point2.x, line.point2.y);
    // let (x2, y2) = (convert_x(x2), convert_y(y2));
    // graphics.draw_line((x1, y1), (x2, y2), 1.0, speedy2d::color::Color::RED);

    // // line 1
    // let line = receiver.expanded_view_bound1;
    // let (x1, y1) = (line.point1.x, line.point1.y);
    // let (x1, y1) = (convert_x(x1), convert_y(y1));

    // let (x2, y2) = (line.point2.x, line.point2.y);
    // let (x2, y2) = (convert_x(x2), convert_y(y2));
    // graphics.draw_line((x1, y1), (x2, y2), 10.0, speedy2d::color::Color::CYAN);

    // // line 2
    // let line = receiver.expanded_view_bound2;
    // let (x1, y1) = (line.point1.x, line.point1.y);
    // let (x1, y1) = (convert_x(x1), convert_y(y1));

    // let (x2, y2) = (line.point2.x, line.point2.y);
    // let (x2, y2) = (convert_x(x2), convert_y(y2));
    // graphics.draw_line((x1, y1), (x2, y2), 10.0, speedy2d::color::Color::CYAN);
}

struct Visualizer {
    table: Table,
    mini_location: Point,
    top_receivers: Vec<(Receiver, bool)>,
    bottom_receivers: Vec<(Receiver, bool)>,
    left_receivers: Vec<(Receiver, bool)>,
    right_receivers: Vec<(Receiver, bool)>,
}

impl Visualizer {
    // solve for the simplest case, the mini is not shadowed
    fn get_bounding_polygon(&self, graphics: &mut speedy2d::Graphics2D) -> Polygon {
        let bounds = MiniBounds::new(
            &self.table,
            self.top_receivers.clone(),
            self.bottom_receivers.clone(),
            self.left_receivers.clone(),
            self.right_receivers.clone(),
        );

        // bounds.worst_case(graphics)
        bounds.best_case(graphics)
    }
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
            (MM_PER_INCH * PX_PER_MM) / 2.0,
            speedy2d::color::Color::RED,
        );

        graphics.draw_circle((x, y), 1.0 * PX_PER_MM, speedy2d::color::Color::BLACK);

        for side in [
            &self.top_receivers,
            &self.bottom_receivers,
            &self.left_receivers,
            &self.right_receivers,
        ] {
            for (receiver, can_see) in side {
                draw_receiver(graphics, receiver, *can_see);
            }
        }

        let low_res_result = self.get_bounding_polygon(graphics);
        draw_polygon(graphics, &low_res_result);
    }

    fn on_key_up(
        &mut self,
        helper: &mut speedy2d::window::WindowHelper<()>,
        virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
        _scancode: speedy2d::window::KeyScancode,
    ) {
        match virtual_key_code {
            Some(speedy2d::window::VirtualKeyCode::Left) => {
                // self.cur_receiver = self.cur_receiver.saturating_sub(1);
            }
            Some(speedy2d::window::VirtualKeyCode::Right) => {
                // self.cur_receiver = (self.cur_receiver + 1).min(self.positive_receivers.len() - 1);
            }
            Some(speedy2d::window::VirtualKeyCode::Escape) => {
                std::process::exit(0);
            }
            _ => (),
        }

        helper.request_redraw();
    }
}

pub fn run(table: Table, mini_location: Point, visible_receivers: Vec<(Receiver, bool)>) {
    let mut top_receivers: Vec<(Receiver, bool)> = visible_receivers
        .iter()
        .filter(|(r, _)| matches!(r.facing, Direction::Down))
        .map(|(r, v)| (*r, *v))
        .collect();
    top_receivers.sort_by(|(a, _), (b, _)| a.location.x.partial_cmp(&b.location.x).unwrap());

    let mut bottom_receivers: Vec<(Receiver, bool)> = visible_receivers
        .iter()
        .filter(|(r, _)| matches!(r.facing, Direction::Up))
        .map(|(r, v)| (*r, *v))
        .collect();
    bottom_receivers.sort_by(|(a, _), (b, _)| a.location.x.partial_cmp(&b.location.x).unwrap());

    let mut left_receivers: Vec<(Receiver, bool)> = visible_receivers
        .iter()
        .filter(|(r, _)| matches!(r.facing, Direction::Right))
        .map(|(r, v)| (*r, *v))
        .collect();
    left_receivers.sort_by(|(a, _), (b, _)| a.location.y.partial_cmp(&b.location.y).unwrap());

    let mut right_receivers: Vec<(Receiver, bool)> = visible_receivers
        .iter()
        .filter(|(r, _)| matches!(r.facing, Direction::Left))
        .map(|(r, v)| (*r, *v))
        .collect();
    right_receivers.sort_by(|(a, _), (b, _)| a.location.y.partial_cmp(&b.location.y).unwrap());

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
        top_receivers,
        bottom_receivers,
        left_receivers,
        right_receivers,
    });
}

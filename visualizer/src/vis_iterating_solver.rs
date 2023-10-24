use super::*;
use mini_tracker::{self, Direction, Point, Polygon, Receiver, Table};
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

enum ReceiverType {
    Positive,
    Negative,
    Ignore,
}

fn draw_receiver(
    graphics: &mut speedy2d::Graphics2D,
    receiver: &Receiver,
    receiver_type: ReceiverType,
    active: bool,
) {
    let draw_color = match receiver_type {
        ReceiverType::Positive => speedy2d::color::Color::BLUE,
        ReceiverType::Negative => speedy2d::color::Color::RED,
        ReceiverType::Ignore => speedy2d::color::Color::GRAY,
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

    if active {
        // draw view_bounds

        // line 1
        let line = receiver.view_bound1;
        let (x1, y1) = (line.point1.x, line.point1.y);
        let (x1, y1) = (convert_x(x1), convert_y(y1));

        let (x2, y2) = (line.point2.x, line.point2.y);
        let (x2, y2) = (convert_x(x2), convert_y(y2));
        graphics.draw_line((x1, y1), (x2, y2), 1.0, speedy2d::color::Color::RED);

        // line 2
        let line = receiver.view_bound2;
        let (x1, y1) = (line.point1.x, line.point1.y);
        let (x1, y1) = (convert_x(x1), convert_y(y1));

        let (x2, y2) = (line.point2.x, line.point2.y);
        let (x2, y2) = (convert_x(x2), convert_y(y2));
        graphics.draw_line((x1, y1), (x2, y2), 1.0, speedy2d::color::Color::RED);

        if matches!(receiver_type, ReceiverType::Positive) {
            // line 1
            let line = receiver.expanded_view_bound1;
            let (x1, y1) = (line.point1.x, line.point1.y);
            let (x1, y1) = (convert_x(x1), convert_y(y1));

            let (x2, y2) = (line.point2.x, line.point2.y);
            let (x2, y2) = (convert_x(x2), convert_y(y2));
            graphics.draw_line((x1, y1), (x2, y2), 10.0, speedy2d::color::Color::CYAN);

            // line 2
            let line = receiver.expanded_view_bound2;
            let (x1, y1) = (line.point1.x, line.point1.y);
            let (x1, y1) = (convert_x(x1), convert_y(y1));

            let (x2, y2) = (line.point2.x, line.point2.y);
            let (x2, y2) = (convert_x(x2), convert_y(y2));
            graphics.draw_line((x1, y1), (x2, y2), 10.0, speedy2d::color::Color::CYAN);
        }
    }
}

struct Visualizer {
    table: Table,
    mini_location: Point,
    positive_receivers: Vec<Receiver>,
    negative_receivers: Vec<Receiver>,
    ignore_receivers: Vec<Receiver>,
    cur_receiver: usize,
}

impl Visualizer {
    fn get_bounding_polygon(&self) -> Polygon {
        // for testing, group by side and sort
        let mut top_receivers: Vec<&Receiver> = self
            .positive_receivers
            .iter()
            .filter(|r| matches!(r.facing, Direction::Down))
            .collect();
        top_receivers.sort_by(|a, b| a.location.x.partial_cmp(&b.location.x).unwrap());

        let mut bottom_receivers: Vec<&Receiver> = self
            .positive_receivers
            .iter()
            .filter(|r| matches!(r.facing, Direction::Up))
            .collect();
        bottom_receivers.sort_by(|a, b| a.location.x.partial_cmp(&b.location.x).unwrap());

        let mut left_receivers: Vec<&Receiver> = self
            .positive_receivers
            .iter()
            .filter(|r| matches!(r.facing, Direction::Right))
            .collect();
        left_receivers.sort_by(|a, b| a.location.y.partial_cmp(&b.location.y).unwrap());

        let mut right_receivers: Vec<&Receiver> = self
            .positive_receivers
            .iter()
            .filter(|r| matches!(r.facing, Direction::Left))
            .collect();
        right_receivers.sort_by(|a, b| a.location.y.partial_cmp(&b.location.y).unwrap());

        let needed_receivers = [
            top_receivers.first().unwrap(),
            top_receivers.last().unwrap(),
            bottom_receivers.first().unwrap(),
            bottom_receivers.last().unwrap(),
            left_receivers.first().unwrap(),
            left_receivers.last().unwrap(),
            right_receivers.first().unwrap(),
            right_receivers.last().unwrap(),
        ];

        let mut cur_polygon = Polygon::new(&[
            self.table
                .table_top
                .intersection(&self.table.table_left, true)
                .unwrap(),
            self.table
                .table_top
                .intersection(&self.table.table_right, true)
                .unwrap(),
            self.table
                .table_bottom
                .intersection(&self.table.table_right, true)
                .unwrap(),
            self.table
                .table_bottom
                .intersection(&self.table.table_left, true)
                .unwrap(),
        ]);

        // TODO become aware of which sides receivers are on and their ordering so that we can work from outside inwards
        // for receiver in self.positive_receivers.iter().take(self.cur_receiver + 1) {
        for receiver in needed_receivers.iter() {
            if let Some((poly1, poly2)) = cur_polygon.bisect(receiver.expanded_view_bound1) {
                cur_polygon = match receiver.facing {
                    mini_tracker::Direction::Up => {
                        if poly1.min_x_point.x < poly2.min_x_point.x {
                            poly2
                        } else {
                            poly1
                        }
                    }
                    mini_tracker::Direction::Down => {
                        if poly1.min_x_point.x < poly2.min_x_point.x {
                            poly1
                        } else {
                            poly2
                        }
                    }
                    mini_tracker::Direction::Left => {
                        if poly1.min_y_point.y < poly2.min_y_point.y {
                            poly2
                        } else {
                            poly1
                        }
                    }
                    mini_tracker::Direction::Right => {
                        if poly1.min_y_point.y < poly2.min_y_point.y {
                            poly1
                        } else {
                            poly2
                        }
                    }
                }
            }
            if let Some((poly1, poly2)) = cur_polygon.bisect(receiver.expanded_view_bound2) {
                cur_polygon = match receiver.facing {
                    mini_tracker::Direction::Up => {
                        if poly1.min_x_point.x < poly2.min_x_point.x {
                            poly1
                        } else {
                            poly2
                        }
                    }
                    mini_tracker::Direction::Down => {
                        if poly1.min_x_point.x < poly2.min_x_point.x {
                            poly2
                        } else {
                            poly1
                        }
                    }
                    mini_tracker::Direction::Left => {
                        if poly1.min_y_point.y < poly2.min_y_point.y {
                            poly1
                        } else {
                            poly2
                        }
                    }
                    mini_tracker::Direction::Right => {
                        if poly1.min_y_point.y < poly2.min_y_point.y {
                            poly2
                        } else {
                            poly1
                        }
                    }
                }
            }
        }

        // Find true but irrelevant negative receivers
        // only for visualizer
        // if self.cur_receiver == self.positive_receivers.len() - 1 {

        // }

        cur_polygon
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

        for (i, receiver) in self.positive_receivers.iter().enumerate() {
            draw_receiver(
                graphics,
                receiver,
                ReceiverType::Positive,
                i == self.cur_receiver,
            );
        }

        for (i, receiver) in self.negative_receivers.iter().enumerate() {
            draw_receiver(graphics, receiver, ReceiverType::Negative, false);
        }

        let bounding_polygon = self.get_bounding_polygon();
        draw_polygon(graphics, &bounding_polygon);
    }

    fn on_key_up(
        &mut self,
        helper: &mut speedy2d::window::WindowHelper<()>,
        virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
        _scancode: speedy2d::window::KeyScancode,
    ) {
        match virtual_key_code {
            Some(speedy2d::window::VirtualKeyCode::Left) => {
                self.cur_receiver = self.cur_receiver.saturating_sub(1);
            }
            Some(speedy2d::window::VirtualKeyCode::Right) => {
                self.cur_receiver = (self.cur_receiver + 1).min(self.positive_receivers.len() - 1);
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
        positive_receivers: visible_receivers
            .iter()
            .filter(|(_, visible)| *visible)
            .map(|(r, _)| *r)
            .collect(),
        negative_receivers: visible_receivers
            .iter()
            .filter(|(_, visible)| !*visible)
            .map(|(r, _)| *r)
            .collect(),
        ignore_receivers: Vec::new(),
        cur_receiver: 0,
    });
}

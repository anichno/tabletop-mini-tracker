use super::*;
use mini_tracker::{self, Point, Polygon, Receiver, Table};
use speedy2d::window::WindowHandler;

#[derive(Clone, Copy, Debug)]
enum ViewMode {
    Initial,
    ShrinkLines,
    CenterOnly,
}

const VIEW_MODES: [ViewMode; 3] = [
    ViewMode::Initial,
    ViewMode::ShrinkLines,
    ViewMode::CenterOnly,
];

fn draw_polygon(graphics: &mut speedy2d::Graphics2D, polygon: &Polygon) {
    for point in polygon.points.iter() {
        let (x, y) = (convert_x(point.x), convert_y(point.y));
        graphics.draw_circle((x, y), 3.0 * PX_PER_MM, speedy2d::color::Color::CYAN);
    }

    for line in polygon.lines.iter() {
        let (x1, y1) = (convert_x(line.point1.x), convert_y(line.point1.y));
        let (x2, y2) = (convert_x(line.point2.x), convert_y(line.point2.y));
        graphics.draw_line((x1, y1), (x2, y2), 1.0, speedy2d::color::Color::LIGHT_GRAY);
    }
}

struct Visualizer {
    table: Table,
    mini_location: Point,
    bounding_polygon: Polygon,
    shrink_lines: Polygon,
    bounding_polygon_centered: Polygon,
    cur_view_mode: usize,
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

        let view_mode = VIEW_MODES[self.cur_view_mode];
        match view_mode {
            ViewMode::Initial => draw_polygon(graphics, &self.bounding_polygon),
            ViewMode::ShrinkLines => draw_polygon(graphics, &self.shrink_lines),
            ViewMode::CenterOnly => draw_polygon(graphics, &self.bounding_polygon_centered),
        }

        graphics.draw_circle((x, y), 1.0 * PX_PER_MM, speedy2d::color::Color::BLACK);
    }

    fn on_key_up(
        &mut self,
        helper: &mut speedy2d::window::WindowHelper<()>,
        virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
        _scancode: speedy2d::window::KeyScancode,
    ) {
        match virtual_key_code {
            Some(speedy2d::window::VirtualKeyCode::Left) => {
                self.cur_view_mode = self.cur_view_mode.saturating_sub(1);
            }
            Some(speedy2d::window::VirtualKeyCode::Right) => {
                self.cur_view_mode = (self.cur_view_mode + 1).min(VIEW_MODES.len() - 1);
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
    let bounding_polygon = table.get_bounding_polygon(&&visible_receivers[..]);
    let shrink_lines = bounding_polygon.get_shrink_lines(25.4 / 2.0);
    let bounding_polygon_centered = bounding_polygon.shrink(25.4 / 2.0);
    dbg!(&bounding_polygon_centered.points);
    dbg!(bounding_polygon.points.len());
    dbg!(bounding_polygon_centered.points.len());

    println!(
        "possible position area: {}",
        bounding_polygon_centered.area()
    );

    println!(
        "max possible error: {}",
        bounding_polygon_centered.max_width()
    );

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
        bounding_polygon, //: bounding_polygon.clone(),
        shrink_lines,
        bounding_polygon_centered, //: bounding_polygon,
        cur_view_mode: 0,
    });
}

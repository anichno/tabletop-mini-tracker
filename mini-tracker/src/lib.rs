use float_cmp::ApproxEq;

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn to_degrees(&self) -> f32 {
        match self {
            Direction::Up => 90.0,
            Direction::Down => 270.0,
            Direction::Left => 180.0,
            Direction::Right => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl float_cmp::ApproxEq for Point {
    type Margin = float_cmp::F32Margin;

    fn approx_eq<M: Into<Self::Margin>>(self, other: Self, margin: M) -> bool {
        let margin = margin.into();
        self.x.approx_eq(other.x, margin) && self.y.approx_eq(other.y, margin)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub point1: Point,
    pub point2: Point,
    pub m: f32,
    pub b: f32,
}

pub struct Table {
    pub receivers: Vec<Receiver>,
    pub table_top: Line,
    pub table_bottom: Line,
    pub table_left: Line,
    pub table_right: Line,
}

#[derive(Clone, Copy, Debug)]
pub struct Receiver {
    pub view_angle: f32,
    pub location: Point,
    pub facing: Direction,
    pub view_bound1: Line, // guaranteed to be longer than the bounds of the table
    pub view_bound2: Line,
    pub expanded_view_bound1: Line,
    pub expanded_view_bound2: Line,
    pub expanded_view_location: Point,
}

impl Table {
    pub fn new(table_width: f32, table_height: f32, receivers: Vec<Receiver>) -> Self {
        let table_top = Line::new(
            Point {
                x: 0.0,
                y: table_height,
            },
            Point {
                x: table_width,
                y: table_height,
            },
        );
        let table_bottom = Line::new(
            Point { x: 0.0, y: 0.0 },
            Point {
                x: table_width,
                y: 0.0,
            },
        );
        let table_left = Line::new(
            Point { x: 0.0, y: 0.0 },
            Point {
                x: 0.0,
                y: table_height,
            },
        );
        let table_right = Line::new(
            Point {
                x: table_width,
                y: 0.0,
            },
            Point {
                x: table_width,
                y: table_height,
            },
        );

        Self {
            receivers,
            table_top,
            table_bottom,
            table_left,
            table_right,
        }
    }
    pub fn send_sync(&self) {}

    fn receivers_can_see_estimated(&self, receivers: &[&Receiver], point: &Point) -> bool {
        for receiver in receivers {
            if !receiver.can_see_estimated(point) {
                return false;
            }
        }
        true
    }

    pub fn get_location(&self, receivers: &[(&Receiver, bool)]) -> (Point, f32) {
        let point_margin = float_cmp::F32Margin::default().epsilon(0.0001);

        // for each receiver that can see the mini, add points for all intersections created by view lines, then remove any points which cannot be seen by this receiver
        let mut bounding_lines = vec![
            self.table_top,
            self.table_bottom,
            self.table_left,
            self.table_right,
        ];

        let mut intersections: Vec<Point> = Vec::new();

        let can_see_receivers: Vec<&Receiver> = receivers
            .iter()
            .filter(|(_, v)| *v)
            .map(|(r, _)| *r)
            .collect();

        // Note: by testing each point as we go against receivers, we can save a lot of memory for really not much of a performance hit
        for (receiver, _) in receivers.iter() {
            for line in &bounding_lines {
                if let Some(intersect) = line.intersection(&receiver.expanded_view_bound1, true) {
                    if self.receivers_can_see_estimated(&can_see_receivers, &intersect) {
                        intersections.push(intersect);
                    }
                }
                if let Some(intersect) = line.intersection(&receiver.expanded_view_bound2, true) {
                    if self.receivers_can_see_estimated(&can_see_receivers, &intersect) {
                        intersections.push(intersect);
                    }
                }

                if let Some(intersect) = line.intersection(&receiver.view_bound1, true) {
                    if intersect.approx_ne(receiver.location, point_margin) {
                        if self.receivers_can_see_estimated(&can_see_receivers, &intersect) {
                            intersections.push(intersect);
                        }
                    }
                }
                if let Some(intersect) = line.intersection(&receiver.view_bound2, true) {
                    if intersect.approx_ne(receiver.location, point_margin) {
                        if self.receivers_can_see_estimated(&can_see_receivers, &intersect) {
                            intersections.push(intersect);
                        }
                    }
                }
            }
            if self.receivers_can_see_estimated(&can_see_receivers, &receiver.location) {
                intersections.push(receiver.location);
            }

            bounding_lines.push(receiver.expanded_view_bound1);
            bounding_lines.push(receiver.expanded_view_bound2);
            bounding_lines.push(receiver.view_bound1);
            bounding_lines.push(receiver.view_bound2);
        }

        for (receiver, _) in receivers.iter().filter(|(_, v)| !*v) {
            intersections.retain(|p| receiver.cannot_see(p));
        }

        assert!(!intersections.is_empty());

        // find the centroid of all remaining points
        let mut x_sum = 0.0;
        let mut y_sum = 0.0;
        for point in &intersections {
            x_sum += point.x;
            y_sum += point.y;
        }
        x_sum /= intersections.len() as f32;
        y_sum /= intersections.len() as f32;

        // TODO: This error calc isn't quite right. We know that the bounds of the mini are within the polygon described by all remaining points
        // find the max distance from the centroid
        let mut max_distance = 0.0;
        for point in &intersections {
            let distance = Point { x: x_sum, y: y_sum }.distance(point);
            if distance > max_distance {
                max_distance = distance;
            }
        }

        (Point { x: x_sum, y: y_sum }, max_distance)
    }
}

impl Receiver {
    pub fn new(
        table_width: f32,
        table_height: f32,
        view_angle: f32,
        location: Point,
        facing: Direction,
    ) -> Self {
        let angle1 = (facing.to_degrees() + (view_angle) / 2.0).to_radians();
        let angle2 = (facing.to_degrees() - (view_angle) / 2.0).to_radians();
        let distance = (table_width + table_height) * 2.0;
        let far_point1 = Point {
            x: (distance * angle1.cos()) + location.x,
            y: (distance * angle1.sin()) + location.y,
        };
        let far_point2 = Point {
            x: (distance * angle2.cos()) + location.x,
            y: (distance * angle2.sin()) + location.y,
        };

        let too_long1 = Line::new(location, far_point1);
        let too_long2 = Line::new(location, far_point2);

        let expanded_view_bound1 = too_long1.parallel_line(25.4, true);
        let expanded_view_bound2 = too_long2.parallel_line(25.4, false);
        let expanded_view_location = expanded_view_bound1
            .intersection(&expanded_view_bound2, false)
            .unwrap();

        Self {
            view_angle,
            location,
            facing,
            view_bound1: too_long1,
            view_bound2: too_long2,
            expanded_view_bound1,
            expanded_view_bound2,
            expanded_view_location,
        }
    }

    pub fn can_see(&self, point: &Point) -> bool {
        let mut angle = self.location.angle(point);

        match self.facing {
            Direction::Up | Direction::Left | Direction::Down => {
                if angle < 0.0 {
                    angle += 360.0;
                }
                angle >= self.facing.to_degrees() - (self.view_angle / 2.0) - 0.01
                    && angle <= self.facing.to_degrees() + (self.view_angle / 2.0) + 0.01
            }
            Direction::Right => angle.abs() <= (self.view_angle / 2.0) + 0.01,
        }
    }

    pub fn can_see_estimated(&self, point: &Point) -> bool {
        let mut angle = self.expanded_view_location.angle(point);

        match self.facing {
            Direction::Up | Direction::Left | Direction::Down => {
                if angle < 0.0 {
                    angle += 360.0;
                }
                angle >= self.facing.to_degrees() - (self.view_angle / 2.0) - 0.01
                    && angle <= self.facing.to_degrees() + (self.view_angle / 2.0) + 0.01
            }
            Direction::Right => angle.abs() <= (self.view_angle / 2.0) + 0.01,
        }
    }

    pub fn cannot_see(&self, point: &Point) -> bool {
        let mut angle = self.location.angle(point);

        match self.facing {
            Direction::Up | Direction::Left | Direction::Down => {
                if angle < 0.0 {
                    angle += 360.0;
                }
                !(angle >= self.facing.to_degrees() - (self.view_angle / 2.0) + 0.01
                    && angle <= self.facing.to_degrees() + (self.view_angle / 2.0) - 0.01)
            }
            Direction::Right => !(angle.abs() <= (self.view_angle / 2.0) - 0.01),
        }
    }
}

impl Point {
    pub fn distance(&self, other: &Self) -> f32 {
        ((self.x - other.x).powf(2.0) + (self.y - other.y).powf(2.0)).sqrt()
    }

    pub fn rotate_around_origin(&self, angle: f32) -> Self {
        let orig_x = self.x;
        let orig_y = self.y;
        let x = orig_x * angle.cos() - orig_y * angle.sin();
        let y = orig_y * angle.cos() + orig_x * angle.sin();
        Point { x, y }
    }

    pub fn angle(&self, other: &Self) -> f32 {
        let x = other.x - self.x;
        let y = other.y - self.y;

        y.atan2(x).to_degrees()
    }

    pub fn angle_from_origin(&self) -> f32 {
        let x = self.x;
        let y = self.y;

        y.atan2(x).to_degrees()
    }
}

impl Line {
    pub fn new(point1: Point, point2: Point) -> Self {
        let x1 = point1.x;
        let y1 = point1.y;
        let x2 = point2.x;
        let y2 = point2.y;

        let m = (y2 - y1) / (x2 - x1);
        let b = y1 - m * x1;

        Self {
            point1,
            point2,
            m,
            b,
        }
    }

    pub fn intersection(&self, other: &Self, on_segments_only: bool) -> Option<Point> {
        if self.m == other.m {
            // Parallel or coincident
            return None;
        }

        let x: f32;
        let y: f32;

        if self.m.is_infinite() {
            x = self.point1.x;
            y = other.m * x + other.b;
        } else if other.m.is_infinite() {
            x = other.point1.x;
            y = self.m * x + self.b;
        } else {
            x = (other.b - self.b) / (self.m - other.m);
            y = self.m * x + self.b;
        }
        // let x = x.round() as i32;
        // let y = y.round() as i32;

        if on_segments_only {
            // check if on segment 1
            if x < self.point1.x.min(self.point2.x) || x > self.point1.x.max(self.point2.x) {
                return None;
            }
            if y < self.point1.y.min(self.point2.y) || y > self.point1.y.max(self.point2.y) {
                return None;
            }

            // check if on segment 2
            if x < other.point1.x.min(other.point2.x) || x > other.point1.x.max(other.point2.x) {
                return None;
            }
            if y < other.point1.y.min(other.point2.y) || y > other.point1.y.max(other.point2.y) {
                return None;
            }
        }

        Some(Point { x, y })
    }

    pub fn parallel_line(&self, distance: f32, left: bool) -> Line {
        let mut angle = self.point1.angle(&self.point2);
        if angle < 0.0 {
            angle += 360.0;
        }
        if left {
            angle += 90.0
        } else {
            angle -= 90.0
        }

        let rad_angle = angle.to_radians();
        let new_point1 = Point {
            x: (distance * rad_angle.cos()) + self.point1.x,
            y: (distance * rad_angle.sin()) + self.point1.y,
        };
        let new_point2 = Point {
            x: (distance * rad_angle.cos()) + self.point2.x,
            y: (distance * rad_angle.sin()) + self.point2.y,
        };

        Line::new(new_point1, new_point2)
    }
}

#[cfg(test)]
mod test {
    use float_cmp::assert_approx_eq;

    use super::*;

    #[test]
    fn distance() {
        let point1 = Point { x: 1.0, y: 1.0 };
        let point2 = Point { x: 4.0, y: 5.0 };

        assert_eq!(point1.distance(&point2), 5.0);
        assert_eq!(point2.distance(&point1), 5.0);
    }

    #[test]
    fn can_see() {
        let point = Point { x: 100.0, y: 100.0 };

        let up = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 100.0, y: 0.0 },
            Direction::Up,
        );
        assert!(up.can_see(&point));

        let right = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 0.0, y: 100.0 },
            Direction::Right,
        );
        assert!(right.can_see(&point));

        let down = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 100.0, y: 200.0 },
            Direction::Down,
        );
        assert!(down.can_see(&point));

        let left = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 200.0, y: 100.0 },
            Direction::Left,
        );
        assert!(left.can_see(&point));
    }

    #[test]
    fn cannot_see() {
        let point = Point { x: 100.0, y: 100.0 };

        let up = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 202.0, y: 0.0 },
            Direction::Up,
        );
        assert!(!up.can_see(&point));

        let right = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 0.0, y: 202.0 },
            Direction::Right,
        );
        assert!(!right.can_see(&point));

        let down = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 202.0, y: 200.0 },
            Direction::Down,
        );
        assert!(!down.can_see(&point));

        let left = Receiver::new(
            200.0,
            200.0,
            45.0,
            Point { x: 200.0, y: 202.0 },
            Direction::Left,
        );
        assert!(!left.can_see(&point));
    }

    #[test]
    fn rotate_around_origin() {
        let point = Point { x: 2.0, y: 2.0 };

        let rotated = point.rotate_around_origin(90.0 * (std::f32::consts::PI / 180.0));
        assert_approx_eq!(f32, rotated.x, -2.0, epsilon = 0.00001);
        assert_approx_eq!(f32, rotated.y, 2.0, epsilon = 0.00001);
    }

    #[test]
    fn angle() {
        let origin = Point { x: 0.0, y: 0.0 };
        let point = Point { x: 8.0, y: 8.0 };

        assert_eq!(origin.angle(&point), 45.0)
    }

    #[test]
    fn angle_from_origin() {
        let point = Point { x: 8.0, y: 8.0 };

        assert_eq!(point.angle_from_origin(), 45.0)
    }

    #[test]
    fn line_segment_intersect() {
        let p1 = Point { x: 1.0, y: 2.0 };
        let p2 = Point { x: 6.0, y: 0.0 };
        let line1 = Line::new(p1, p2);

        let p1 = Point { x: 3.0, y: 1.0 };
        let p2 = Point { x: 7.0, y: 8.0 };
        let line2 = Line::new(p1, p2);

        let intersect = line1.intersection(&line2, true).unwrap();
        assert_approx_eq!(f32, intersect.x, 3.09302, epsilon = 0.00001);
        assert_approx_eq!(f32, intersect.y, 1.16279, epsilon = 0.00001);

        let p1 = Point { x: 0.0, y: 0.0 };
        let p2 = Point { x: 200.0, y: 200.0 };
        let line1 = Line::new(p1, p2);

        let p1 = Point { x: 0.0, y: 200.0 };
        let p2 = Point { x: 200.0, y: 0.0 };
        let line2 = Line::new(p1, p2);

        let intersect = line1.intersection(&line2, true).unwrap();
        assert_eq!(intersect.x, 100.0);
        assert_eq!(intersect.y, 100.0);
    }

    #[test]
    fn receivers_can_see_own_intersections() {
        const MM_PER_INCH: f32 = 25.4;
        const TABLE_WIDTH: f32 = 930.0;
        const TABLE_HEIGHT: f32 = 523.0;

        let point_margin = float_cmp::F32Margin::default().epsilon(0.0001);

        // assume all equal view angles and spacing, 1 sensor per inch
        let mut receivers = Vec::new();

        // top/bottom
        let mut x = MM_PER_INCH / 2.0;
        while x < TABLE_WIDTH {
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                30.0,
                Point { x, y: 0.0 },
                Direction::Up,
            ));
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                30.0,
                Point { x, y: TABLE_HEIGHT },
                Direction::Down,
            ));

            x += MM_PER_INCH;
        }

        // left/right
        let mut y = MM_PER_INCH / 2.0;
        while y < TABLE_HEIGHT {
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                20.0,
                Point { x: 0.0, y },
                Direction::Right,
            ));
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                20.0,
                Point { x: TABLE_WIDTH, y },
                Direction::Left,
            ));

            y += MM_PER_INCH;
        }

        let table = Table::new(TABLE_WIDTH, TABLE_HEIGHT, receivers);

        let bounding_lines = [
            table.table_top,
            table.table_bottom,
            table.table_left,
            table.table_right,
        ];

        for receiver in &table.receivers {
            let mut intersections = 0;
            for line in &bounding_lines {
                if let Some(intersect) = line.intersection(&receiver.view_bound1, true) {
                    // dbg!(intersect);
                    if intersect.approx_ne(receiver.location, point_margin) {
                        // dbg!(line);
                        intersections += 1;
                    }
                }
                if let Some(intersect) = line.intersection(&receiver.view_bound2, true) {
                    // dbg!(intersect);
                    if intersect.approx_ne(receiver.location, point_margin) {
                        // dbg!(line);
                        intersections += 1;
                    }
                }
            }

            // always add intersect for receiver on table bounds
            intersections += 1;

            // dbg!(receiver.location, receiver.facing, intersections);
            assert_eq!(intersections, 3);
        }
    }
}

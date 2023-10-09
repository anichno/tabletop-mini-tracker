#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn to_degrees(&self) -> i32 {
        match self {
            Direction::Up => 90,
            Direction::Down => -90,
            Direction::Left => 180,
            Direction::Right => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub point1: Point,
    pub point2: Point,
    pub m: f64,
    pub b: f64,
}

pub struct Table {
    pub receivers: Vec<Receiver>,
    table_top: Line,
    table_bottom: Line,
    table_left: Line,
    table_right: Line,
}

pub struct Receiver {
    pub view_angle: i32,
    pub location: Point,
    pub facing: Direction,
    view_bound1: Line, // guaranteed to be longer than the bounds of the table
    view_bound2: Line,
}

impl Table {
    pub fn new(table_width: i32, table_height: i32, receivers: Vec<Receiver>) -> Self {
        let table_top = Line::new(
            Point {
                x: 0,
                y: table_height,
            },
            Point {
                x: table_width,
                y: table_height,
            },
        );
        let table_bottom = Line::new(
            Point { x: 0, y: 0 },
            Point {
                x: table_width,
                y: 0,
            },
        );
        let table_left = Line::new(
            Point { x: 0, y: 0 },
            Point {
                x: 0,
                y: table_height,
            },
        );
        let table_right = Line::new(
            Point {
                x: table_width,
                y: 0,
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

    pub fn get_location(&self, receivers: &[(&Receiver, bool)]) -> (Point, i32) {
        // for each receiver that can see the mini, add points for all intersections created by view lines, then remove any points which cannot be seen by this receiver
        let mut bounding_lines = vec![
            self.table_top,
            self.table_bottom,
            self.table_left,
            self.table_right,
        ];

        let mut intersections: Vec<Point> = Vec::new();

        for (receiver, _) in receivers.iter().filter(|(_, v)| *v) {
            for line in &bounding_lines {
                if let Some(intersect) = line.intersection(&receiver.view_bound1) {
                    if intersect != receiver.location {
                        intersections.push(intersect);
                    }
                }
                if let Some(intersect) = line.intersection(&receiver.view_bound2) {
                    if intersect != receiver.location {
                        intersections.push(intersect);
                    }
                }
            }
            intersections.push(receiver.location);

            bounding_lines.push(receiver.view_bound1);
            bounding_lines.push(receiver.view_bound2);
        }

        // for each receiver, if receiver can see mini, keep points it CAN see. if CANNOT see mini, keep points it CANNOT see
        for (receiver, mini_visible) in receivers {
            if *mini_visible {
                intersections.retain(|p| receiver.can_see(p));
            } else {
                // intersections.retain(|p| !receiver.can_see(p));
            }
        }

        // find the centroid of all remaining points
        let mut x_sum = 0;
        let mut y_sum = 0;
        for point in &intersections {
            x_sum += point.x;
            y_sum += point.y;
        }
        x_sum /= intersections.len() as i32;
        y_sum /= intersections.len() as i32;

        // find the max distance from the centroid
        let mut max_distance = 0;
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
        table_width: i32,
        table_height: i32,
        view_angle: i32,
        location: Point,
        facing: Direction,
    ) -> Self {
        let angle1 = (facing.to_degrees() as f64 + (view_angle as f64) / 2.0).to_radians();
        let angle2 = (facing.to_degrees() as f64 - (view_angle as f64) / 2.0).to_radians();
        let distance = ((table_width + table_height) * 2) as f64;
        let far_point1 = Point {
            x: (distance * angle1.cos()).round() as i32,
            y: (distance * angle1.sin()).round() as i32,
        };
        let far_point2 = Point {
            x: (distance * angle2.cos()).round() as i32,
            y: (distance * angle2.sin()).round() as i32,
        };

        let too_long1 = Line::new(location, far_point1);
        let too_long2 = Line::new(location, far_point2);

        Self {
            view_angle,
            location,
            facing,
            view_bound1: too_long1,
            view_bound2: too_long2,
        }
    }

    pub fn can_see(&self, point: &Point) -> bool {
        // move receiver to origin and do corresponding move to point
        // can assume location is always positive Q1
        let moved_point = Point {
            x: point.x - self.location.x,
            y: point.y - self.location.y,
        };

        // rotate so view angle is looking to the right
        let rotate_rad = match self.facing {
            Direction::Up => -90.0 * (std::f64::consts::PI / 180.0),
            Direction::Down => 90.0 * (std::f64::consts::PI / 180.0),
            Direction::Left => 180.0 * (std::f64::consts::PI / 180.0),
            Direction::Right => 0.0 * (std::f64::consts::PI / 180.0),
        };

        let rotated_point = moved_point.rotate_around_origin(rotate_rad);

        // check if point angle is within fov
        let angle = rotated_point.angle_from_origin();
        angle <= (self.view_angle / 2) + 1 && angle >= -(self.view_angle / 2) - 1
    }
}

impl Point {
    pub fn distance(&self, other: &Self) -> i32 {
        (((self.x - other.x).pow(2) + (self.y - other.y).pow(2)) as f64).sqrt() as i32
    }

    pub fn rotate_around_origin(&self, angle: f64) -> Self {
        let orig_x = self.x as f64;
        let orig_y = self.y as f64;
        let x = orig_x * angle.cos() - orig_y * angle.sin();
        let y = orig_y * angle.cos() + orig_x * angle.sin();
        Point {
            x: x.round() as i32,
            y: y.round() as i32,
        }
    }

    pub fn angle(&self, other: &Self) -> i32 {
        let x = (other.x - self.x) as f64;
        let y = (other.y - self.y) as f64;

        y.atan2(x).to_degrees().round() as i32
    }

    pub fn angle_from_origin(&self) -> i32 {
        let x = self.x as f64;
        let y = self.y as f64;

        y.atan2(x).to_degrees().round() as i32
    }
}

impl Line {
    pub fn new(point1: Point, point2: Point) -> Self {
        let x1 = point1.x as f64;
        let y1 = point1.y as f64;
        let x2 = point2.x as f64;
        let y2 = point2.y as f64;

        let m = (y2 - y1) / (x2 - x1);
        let b = y1 - m * x1;

        Self {
            point1,
            point2,
            m,
            b,
        }
    }

    pub fn intersection(&self, other: &Self) -> Option<Point> {
        if self.m == other.m {
            // Parallel or coincident
            return None;
        }

        let x: f64;
        let y: f64;

        if self.m.is_infinite() {
            x = self.point1.x as f64;
            y = other.m * x + other.b;
        } else if other.m.is_infinite() {
            x = other.point1.x as f64;
            y = self.m * x + self.b;
        } else {
            x = (other.b - self.b) / (self.m - other.m);
            y = self.m * x + self.b;
        }
        let x = x.round() as i32;
        let y = y.round() as i32;

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

        Some(Point { x, y })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn distance() {
        let point1 = Point { x: 1, y: 1 };
        let point2 = Point { x: 4, y: 5 };

        assert_eq!(point1.distance(&point2), 5);
        assert_eq!(point2.distance(&point1), 5);
    }

    #[test]
    fn can_see() {
        let point = Point { x: 100, y: 100 };

        let up = Receiver::new(200, 200, 45, Point { x: 100, y: 0 }, Direction::Up);
        assert!(up.can_see(&point));

        let right = Receiver::new(200, 200, 45, Point { x: 0, y: 100 }, Direction::Right);
        assert!(right.can_see(&point));

        let down = Receiver::new(200, 200, 45, Point { x: 100, y: 200 }, Direction::Down);
        assert!(down.can_see(&point));

        let left = Receiver::new(200, 200, 45, Point { x: 200, y: 100 }, Direction::Left);
        assert!(left.can_see(&point));
    }

    #[test]
    fn cannot_see() {
        let point = Point { x: 100, y: 100 };

        let up = Receiver::new(200, 200, 45, Point { x: 202, y: 0 }, Direction::Up);
        assert!(!up.can_see(&point));

        let right = Receiver::new(200, 200, 45, Point { x: 0, y: 202 }, Direction::Right);
        assert!(!right.can_see(&point));

        let down = Receiver::new(200, 200, 45, Point { x: 202, y: 200 }, Direction::Down);
        assert!(!down.can_see(&point));

        let left = Receiver::new(200, 200, 45, Point { x: 200, y: 202 }, Direction::Left);
        assert!(!left.can_see(&point));
    }

    #[test]
    fn rotate_around_origin() {
        let point = Point { x: 2, y: 2 };

        let rotated = point.rotate_around_origin(90.0 * (std::f64::consts::PI / 180.0));
        assert_eq!(rotated.x, -2);
        assert_eq!(rotated.y, 2);
    }

    #[test]
    fn angle() {
        let origin = Point { x: 0, y: 0 };
        let point = Point { x: 8, y: 8 };

        assert_eq!(origin.angle(&point), 45)
    }

    #[test]
    fn angle_from_origin() {
        let point = Point { x: 8, y: 8 };

        assert_eq!(point.angle_from_origin(), 45)
    }

    #[test]
    fn line_segment_intersect() {
        let p1 = Point { x: 1, y: 2 };
        let p2 = Point { x: 6, y: 0 };
        let line1 = Line::new(p1, p2);

        let p1 = Point { x: 3, y: 1 };
        let p2 = Point { x: 7, y: 8 };
        let line2 = Line::new(p1, p2);

        let intersect = line1.intersection(&line2).unwrap();
        assert_eq!(intersect.x, 3);
        assert_eq!(intersect.y, 1);

        let p1 = Point { x: 0, y: 0 };
        let p2 = Point { x: 200, y: 200 };
        let line1 = Line::new(p1, p2);

        let p1 = Point { x: 0, y: 200 };
        let p2 = Point { x: 200, y: 0 };
        let line2 = Line::new(p1, p2);

        let intersect = line1.intersection(&line2).unwrap();
        assert_eq!(intersect.x, 100);
        assert_eq!(intersect.y, 100);
    }

    #[test]
    fn receivers_can_see_own_intersections() {
        const MM_PER_INCH: i32 = 25;
        const TABLE_WIDTH: i32 = 930;
        const TABLE_HEIGHT: i32 = 523;

        // assume all equal view angles and spacing, 1 sensor per inch
        let mut receivers = Vec::new();

        // top/bottom
        for x in
            (MM_PER_INCH / 2..TABLE_WIDTH - (MM_PER_INCH / 2)).step_by((MM_PER_INCH / 1) as usize)
        {
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                20,
                Point { x, y: 0 },
                Direction::Up,
            ));
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                20,
                Point { x, y: TABLE_HEIGHT },
                Direction::Down,
            ));
        }

        // left/right
        for y in
            (MM_PER_INCH / 2..TABLE_HEIGHT - (MM_PER_INCH / 2)).step_by((MM_PER_INCH / 1) as usize)
        {
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                20,
                Point { x: 0, y },
                Direction::Right,
            ));
            receivers.push(Receiver::new(
                TABLE_WIDTH,
                TABLE_HEIGHT,
                20,
                Point { x: TABLE_WIDTH, y },
                Direction::Left,
            ));
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
                if let Some(intersect) = line.intersection(&receiver.view_bound1) {
                    if intersect != receiver.location {
                        dbg!(line);
                        intersections += 1;
                    }
                }
                if let Some(intersect) = line.intersection(&receiver.view_bound2) {
                    if intersect != receiver.location {
                        dbg!(line);
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

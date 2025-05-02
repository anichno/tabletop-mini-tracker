#![no_std]

use arrayvec::ArrayVec;
use libm::{atan2f, cosf, powf, sinf, sqrtf};

const IMAGE_WIDTH: usize = 36;
const IMAGE_HEIGHT: usize = 36;
const GRID_SPACING: f32 = 7.666666666666667;
const DOT_CENTER_TO_GRID: f32 = GRID_SPACING / 3.0;

const MAX_BLOBS: usize = 64;
const MAX_POINTS_IN_BLOB: usize = 50;

type Image = [[u8; IMAGE_WIDTH]; IMAGE_HEIGHT];

#[derive(Debug)]
pub enum CodePoint {
    Up,
    Down,
    Left,
    Right,
    Unknown,
    NotSeen,
}

impl Into<char> for CodePoint {
    fn into(self) -> char {
        match self {
            CodePoint::Up => 'U',
            CodePoint::Down => 'D',
            CodePoint::Left => 'L',
            CodePoint::Right => 'R',
            CodePoint::Unknown => '!',
            CodePoint::NotSeen => '*',
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
struct Point {
    x: f32,
    y: f32,
}

impl Point {
    fn distance(&self, other: &Self) -> f32 {
        sqrtf(powf(self.x - other.x, 2.0) + powf(self.y - other.y, 2.0))
    }

    fn angle_90(&self, other: &Self) -> f32 {
        let dx: f32 = self.x - other.x;
        let dy = self.y - other.y;

        let radians = atan2f(dy, dx);
        let mut degrees = radians.to_degrees();
        if degrees < 0.0 {
            degrees += 90.0
        }

        degrees
    }

    fn angle_360(&self, other: &Self) -> f32 {
        let dx: f32 = other.x - self.x;
        let dy = other.y - self.y;

        let radians = atan2f(dy, dx);
        radians.to_degrees()
    }

    fn rotate(&mut self, degrees: f32) {
        let radians = degrees.to_radians();

        let x_new = self.x * cosf(radians) - self.y * sinf(radians);
        let y_new = self.x * sinf(radians) + self.y * cosf(radians);

        self.x = x_new;
        self.y = y_new;
    }
}

#[derive(Debug)]
struct Blob {
    id: Option<u8>,
    points: ArrayVec<Point, MAX_POINTS_IN_BLOB>,
}

impl Blob {
    fn max_length(&self) -> f32 {
        let mut max_len: f32 = 0.0;

        for (point1_idx, point1) in self.points.iter().enumerate() {
            for point2 in self.points.iter().skip(point1_idx + 1) {
                let distance = point1.distance(point2);
                max_len = max_len.max(distance);
            }
        }

        max_len
    }

    fn split(self) -> (Self, Self) {
        let mut max_len: f32 = 0.0;
        let mut max_point1 = &Point::default();
        let mut max_point2 = &Point::default();

        for (point1_idx, point1) in self.points.iter().enumerate() {
            for point2 in self.points.iter().skip(point1_idx + 1) {
                let distance = point1.distance(point2);
                if distance > max_len {
                    max_len = distance;
                    max_point1 = point1;
                    max_point2 = point2;
                }
                max_len = max_len.max(distance);
            }
        }

        max_len = 0.0;
        let mut max_point3 = &Point::default();
        let mut max_point4 = &Point::default();

        for (point1_idx, point1) in self.points.iter().enumerate() {
            if point1 == max_point1 || point1 == max_point2 {
                continue;
            }
            for point2 in self.points.iter().skip(point1_idx + 1) {
                if point2 == max_point1 || point2 == max_point2 {
                    continue;
                }
                let distance = point1.distance(point2);
                if distance > max_len {
                    max_len = distance;
                    max_point3 = point1;
                    max_point4 = point2;
                }
                max_len = max_len.max(distance);
            }
        }

        let distance1 = max_point1.distance(max_point3);
        let distance2 = max_point1.distance(max_point4);

        let (midpoint1, midpoint2) = if distance1 < distance2 {
            let midpoint1 = Point {
                x: (max_point1.x + max_point3.x) / 2.0,
                y: (max_point1.y + max_point3.y) / 2.0,
            };
            let midpoint2 = Point {
                x: (max_point2.x + max_point4.x) / 2.0,
                y: (max_point2.y + max_point4.y) / 2.0,
            };
            (midpoint1, midpoint2)
        } else {
            let midpoint1 = Point {
                x: (max_point1.x + max_point4.x) / 2.0,
                y: (max_point1.y + max_point4.y) / 2.0,
            };
            let midpoint2 = Point {
                x: (max_point2.x + max_point3.x) / 2.0,
                y: (max_point2.y + max_point3.y) / 2.0,
            };
            (midpoint1, midpoint2)
        };
        let mut blob1 = Blob {
            id: None,
            points: ArrayVec::<Point, MAX_POINTS_IN_BLOB>::new(),
        };
        let mut blob2 = Blob {
            id: None,
            points: ArrayVec::<Point, MAX_POINTS_IN_BLOB>::new(),
        };

        for point in self.points.iter() {
            let distance1 = point.distance(&midpoint1);
            let distance2 = point.distance(&midpoint2);
            if distance1 < distance2 {
                blob1.points.push(*point);
            } else {
                blob2.points.push(*point);
            }
        }

        (blob1, blob2)
    }

    fn center(&self) -> Point {
        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        for point in self.points.iter() {
            avg_x += point.x;
            avg_y += point.y;
        }

        avg_x /= self.points.len() as f32;
        avg_y /= self.points.len() as f32;

        Point { x: avg_x, y: avg_y }
    }
}

fn threshold(image: &Image, threshold_val: u8) -> Image {
    let mut thresholded: Image = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];

    for (y, row) in image.iter().enumerate() {
        for (x, col) in row.iter().enumerate() {
            if *col < threshold_val {
                thresholded[y][x] = 1;
            } else {
                thresholded[y][x] = 0;
            }
        }
    }

    thresholded
}

fn flood_fill(image: &mut Image, x: usize, y: usize, blob: &mut Blob) {
    image[y][x] = blob.id.unwrap();
    blob.points.push(Point {
        x: x as f32,
        y: y as f32,
    });

    for ydiff in -1..=1 {
        for xdiff in -1..=1 {
            if ydiff == 0 && xdiff == 0 {
                continue;
            }
            if let (Some(new_x), Some(new_y)) =
                (x.checked_add_signed(xdiff), y.checked_add_signed(ydiff))
            {
                if new_y < IMAGE_HEIGHT && new_x < IMAGE_WIDTH && image[new_y][new_x] == 1 {
                    flood_fill(image, new_x, new_y, blob);
                }
            }
        }
    }
}

fn label_blobs(mut image: Image) -> ArrayVec<Blob, MAX_BLOBS> {
    let mut blobs = ArrayVec::new();
    let mut cur_blob_id = 2;
    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            if image[y][x] == 1 {
                // need to label this pixel
                let mut cur_blob = Blob {
                    id: Some(cur_blob_id),
                    points: ArrayVec::new(),
                };
                flood_fill(&mut image, x, y, &mut cur_blob);
                blobs.push(cur_blob);
                cur_blob_id += 1;
            }
        }
    }

    blobs
}

fn cluster_pairs(
    mut point_pairs: ArrayVec<(Point, Point, f32), 50>,
    tolerance: f32,
) -> ArrayVec<ArrayVec<(Point, Point, f32), 20>, 10> {
    point_pairs.sort_unstable_by(|p1, p2| p1.2.total_cmp(&p2.2));
    let mut clusters = ArrayVec::new();

    let mut cur_cluster = ArrayVec::new();
    cur_cluster.push(point_pairs[0]);
    let mut cur_angle = point_pairs[0].2;

    for pair in point_pairs.iter().skip(1) {
        let angle = pair.2;
        if (angle - cur_angle).abs() <= tolerance {
            cur_cluster.push(*pair);
        } else {
            clusters.push(cur_cluster);
            cur_cluster = ArrayVec::new();
            cur_cluster.push(*pair);
            cur_angle = pair.2;
        }
    }

    clusters.push(cur_cluster);
    clusters.sort_unstable_by_key(|c| c.len());
    clusters.reverse();

    clusters
}

fn grid_to_blob_direction(g: &Point, b: &Point, tolerance: f32) -> CodePoint {
    let angle = g.angle_360(b);

    if -tolerance <= angle && angle <= tolerance {
        return CodePoint::Right;
    } else if 90.0 - tolerance <= angle && angle <= 90.0 + tolerance {
        return CodePoint::Down;
    } else if -90.0 - tolerance <= angle && angle <= -90.0 + tolerance {
        return CodePoint::Up;
    } else if (180.0 - tolerance <= angle && angle <= 180.0)
        || (-180.0 <= angle && angle <= -180.0 + tolerance)
    {
        return CodePoint::Left;
    } else {
        return CodePoint::Unknown;
    }
}

fn bruteforce_grid_shift(
    grid_points: &[ArrayVec<Point, 8>],
    blob_centers: &[Point],
) -> ArrayVec<ArrayVec<CodePoint, 8>, 8> {
    let mut best_shift_code_points = ArrayVec::new();
    let mut best_found = 0;

    for (shift_x, shift_y) in [
        (-DOT_CENTER_TO_GRID, 0.0),
        (0.0, -DOT_CENTER_TO_GRID),
        (DOT_CENTER_TO_GRID, 0.0),
        (0.0, DOT_CENTER_TO_GRID),
    ] {
        let mut code_points = ArrayVec::new();
        let mut num_found = 0;
        for row in grid_points {
            let mut crow = ArrayVec::new();
            for col in row {
                // find blob center that is within DOT_CENTER_TO_GRID (+margin), then derive that grid points direction
                let grid_point = Point {
                    x: col.x + shift_x,
                    y: col.y + shift_y,
                };
                let mut found = false;
                for blob in blob_centers {
                    if grid_point.distance(blob) < DOT_CENTER_TO_GRID * 1.5 {
                        crow.push(grid_to_blob_direction(&grid_point, blob, 35.0));
                        found = true;
                        num_found += 1;
                        break;
                    }
                }
                if !found {
                    crow.push(CodePoint::NotSeen);
                }
            }
            code_points.push(crow);
        }
        if num_found > best_found {
            best_found = num_found;
            best_shift_code_points = code_points;
        }
    }

    best_shift_code_points
}

pub fn solve(image: &Image) -> ArrayVec<ArrayVec<CodePoint, 8>, 8> {
    let thresholded = threshold(image, 200);
    let blobs = label_blobs(thresholded);

    let mut blob_centers: ArrayVec<Point, 64> = ArrayVec::new();
    for blob in blobs {
        if blob.max_length() > DOT_CENTER_TO_GRID * 2.0 {
            let (blob1, blob2) = blob.split();
            blob_centers.push(blob1.center());
            blob_centers.push(blob2.center());
        } else {
            blob_centers.push(blob.center());
        }
    }

    // debug!("found {:?} blobs", blob_centers.len());

    let mut grid_pairs: ArrayVec<(Point, Point, f32), 50> = ArrayVec::new();
    for (idx, point1) in blob_centers.iter().enumerate() {
        for point2 in blob_centers.iter().skip(idx + 1) {
            let distance = point1.distance(point2);
            for loc in 1..3 {
                if (distance - loc as f32 * GRID_SPACING) < 0.05 * GRID_SPACING {
                    grid_pairs.push((*point1, *point2, point1.angle_90(point2)));
                    break;
                }
            }
        }
    }

    // cluster based on angle, there should be two clusters roughly 90 deg apart
    let clusters = cluster_pairs(grid_pairs, 5.0);

    // dbg!(&clusters);

    // if more than 1 cluster, avg the cluster angles and make sure they are ~90 deg apart. if not, only use the first cluster and calc 90 deg off of it
    let mut angle1 = 0.0;
    for pair in &clusters[0] {
        angle1 += pair.2;
    }
    angle1 /= clusters[0].len() as f32;

    let angle2 = if clusters.len() > 1 {
        let mut angle2 = 0.0;
        for pair in &clusters[1] {
            angle2 += pair.2;
        }
        angle2 /= clusters[1].len() as f32;

        let mut diff: f32 = angle1 - angle2;
        if diff < 0.0 {
            diff += 180.0;
        }

        if (90.0 - diff).abs() > 5.0 {
            None
        } else {
            Some(angle2)
        }
    } else {
        None
    };

    let angle2 = angle2.unwrap_or_else(|| {
        let mut a = angle1 - 90.0;
        if a < 0.0 {
            a += 180.0;
        }
        a
    });

    // Rotate grid so it is horizontal/vertically aligned

    // Figure out which angle is closest to 0
    let mut rotation = if angle1.abs() < angle2.abs() {
        angle1
    } else {
        angle2
    };

    if rotation > 45.0 {
        rotation = 90.0 - rotation;
    }

    // rotate all points around 0,0
    blob_centers.iter_mut().for_each(|b| b.rotate(-rotation));

    // starting with the most central point, create the horizontal and vertical grid points, going 1 line past the bounds of the view window
    let mut center_point = Point {
        x: IMAGE_WIDTH as f32 / 2.0,
        y: IMAGE_HEIGHT as f32 / 2.0,
    };
    center_point.rotate(-rotation);

    let mut cur_point = blob_centers[0];
    let mut cur_dist = cur_point.distance(&center_point);
    for point in blob_centers.iter().skip(1) {
        let dist = point.distance(&center_point);
        if dist < cur_dist {
            cur_dist = dist;
            cur_point = *point;
        }
    }

    let min_x = blob_centers
        .iter()
        .map(|b| b.x)
        .fold(f32::MAX, |a, b| if a < b { a } else { b });
    let min_y = blob_centers
        .iter()
        .map(|b| b.y)
        .fold(f32::MAX, |a, b| if a < b { a } else { b });
    let max_x = blob_centers
        .iter()
        .map(|b| b.x)
        .fold(f32::MIN, |a, b| if a > b { a } else { b });
    let max_y = blob_centers
        .iter()
        .map(|b| b.y)
        .fold(f32::MIN, |a, b| if a > b { a } else { b });
    let mut cur_x = cur_point.x;
    let mut cur_y = cur_point.y;

    while cur_y > min_y - GRID_SPACING {
        cur_y -= GRID_SPACING;
    }

    while cur_x > min_x - GRID_SPACING {
        cur_x -= GRID_SPACING;
    }

    let mut grid_points: ArrayVec<ArrayVec<Point, 8>, 8> = ArrayVec::new();
    while cur_y <= max_y + GRID_SPACING * 2.0 {
        let mut x = cur_x;
        let mut row = ArrayVec::new();
        while x <= max_x + GRID_SPACING * 2.0 {
            row.push(Point { x, y: cur_y });
            x += GRID_SPACING;
        }
        cur_y += GRID_SPACING;
        grid_points.push(row);
    }

    bruteforce_grid_shift(&grid_points, &blob_centers)
}

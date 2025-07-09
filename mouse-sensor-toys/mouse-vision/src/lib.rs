#![cfg_attr(not(feature = "std"), no_std)]

// #![no_std]

use core::ops::Index;

use arrayvec::ArrayVec;
use image::imageops::FilterType::Lanczos3;
use libm::{atan2f, cosf, powf, sinf, sqrtf};

const IMAGE_WIDTH: usize = 36;
const IMAGE_HEIGHT: usize = 36;
const GRID_SPACING: f32 = 7.666666666666667;
const DOT_CENTER_TO_GRID: f32 = GRID_SPACING / 3.0;

const MAX_BLOBS: usize = 40;
const MAX_POINTS_IN_BLOB: usize = 50;
const MIN_BLOB_SIZE: usize = 10;
const MAX_BLOB_SIZE: usize = 50;

const OFFSET_GRID_SPACING: f32 = 8.5;
const SUBGRID_SPACING: f32 = 2.8333;

pub type Image = [[u8; IMAGE_WIDTH]; IMAGE_HEIGHT];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
pub struct Point {
    pub x: f32,
    pub y: f32,
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

    fn rotate_around(&mut self, other: &Self, degrees: f32) {
        self.x -= other.x;
        self.y -= other.y;
        self.rotate(degrees);
        self.x += other.x;
        self.y += other.y;
    }
}

#[derive(Debug)]
struct Blob<const M: usize> {
    // id: Option<u8>,
    points: ArrayVec<Point, M>,
}

impl<const M: usize> Blob<M> {
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
            // id: None,
            points: ArrayVec::new(),
        };
        let mut blob2 = Blob {
            // id: None,
            points: ArrayVec::new(),
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

// fn find_dark_regions(image: &Image, kernel_size: usize) -> Image {
//     let mut new_image = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];
//     for y in 0..IMAGE_HEIGHT {
//         for x in 0..IMAGE_WIDTH {

//         }
//     }

//     new_image
// }

fn flood_fill(
    image: &mut Image,
    x: usize,
    y: usize,
    blob_id: u8,
    blob_points: &mut ArrayVec<Point, MAX_BLOB_SIZE>,
) -> bool {
    image[y][x] = blob_id;

    let mut added_point_to_blob = blob_points
        .try_push(Point {
            x: x as f32,
            y: y as f32,
        })
        .is_ok();

    for ydiff in -2..=2 {
        for xdiff in -2..=2 {
            if ydiff == 0 && xdiff == 0 {
                continue;
            }
            if let (Some(new_x), Some(new_y)) =
                (x.checked_add_signed(xdiff), y.checked_add_signed(ydiff))
            {
                if new_y < IMAGE_HEIGHT && new_x < IMAGE_WIDTH && image[new_y][new_x] == 1 {
                    added_point_to_blob &= flood_fill(image, new_x, new_y, blob_id, blob_points);
                }
            }
        }
    }

    added_point_to_blob
}

/// This function will scan the image and extract all blobs (continuous dark regions)
/// Blobs will first be evaluated to only extract blobs of valid sizes (Accounting for 2 adjacent blobs, which look like one big one)
/// Return value is the centers of the found blobs
fn label_blobs(mut image: Image) -> ArrayVec<Point, MAX_BLOBS> {
    let mut blobs = ArrayVec::new();
    let mut cur_blob_id = 2;
    let mut img = image::RgbImage::new(36, 36);

    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            if image[y][x] == 1 {
                // need to label this pixel
                let mut blob_points: ArrayVec<Point, MAX_BLOB_SIZE> = ArrayVec::new();
                let possible_blob_points =
                    flood_fill(&mut image, x, y, cur_blob_id, &mut blob_points);
                if possible_blob_points {
                    for p in &blob_points {
                        img.put_pixel(p.x as u32, p.y as u32, image::Rgb([0, 0, 255]));
                    }
                    // && blob_points.len() >= MIN_BLOB_SIZE {
                    // Build blob from points
                    let blob = Blob {
                        points: blob_points,
                    };
                    if cur_blob_id == 10 {
                        println!("len: {}", blob.max_length());
                    }
                    if blob.max_length() > DOT_CENTER_TO_GRID * 2.0 {
                        let (blob1, blob2) = blob.split();
                        blobs.push(blob1.center());
                        blobs.push(blob2.center());
                    } else {
                        blobs.push(blob.center());
                    }
                }
                cur_blob_id += 1;
            }
        }
    }

    #[cfg(feature = "std")]
    print_image(&image, "labeled");
    img.save("labeled.png").expect("Failed to save image");

    blobs
}

// fn extract_blobs(labeled_image: &LabeledImage) -> ArrayVec<Blob<MAX_POINTS_IN_BLOB>, MAX_BLOBS> {
//     todo!()
// }

fn cluster_pairs(
    mut point_pairs: ArrayVec<(Point, Point, f32), 50>,
    tolerance: f32,
) -> ArrayVec<ArrayVec<(Point, Point, f32), 20>, 20> {
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
    grid_points: &[ArrayVec<Point, 10>],
    blob_centers: &[Point],
) -> ArrayVec<ArrayVec<CodePoint, 12>, 12> {
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

pub fn solve(image: &Image) -> Option<ArrayVec<ArrayVec<CodePoint, 12>, 12>> {
    print_image(image, "base");
    let mut img = image::RgbImage::new(36, 36);
    for (y, row) in image.iter().enumerate() {
        for (x, col) in row.iter().enumerate() {
            img.put_pixel(x as u32, y as u32, image::Rgb([*col, *col, *col]));
        }
    }

    let thresholded = threshold(image, 20);
    let mut t_img = img.clone();
    for (y, row) in thresholded.iter().enumerate() {
        for (x, col) in row.iter().enumerate() {
            if *col > 0 {
                t_img.put_pixel(x as u32, y as u32, image::Rgb([0, 255, 0]));
            }
        }
    }
    t_img.save("threshold.png").expect("Failed to save image");

    print_image(&thresholded, "thresholded");

    // using thresholded image (fairly brutal 15 or less), find adjacent pixels that are likely connected

    // let Some(blobs) = label_blobs(thresholded) else {
    //     return None;
    // };
    let mut blob_centers = label_blobs(thresholded);
    dbg!(&blob_centers, blob_centers.len());
    if blob_centers.len() < 15 {
        return None;
    }

    // let mut img = image::GrayImage::new(36, 36);
    for p in &blob_centers {
        img.put_pixel(p.x as u32, p.y as u32, image::Rgb([255, 0, 0]));
    }
    img.save("centers.png").expect("Failed to save image");

    // return None;

    // let mut blob_centers: ArrayVec<Point, 64> = ArrayVec::new();
    // for blob in blobs {
    //     if blob.max_length() > DOT_CENTER_TO_GRID * 2.0 {
    //         let (blob1, blob2) = blob.split();
    //         blob_centers.push(blob1.center());
    //         blob_centers.push(blob2.center());
    //     } else {
    //         blob_centers.push(blob.center());
    //     }
    // }

    // // debug!("found {:?} blobs", blob_centers.len());

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
    dbg!(&grid_pairs);

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

    dbg!(rotation);
    // rotation = 0.0;
    todo!("rotation wrong");

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

    let mut grid_points: ArrayVec<ArrayVec<Point, 10>, 10> = ArrayVec::new();
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

    Some(bruteforce_grid_shift(&grid_points, &blob_centers))
}

#[cfg(feature = "std")]
fn print_image(image: &Image, label: &str) {
    println!("{label}");
    for row in image {
        for col in row {
            print!("{col:>3} ");
        }
        println!();
    }
    println!();
}

#[cfg(feature = "debug")]
fn save_image(image: &Image, name: &str) {
    let mut img = image::RgbImage::new(36, 36);
    for (y, row) in image.iter().enumerate() {
        for (x, col) in row.iter().enumerate() {
            img.put_pixel(x as u32, y as u32, image::Rgb([*col, *col, *col]));
        }
    }

    img.save(name).expect("Failed to save image");
}

struct Drawable {
    img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    orig_image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    scale: u32,
}

impl Drawable {
    fn new(image: &Image) -> Self {
        let mut img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = image::RgbImage::new(36, 36);
        for (y, row) in image.iter().enumerate() {
            for (x, col) in row.iter().enumerate() {
                img.put_pixel(x as u32, y as u32, image::Rgb([*col, *col, *col]));
            }
        }

        Self {
            img: img.clone(),
            orig_image: img,
            scale: 1,
        }
    }

    fn resize(&mut self, new_scale: u32) {
        let new_img = image::imageops::resize(
            &self.orig_image,
            self.orig_image.width() * new_scale,
            self.orig_image.height() * new_scale,
            Lanczos3,
        );

        self.img = new_img;
        self.scale = new_scale
    }

    fn save(&self, name: &str) {
        self.img.save(name).unwrap();
    }

    fn draw_point(&mut self, point: &Point, color: [u8; 3]) {
        let x = (point.x * self.scale as f32) as u32;
        let y = (point.y * self.scale as f32) as u32;

        // if x >= 0 && x < self.img.width() && y >= 0 && y < self.img.height() {
        self.img.put_pixel(x, y, image::Rgb(color));
        // }
    }
}

#[cfg(feature = "debug")]
fn draw_points_on_image(points: &[Point], image: &Image, name: &str, point_color: [u8; 3]) {
    let mut img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = image::RgbImage::new(36, 36);
    for (y, row) in image.iter().enumerate() {
        for (x, col) in row.iter().enumerate() {
            img.put_pixel(x as u32, y as u32, image::Rgb([*col, *col, *col]));
        }
    }

    for point in points {
        let x = point.x.round() as u32;
        let y = point.y.round() as u32;
        img.put_pixel(x, y, image::Rgb(point_color));
    }

    img.save(name).expect("Failed to save image");
}

pub struct SmoothedImage(Image);

pub fn smooth_image(image: &Image, kernel_size: usize) -> SmoothedImage {
    let mut new_image = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];

    for (row_idx, row) in image.iter().enumerate() {
        for (col_idx, _col) in row.iter().enumerate() {
            let offset = (kernel_size - 1) / 2;
            let start_col_idx = col_idx.saturating_sub(offset);
            let end_col_idx = (col_idx + offset).min(IMAGE_WIDTH - 1);
            let start_row_idx = row_idx.saturating_sub(offset);
            let end_row_idx = (row_idx + offset).min(IMAGE_HEIGHT - 1);

            let mut avg = 0;
            let mut added = 0;
            for y in start_row_idx..=end_row_idx {
                for x in start_col_idx..=end_col_idx {
                    avg += image[y][x] as usize;
                    added += 1;
                }
            }

            avg /= added;

            new_image[row_idx][col_idx] = avg as u8;
        }
    }

    SmoothedImage(new_image)
}

fn max_derivative(image: &Image) -> Image {
    let mut new_image = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];

    for (row_idx, row) in image.iter().enumerate() {
        for (col_idx, col) in row.iter().enumerate() {
            let offset = 2;
            let start_col_idx = col_idx.saturating_sub(offset);
            let end_col_idx = (col_idx + offset).min(IMAGE_WIDTH - 1);
            let start_row_idx = row_idx.saturating_sub(offset);
            let end_row_idx = (row_idx + offset).min(IMAGE_HEIGHT - 1);

            let mut max = 0;
            for y in start_row_idx..=end_row_idx {
                for x in start_col_idx..=end_col_idx {
                    max = max.max(image[y][x].saturating_sub(*col));
                }
            }

            new_image[row_idx][col_idx] = max;
        }
    }

    new_image
}

fn find_local_minimums(image: &Image, kernel_size: usize) -> Image {
    assert!(kernel_size > 1 && kernel_size % 2 == 1);

    let mut new_image = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];

    for (row_idx, row) in image.iter().enumerate() {
        for (col_idx, col) in row.iter().enumerate() {
            let offset = (kernel_size - 1) / 2;
            let start_col_idx = col_idx.saturating_sub(offset);
            let end_col_idx = (col_idx + offset).min(IMAGE_WIDTH - 1);
            let start_row_idx = row_idx.saturating_sub(offset);
            let end_row_idx = (row_idx + offset).min(IMAGE_HEIGHT - 1);

            let mut is_smallest = true;
            for y in start_row_idx..=end_row_idx {
                for x in start_col_idx..=end_col_idx {
                    if *col > image[y][x] + 1 {
                        is_smallest = false;
                        break;
                    }
                }
            }

            if is_smallest {
                new_image[row_idx][col_idx] = 255;
            }
        }
    }

    new_image
}

// fn find_dark_to_light_transitions(image: &Image, delta: u8) -> Image {
//     let mut new_image = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];

//     for (row_idx, row) in image.iter().enumerate() {
//         for (col_idx, col) in row.iter().enumerate() {
//             let offset = (kernel_size - 1) / 2;
//             let start_col_idx = col_idx.saturating_sub(offset);
//             let end_col_idx = (col_idx + offset).min(IMAGE_WIDTH - 1);
//             let start_row_idx = row_idx.saturating_sub(offset);
//             let end_row_idx = (row_idx + offset).min(IMAGE_HEIGHT - 1);

//             let mut num_smaller = 0;
//             for y in start_row_idx..=end_row_idx {
//                 for x in start_col_idx..=end_col_idx {
//                     if *col < image[y][x] {
//                         num_smaller += 1;

//                         if num_smaller > 2 {
//                             break;
//                         }
//                     }
//                 }
//             }

//             if num_smaller > 2 {
//                 new_image[row_idx][col_idx] = 255;
//             }
//         }
//     }

//     new_image
// }

pub fn generate_brightness_map(empty_images: &[Image]) -> Image {
    let mut map = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];

    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let mut avg = 0;
            for empty in empty_images {
                avg += 255 - empty[y][x] as usize;
            }
            map[y][x] = (avg / empty_images.len()) as u8;
        }
    }

    map
}

fn apply_brightness_map(image: &Image, bmap: &Image) -> Image {
    let mut new_image = [[0; IMAGE_WIDTH]; IMAGE_HEIGHT];

    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            new_image[y][x] = image[y][x].saturating_add(bmap[y][x]);
        }
    }

    new_image
}

// struct ColumnView<'a>(&'a Image);

// impl Index for ColumnView {
//     type Output;

//     fn index(&self, index: Idx) -> &Self::Output {
//         todo!()
//     }
// }

// trait ScanLines {
//     type Ouput;

//     fn
// }

// fn dot_pattern_scan(image: &Image) -> Vec<Point> {
//     let mut horizontal_dots: Vec<Point> = image.iter().map(|row| );

// }

struct VerticalView<'a> {
    image: &'a Image,
    col_idx: usize,
}

struct HorizontalView<'a> {
    image: &'a Image,
    row_idx: usize,
}

trait ScanView {
    fn len(&self) -> usize;
}

impl<'a> Index<usize> for VerticalView<'a> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.image[index][self.col_idx]
    }
}

impl<'a> ScanView for VerticalView<'a> {
    fn len(&self) -> usize {
        self.image.len()
    }
}

impl<'a> Index<usize> for HorizontalView<'a> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.image[self.row_idx][index]
    }
}

impl<'a> ScanView for HorizontalView<'a> {
    fn len(&self) -> usize {
        self.image[0].len()
    }
}

fn linear_scan<S>(line: S) -> Vec<f32>
where
    S: ScanView + Index<usize, Output = u8>,
{
    let mut mid_points = Vec::new();

    let mut col_idx = 0;
    let mut possible_dot = false;
    let mut dot_start = 0;
    let mut end_dot_threshold = 0;
    let mut dot_bottom_threshold = 0;
    let mut found_bottom = false;
    while col_idx < line.len() - 2 {
        if !possible_dot
            && line[col_idx + 1] < line[col_idx]
            && line[col_idx + 2] < line[col_idx + 1]
        {
            possible_dot = true;
            found_bottom = false;
            dot_start = col_idx;
            end_dot_threshold = (line[col_idx] as f32 * 0.7) as u8;
            dot_bottom_threshold = (line[col_idx] as f32 * 0.4) as u8;
            col_idx += 1;
        }

        if possible_dot {
            if col_idx - dot_start > 6 {
                col_idx = dot_start;
                possible_dot = false;
            } else {
                if !found_bottom && line[col_idx] <= dot_bottom_threshold {
                    found_bottom = true;
                }
                if found_bottom && line[col_idx] >= end_dot_threshold {
                    possible_dot = false;

                    if col_idx - dot_start > 4 {
                        mid_points.push((col_idx + dot_start) as f32 / 2.0);
                    }
                }
            }
        }

        col_idx += 1;
    }

    mid_points
}

pub fn find_dots(image: &SmoothedImage) -> Vec<Point> {
    struct DotSample {
        x_accum: f32,
        y_accum: f32,
        num_samples: u8,
        found_by_cross_search: bool,
    }

    impl DotSample {
        fn estimate_x(&self) -> f32 {
            self.x_accum / self.num_samples as f32
        }

        fn estimate_y(&self) -> f32 {
            self.y_accum / self.num_samples as f32
        }
    }

    #[cfg(feature = "debug")]
    save_image(&image.0, "smoothed_3.png");
    print_image(&image.0, "smoothed");

    let mut dot_samples: Vec<DotSample> = Vec::new();

    for col_idx in 0..image.0[0].len() {
        let column = VerticalView {
            image: &image.0,
            col_idx,
        };
        for mid_point_y in linear_scan(column) {
            let mut found_similar = false;
            for existing_sample in dot_samples.iter_mut() {
                let diff_x = (existing_sample.estimate_x() - col_idx as f32).abs();
                let diff_y = (existing_sample.estimate_y() - mid_point_y).abs();

                if diff_x < 1.5 && diff_y <= 1.5 {
                    found_similar = true;
                    existing_sample.x_accum += col_idx as f32;
                    existing_sample.y_accum += mid_point_y;
                    existing_sample.num_samples += 1;
                    break;
                }
            }

            if !found_similar {
                dot_samples.push(DotSample {
                    x_accum: col_idx as f32,
                    y_accum: mid_point_y,
                    num_samples: 1,
                    found_by_cross_search: false,
                });
            }
        }
    }

    for row_idx in 0..image.0.len() {
        let row = HorizontalView {
            image: &image.0,
            row_idx,
        };

        for mid_point_x in linear_scan(row) {
            // must find similar, no additions here. otherwise we might store adjacent blobs instead of standalone ones
            for existing_sample in dot_samples.iter_mut() {
                let diff_x = (existing_sample.estimate_x() - mid_point_x).abs();
                let diff_y = (existing_sample.estimate_y() - row_idx as f32).abs();

                if diff_x < 1.5 && diff_y <= 1.5 {
                    existing_sample.x_accum += mid_point_x;
                    existing_sample.y_accum += row_idx as f32;
                    existing_sample.num_samples += 1;
                    existing_sample.found_by_cross_search = true;
                    break;
                }
            }
        }
    }

    let dots: Vec<Point> = dot_samples
        .iter()
        .filter(|sample| sample.found_by_cross_search)
        .map(|sample| Point {
            x: sample.estimate_x() + 0.75, // correction since it will be naturally shifted left and up
            y: sample.estimate_y() + 0.75,
        })
        .collect();

    #[cfg(feature = "debug")]
    {
        let mut draw = Drawable::new(&image.0);
        draw.resize(5);

        for dot in dots.iter() {
            draw.draw_point(dot, [255, 0, 0]);
        }
        draw.save("linear_scan.png");
    }

    dots
}

// struct SubgridIter {
//     cur_x: f32,
//     cur_y: f32
// }

// impl Iterator for SubgridIter {
//     type Item = Point;

//     fn next(&mut self) -> Option<Self::Item> {
//         todo!()
//     }
// }

#[derive(Debug, Clone, Copy)]
pub struct GridModel {
    anchor: Point,
    rotation: f32,
}

impl GridModel {
    pub fn get_angle(&self) -> f32 {
        self.rotation
    }

    fn get_subgrid_points(&self) -> Vec<Point> {
        let mut points = Vec::new();

        let mut start_x = self.anchor.x;
        let mut start_y = self.anchor.y;

        while start_x > -36.0 {
            start_x -= SUBGRID_SPACING;
        }
        while start_y > -36.0 {
            start_y -= SUBGRID_SPACING;
        }

        while start_y < 36.0 * 2.0 {
            let mut cur_x = start_x;
            while cur_x < 36.0 * 2.0 {
                let mut possible_point = Point {
                    x: cur_x,
                    y: start_y,
                };
                possible_point.rotate_around(&self.anchor, self.rotation);
                if possible_point.x > 0.0
                    && possible_point.x < 36.0
                    && possible_point.y > 0.0
                    && possible_point.y < 36.0
                {
                    points.push(possible_point);
                }
                cur_x += SUBGRID_SPACING;
            }
            start_y += SUBGRID_SPACING;
        }

        points
    }

    #[cfg(feature = "debug")]
    fn draw_grid(&self, draw: &mut Drawable) {
        let subgrid_points = self.get_subgrid_points();
        dbg!(subgrid_points.len());
        for point in subgrid_points.iter() {
            draw.draw_point(point, [0, 0, 255]);
        }

        // draw anchor
        draw.draw_point(&self.anchor, [255, 0, 0]);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Debug, Clone, Copy)]
struct GridPoint<'a> {
    grid_model: &'a GridModel,
    point: Point,
}

impl<'a> GridPoint<'a> {
    fn move_subgrid(&mut self, dir: Direction) {
        self.move_by(SUBGRID_SPACING, dir);
    }
    fn move_grid(&mut self, dir: Direction) {
        self.move_by(OFFSET_GRID_SPACING, dir);
    }

    fn move_by(&mut self, distance: f32, dir: Direction) {
        // un-rotate point
        self.point
            .rotate_around(&self.grid_model.anchor, -self.grid_model.rotation);

        // shift it
        match dir {
            Direction::Up => self.point.y -= distance,
            Direction::Right => self.point.x += distance,
            Direction::Down => self.point.y += distance,
            Direction::Left => self.point.x -= distance,
        }

        // re-rotate it to grid
        self.point
            .rotate_around(&self.grid_model.anchor, self.grid_model.rotation);
    }

    /// Gives the coords of this GridPoint rotation corrected, as if the image had zero rotation
    fn get_flat_coords(&self) -> Point {
        let mut point = self.point;
        point.rotate_around(&self.grid_model.anchor, -self.grid_model.rotation);

        point
    }

    fn sample_image(&self, image: &SmoothedImage) -> Option<u8> {
        let sample_point = self.get_flat_coords();
        let mut x = sample_point.x.round();
        x = x.max(0.0);
        x = x.min(image.0.len() as f32);

        let mut y = sample_point.y.round();
        y = y.max(0.0);
        y = y.min(image.0.len() as f32);

        if x < 0.0 || x >= 36.0 || y < 0.0 || y >= 36.0 {
            return None;
        }

        let x = x as usize;
        let y = y as usize;

        Some(image.0[y][x])
    }
}

pub fn get_code_points(
    grid_model: &GridModel,
    image: &SmoothedImage,
    shift: Direction,
) -> Vec<Vec<CodePoint>> {
    // Get initial point
    let mut cur_point = GridPoint {
        grid_model,
        point: grid_model.anchor,
    };

    // Shift it since it'll be in the middle of a dot
    cur_point.move_subgrid(shift);

    // // Make sure point is within bounds of image. If not move it inwards
    // let p = cur_point.get_flat_coords();
    // if p.x < 0.0 || p.x > 36.0 || p.y < 0.0 || p.y > 36.0 {
    //     match shift {
    //         Direction::Up => cur_point.move_grid(Direction::Down),
    //         Direction::Right => cur_point.move_grid(Direction::Left),
    //         Direction::Down => cur_point.move_grid(Direction::Up),
    //         Direction::Left => cur_point.move_grid(Direction::Right),
    //     }
    // }

    // move this point towards the middle of the image, also to guarantee all adjacent subpoints are within image bounds
    let p = cur_point.get_flat_coords();
    let x_shift = if (p.x as usize) < image.0.len() / 2 {
        Direction::Right
    } else {
        Direction::Left
    };

    let y_shift = if (p.y as usize) < image.0.len() / 2 {
        Direction::Down
    } else {
        Direction::Up
    };

    for _ in 0..2 {
        cur_point.move_grid(x_shift);
        cur_point.move_grid(y_shift);
    }

    // Find top left
    let mut prev = cur_point;

    // leftmost point
    loop {
        let mut left_subgrid = cur_point;
        left_subgrid.move_subgrid(Direction::Left);
        if left_subgrid.get_flat_coords().x < -2.0 {
            cur_point = prev;
            break;
        } else {
            prev = cur_point;
            cur_point.move_grid(Direction::Left);
        }
    }

    // topmost point
    loop {
        let mut up_subgrid = cur_point;
        up_subgrid.move_subgrid(Direction::Up);
        if up_subgrid.get_flat_coords().y < -2.0 {
            cur_point = prev;
            break;
        } else {
            prev = cur_point;
            cur_point.move_grid(Direction::Up);
        }
    }

    println!("topleft: {:?}", cur_point.get_flat_coords());

    // #[cfg(feature = "debug")]
    // {
    //     let mut draw = Drawable::new(&image.0);

    //     // draw subgrid
    //     let mut subgrid_point = cur_point;
    //     subgrid_point.move_subgrid(Direction::Left);
    //     subgrid_point.move_subgrid(Direction::Up);

    //     draw.draw_point(&cur_point.get_flat_coords(), [255, 0, 0]);

    //     let
    // }

    // Scan through grid, sampling neighbors to determine
    #[cfg(feature = "debug")]
    let mut grid_points = Vec::new();

    #[cfg(feature = "debug")]
    let mut subgrid_points = Vec::new();

    let mut codepoints = Vec::new();
    loop {
        let mut x_cur_point = cur_point;

        let mut row = Vec::new();

        'x_scan: loop {
            #[cfg(feature = "debug")]
            grid_points.push(x_cur_point.point);

            let mut samples = [0; 4];
            for (i, dir) in [
                Direction::Up,
                Direction::Right,
                Direction::Down,
                Direction::Left,
            ]
            .iter()
            .enumerate()
            {
                let mut sample_point = x_cur_point;
                sample_point.move_subgrid(*dir);

                let sample_val = sample_point.sample_image(image);
                if let Some(sample_val) = sample_val {
                    samples[i] = sample_val;

                    #[cfg(feature = "debug")]
                    subgrid_points.push(sample_point.point);
                } else {
                    break 'x_scan;
                }
            }

            // make sure only one sample is darker (by appropriate margin) than all others, as well as darker than center
            let mut min_sample = u8::MAX;
            let mut min_sample_idx = 0;
            for (sample_idx, sample) in samples.iter().enumerate() {
                if *sample < min_sample {
                    min_sample = *sample;
                    min_sample_idx = sample_idx;
                }
            }

            let threshold = (min_sample as f32 * 1.3) as u8;
            let mut num_roughly_as_dark = 0;
            for sample in samples {
                if sample <= threshold {
                    num_roughly_as_dark += 1;
                }
            }

            let codepoint = if num_roughly_as_dark == 1
                && min_sample < x_cur_point.sample_image(image).unwrap()
            {
                match min_sample_idx {
                    0 => CodePoint::Up,
                    1 => CodePoint::Right,
                    2 => CodePoint::Down,
                    3 => CodePoint::Left,
                    _ => unreachable!(),
                }
            } else {
                dbg!(
                    x_cur_point,
                    threshold,
                    x_cur_point.sample_image(image).unwrap(),
                    samples
                );
                CodePoint::Unknown
            };

            row.push(codepoint);

            x_cur_point.move_grid(Direction::Right);
            let mut subpoint = x_cur_point;
            subpoint.move_subgrid(Direction::Right);
            if subpoint.sample_image(image).is_none() {
                break;
            }
        }

        codepoints.push(row);

        cur_point.move_grid(Direction::Down);

        // make sure points below will be valid
        let mut subpoint = cur_point;
        subpoint.move_subgrid(Direction::Down);
        if subpoint.sample_image(image).is_none() {
            break;
        }
    }

    #[cfg(feature = "debug")]
    {
        let mut draw = Drawable::new(&image.0);
        draw.resize(5);

        for p in subgrid_points {
            draw.draw_point(&p, [0, 0, 255]);
        }

        for p in grid_points {
            draw.draw_point(&p, [255, 0, 0]);
        }

        draw.save("sampled_grid.png");
    }

    for row in &codepoints {
        for col in row {
            print!("{} ", Into::<char>::into(*col));
        }
        println!();
    }

    codepoints
}

// struct GridView<'a> {
//     model: &'a GridModel,
//     image: &'a SmoothedImage,
//     shift: Direction
//     // min_x: f32,
//     // min_y: f32,
//     // max_x: f32,
//     // max_y: f32,
// }

// impl<'a> GridView<'a> {
//     // fn new(model: &'a GridModel, image: &'a SmoothedImage) -> Self {
//     //     let mut start_x = model.anchor.x;
//     //     let mut start_y = model.anchor.y;

//     //     let mut min_x = f32::MAX;
//     //     let mut min_y = f32::MAX;
//     //     let mut max_x: f32 = 0.0;
//     //     let mut max_y: f32 = 0.0;

//     //     while start_x > -36.0 {
//     //         start_x -= SUBGRID_SPACING;
//     //     }
//     //     while start_y > -36.0 {
//     //         start_y -= SUBGRID_SPACING;
//     //     }

//     //     while start_y < 36.0 * 2.0 {
//     //         let mut cur_x = start_x;
//     //         while cur_x < 36.0 * 2.0 {
//     //             let mut possible_point = Point {
//     //                 x: cur_x,
//     //                 y: start_y,
//     //             };
//     //             possible_point.rotate_around(&model.anchor, model.rotation);

//     //             if possible_point.x > 0.0 && possible_point.x < 36.0 {
//     //                 min_x = min_x.min(possible_point.x);
//     //                 max_x = max_x.max(possible_point.x);
//     //             }

//     //             if possible_point.y > 0.0 && possible_point.y < 36.0 {
//     //                 min_y = min_y.min(possible_point.y);
//     //                 max_y = max_y.max(possible_point.y);
//     //             }
//     //             cur_x += SUBGRID_SPACING;
//     //         }
//     //         start_y += SUBGRID_SPACING;
//     //     }

//     //     Self {
//     //         model,
//     //         image,
//     //         min_x,
//     //         min_y,
//     //         max_x,
//     //         max_y,
//     //     }
//     // }

//     // fn width(&self) -> usize {
//     //     ((self.max_x - self.min_x) / SUBGRID_SPACING).ceil() as usize
//     // }

//     // fn height(&self) -> usize {
//     //     ((self.max_y - self.min_y) / SUBGRID_SPACING).ceil() as usize
//     // }

//     // fn get(&self, x: usize, y: usize) -> Option<u8> {
//     //     if x >= self.width() || y >= self.height() {
//     //         return None;
//     //     }
//     //     todo!()
//     // }
// }

pub fn reconstruct_grid(dots: &[Point], image: &SmoothedImage) -> Option<GridModel> {
    // Find furthest apart points
    let mut max_dist = 0.0;
    let mut anchor = dots[0];
    let mut anchor_idx = 0;
    let mut compare_point = dots[0];
    for p1_idx in 0..dots.len() {
        for p2_idx in p1_idx + 1..dots.len() {
            let p1 = dots[p1_idx];
            let p2 = dots[p2_idx];
            let dist = p1.distance(&p2);
            if dist > max_dist {
                max_dist = dist;
                anchor = p1;
                anchor_idx = p1_idx;
                compare_point = p2;
            }
        }
    }

    // Determine rotation
    let mut best_collective_error = f32::MAX;
    let mut best_angle = 0;
    for rot in 0..90 {
        let mut collective_error = 0.0;
        for (dot_idx, dot) in dots.iter().enumerate() {
            if dot_idx == anchor_idx {
                continue;
            }

            let mut rotated_compare = dot.clone();
            rotated_compare.rotate_around(&anchor, -rot as f32);
            let x_diff = rotated_compare.x - anchor.x;
            let y_diff = rotated_compare.y - anchor.y;

            let closest_subgrid_point = Point {
                x: anchor.x + (x_diff / SUBGRID_SPACING).round() * SUBGRID_SPACING,
                y: anchor.y + (y_diff / SUBGRID_SPACING).round() * SUBGRID_SPACING,
            };

            let dist = rotated_compare.distance(&closest_subgrid_point);
            collective_error += dist;
        }

        if collective_error < best_collective_error {
            best_collective_error = collective_error;
            best_angle = rot;
        }
    }

    let model = GridModel {
        anchor,
        rotation: best_angle as f32,
    };

    #[cfg(feature = "debug")]
    {
        let mut draw = Drawable::new(&image.0);
        draw.resize(5);
        model.draw_grid(&mut draw);
        draw.draw_point(&compare_point, [255, 128, 128]);
        draw.save(&format!("tmp/grid_{}.png", model.rotation));
    }

    dbg!(best_collective_error, best_angle);

    Some(model)
}

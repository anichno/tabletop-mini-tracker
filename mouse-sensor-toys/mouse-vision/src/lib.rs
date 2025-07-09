#![cfg_attr(not(feature = "std"), no_std)]

// #![no_std]

use core::ops::Index;

use image::imageops::FilterType::Lanczos3;
use libm::{atan2f, cosf, powf, sinf, sqrtf};

const IMAGE_WIDTH: usize = 36;
const IMAGE_HEIGHT: usize = 36;

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

        self.img.put_pixel(x, y, image::Rgb(color));
    }
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

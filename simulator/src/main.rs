use mini_tracker::{self, Point, Receiver, Table};

// mod receiver_placements;

// 1 px == 1 mm
// 25mm == ~1 in
const MM_PER_INCH: f32 = 25.4;

// grid is
// |
// |
// |
// .--------

// 2 in min distance from mini to edge
const STANDOFF_DISTANCE: f32 = 2.0 * MM_PER_INCH;
const TABLE_WIDTH: f32 = 930.0 + STANDOFF_DISTANCE;
const TABLE_HEIGHT: f32 = 523.0 + STANDOFF_DISTANCE;

const GRID_SIZE: f32 = MM_PER_INCH;

fn get_mini_edge_points(mini_center: Point) -> [Point; 360] {
    let mut points = [Point { x: 0.0, y: 0.0 }; 360];
    let distance = MM_PER_INCH / 2.0;
    for (i, point) in points.iter_mut().enumerate() {
        let angle = i as f32;
        point.x = mini_center.x + distance * angle.to_radians().cos();
        point.y = mini_center.y + distance * angle.to_radians().sin();
    }
    points
}

fn place_vertical_receivers(view_angle: f32, receivers_per_mm: f32) -> Vec<Receiver> {
    let mut receivers = Vec::new();
    let mut y = MM_PER_INCH / 2.0;
    while y < TABLE_HEIGHT {
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            view_angle,
            Point { x: 0.0, y },
            mini_tracker::Direction::Right,
        ));
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            view_angle,
            Point { x: TABLE_WIDTH, y },
            mini_tracker::Direction::Left,
        ));

        y += MM_PER_INCH / receivers_per_mm;
    }

    receivers
}

fn place_horizontal_receivers(view_angle: f32, receivers_per_mm: f32) -> Vec<Receiver> {
    let mut receivers = Vec::new();
    let mut x = MM_PER_INCH / 2.0;
    while x < TABLE_WIDTH {
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            view_angle,
            Point { x, y: 0.0 },
            mini_tracker::Direction::Up,
        ));
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            view_angle,
            Point { x, y: TABLE_HEIGHT },
            mini_tracker::Direction::Down,
        ));

        x += MM_PER_INCH / receivers_per_mm;
    }

    receivers
}

#[derive(Debug)]
struct TestResult {
    total_receivers: usize,
    all_correct: bool,
    avg_area: f32,
    avg_error: f32,
    max_area: f32,
    max_error: f32,
}

fn run_test(
    vert_density: f32,
    horiz_density: f32,
    vert_view_angle: f32,
    horiz_view_angle: f32,
) -> TestResult {
    let mut receivers = place_horizontal_receivers(horiz_view_angle, horiz_density);
    receivers.append(&mut place_vertical_receivers(vert_view_angle, vert_density));

    let total_receivers = receivers.len();

    let table = Table::new(TABLE_WIDTH, TABLE_HEIGHT, receivers);

    // For each valid table location, check if table can determine mini location within error bounds
    let mut num_correct = 0;
    let mut tot_locations = 0;
    let mut avg_area = 0.0;
    let mut avg_error = 0.0;
    let mut max_error = 0.0;
    let mut max_area = 0.0;

    let mut x = STANDOFF_DISTANCE + MM_PER_INCH / 2.0;
    while x < TABLE_WIDTH - STANDOFF_DISTANCE - MM_PER_INCH / 2.0 {
        let mut y = STANDOFF_DISTANCE + MM_PER_INCH / 2.0;
        while y < TABLE_HEIGHT - STANDOFF_DISTANCE - MM_PER_INCH / 2.0 {
            tot_locations += 1;
            let mini_location = Point { x, y };
            let mut num_visible_receivers = 0;
            let mini_edge_points = get_mini_edge_points(mini_location);
            let mut visible_receivers = Vec::new();
            for receiver in table.receivers.iter() {
                let mut can_see = false;
                for point in &mini_edge_points {
                    if receiver.can_see(point) {
                        can_see = true;
                        break;
                    }
                }
                if can_see {
                    num_visible_receivers += 1;
                }
                visible_receivers.push((*receiver, can_see));
            }

            assert!(num_visible_receivers > 0);

            // println!("{:?}", mini_location);
            // dbg!(
            //     vert_density,
            //     horiz_density,
            //     vert_view_angle,
            //     horiz_view_angle
            // );
            let Some(bounding_polygon) = table.get_bounding_polygon(&visible_receivers[..]) else {
                return TestResult {
                    total_receivers,
                    all_correct: false,
                    avg_area: avg_area / tot_locations as f32,
                    avg_error: avg_error / tot_locations as f32,
                    max_area,
                    max_error,
                };
            };
            assert!(!bounding_polygon.points.is_empty());
            // let Some(shrink_polygon) = bounding_polygon.shrink(25.4 / 2.0) else {
            //     return TestResult {
            //         total_receivers,
            //         all_correct: false,
            //         avg_area: avg_area / tot_locations as f32,
            //         avg_error: avg_error / tot_locations as f32,
            //         max_area,
            //         max_error,
            //     };
            // };
            let guessed_location = bounding_polygon.center();
            let error = 0.0; //shrink_polygon.max_width();
            let area = bounding_polygon.area();
            avg_area += area;
            avg_error += error;

            if guessed_location.distance(&mini_location) < GRID_SIZE {
                num_correct += 1;
            } else {
                // dbg!(mini_location, guessed_location);
                return TestResult {
                    total_receivers,
                    all_correct: false,
                    avg_area: avg_area / tot_locations as f32,
                    avg_error: avg_error / tot_locations as f32,
                    max_area,
                    max_error,
                };
            }

            if error > max_error {
                max_error = error;
                // dbg!(mini_location);
            }

            if area > max_area {
                max_area = area;
                // dbg!(mini_location);
            }
            y += GRID_SIZE;
        }

        x += GRID_SIZE;
    }

    TestResult {
        total_receivers,
        all_correct: num_correct == tot_locations,
        avg_area: avg_area / tot_locations as f32,
        avg_error: avg_error / tot_locations as f32,
        max_area,
        max_error,
    }
}

fn main() {
    const VERT_DENSITY_MIN: f32 = 0.5;
    const VERT_DENSITY_MAX: f32 = 3.0;
    const HORIZ_DENSITY_MIN: f32 = 0.5;
    const HORIZ_DENSITY_MAX: f32 = 3.0;
    const VERT_VIEW_ANGLE_MIN: f32 = 10.0;
    const VERT_VIEW_ANGLE_MAX: f32 = 90.0;
    const HORIZ_VIEW_ANGLE_MIN: f32 = 10.0;
    const HORIZ_VIEW_ANGLE_MAX: f32 = 90.0;

    const VERT_DENSITY_STEP: f32 = 0.5;
    const HORIZ_DENSITY_STEP: f32 = 0.5;
    const VERT_VIEW_ANGLE_STEP: f32 = 10.0;
    const HORIZ_VIEW_ANGLE_STEP: f32 = 10.0;

    println!("total_receivers,avg_area,avg_error,max_area,max_error,vert_density,horiz_density,vert_view_angle,horiz_view_angle");

    let mut vert_density = VERT_DENSITY_MIN;
    while vert_density <= VERT_DENSITY_MAX {
        let mut horiz_density = HORIZ_DENSITY_MIN;
        while horiz_density <= HORIZ_DENSITY_MAX {
            let mut vert_view_angle = VERT_VIEW_ANGLE_MIN;
            while vert_view_angle <= VERT_VIEW_ANGLE_MAX {
                let mut horiz_view_angle = HORIZ_VIEW_ANGLE_MIN;
                while horiz_view_angle <= HORIZ_VIEW_ANGLE_MAX {
                    let result = run_test(
                        vert_density,
                        horiz_density,
                        vert_view_angle,
                        horiz_view_angle,
                    );

                    if result.all_correct {
                        println!(
                            "{},{},{},{},{},{},{},{},{}",
                            result.total_receivers,
                            result.avg_area,
                            result.avg_error,
                            result.max_area,
                            result.max_error,
                            vert_density,
                            horiz_density,
                            vert_view_angle,
                            horiz_view_angle
                        );
                    }

                    horiz_view_angle += HORIZ_VIEW_ANGLE_STEP;
                }

                vert_view_angle += VERT_VIEW_ANGLE_STEP;
            }

            horiz_density += HORIZ_DENSITY_STEP;
        }

        vert_density += VERT_DENSITY_STEP;
    }
}

use mini_tracker::{self, Point, Receiver, Table};

// 1 px == 1 mm
// 25mm == ~1 in
const MM_PER_INCH: i32 = 25;

// grid is
// |
// |
// |
// .--------

const TABLE_WIDTH: i32 = 930;
const TABLE_HEIGHT: i32 = 523;

const GRID_SIZE: i32 = MM_PER_INCH;

fn get_mini_edge_points(mini_center: Point) -> [Point; 360] {
    let mut points = [Point { x: 0.0, y: 0.0 }; 360];
    let distance = MM_PER_INCH as f64 / 2.0;
    for i in 0..360 {
        let angle = i as f64;
        let x = mini_center.x + distance * angle.to_radians().cos();
        let y = mini_center.y + distance * angle.to_radians().sin();
        points[i] = Point { x, y };
    }
    points
}

fn main() {
    // Setup table

    // assume all equal view angles and spacing, 1 sensor per inch
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
            20.0,
            Point {
                x: 0.0,
                y: y as f64,
            },
            mini_tracker::Direction::Right,
        ));
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            20.0,
            Point {
                x: TABLE_WIDTH as f64,
                y: y as f64,
            },
            mini_tracker::Direction::Left,
        ));
    }

    let table = Table::new(TABLE_WIDTH, TABLE_HEIGHT, receivers);

    // For each valid table location, check if table can determine mini location within error bounds
    let mut num_correct = 0;
    let mut tot_locations = 0;
    let mut max_error = 0.0;
    let mut max_actual_error = 0.0;

    for x in (GRID_SIZE / 2..TABLE_WIDTH - (GRID_SIZE / 2)).step_by(GRID_SIZE as usize) {
        for y in (GRID_SIZE / 2..TABLE_HEIGHT - (GRID_SIZE / 2)).step_by(GRID_SIZE as usize) {
            tot_locations += 1;
            let mini_location = Point {
                x: x as f64,
                y: y as f64,
            };
            let mut num_visible_receivers = 0;
            println!("mini_location: {:?}", mini_location);
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
                // let can_see = receiver.can_see(&mini_location);
                if can_see {
                    num_visible_receivers += 1;
                }
                visible_receivers.push((receiver, can_see));
            }

            // dbg!(num_visible_receivers);
            assert!(num_visible_receivers > 0);
            // if num_visible_receivers == 0 {
            //     continue;
            // }

            let (guessed_location, error) = table.get_location(&visible_receivers);
            if guessed_location.distance(&mini_location) < GRID_SIZE as f64 {
                num_correct += 1;
            } else {
                dbg!(mini_location, guessed_location);
            }

            let actual_error = guessed_location.distance(&mini_location);
            if actual_error > max_actual_error {
                max_actual_error = actual_error;
            }

            if error > max_error {
                max_error = error;
            }
        }
    }

    println!("total: {tot_locations}, correct: {num_correct}, max calculated error: {max_error}, max actual error: {max_actual_error}");
}

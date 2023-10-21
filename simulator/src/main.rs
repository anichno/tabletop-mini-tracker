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

const TABLE_WIDTH: f32 = 930.0;
const TABLE_HEIGHT: f32 = 523.0;

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

fn main() {
    // Setup table

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
            mini_tracker::Direction::Up,
        ));
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            30.0,
            Point { x, y: TABLE_HEIGHT },
            mini_tracker::Direction::Down,
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
            mini_tracker::Direction::Right,
        ));
        receivers.push(Receiver::new(
            TABLE_WIDTH,
            TABLE_HEIGHT,
            20.0,
            Point { x: TABLE_WIDTH, y },
            mini_tracker::Direction::Left,
        ));

        y += MM_PER_INCH;
    }

    let table = Table::new(TABLE_WIDTH, TABLE_HEIGHT, receivers);

    // For each valid table location, check if table can determine mini location within error bounds
    let mut num_correct = 0;
    let mut tot_locations = 0;
    let mut max_error = 0.0;
    let mut max_actual_error = 0.0;

    let mut x = GRID_SIZE / 2.0;

    while x < TABLE_WIDTH - GRID_SIZE / 2.0 {
        let mut y = GRID_SIZE / 2.0;
        while y < TABLE_HEIGHT - GRID_SIZE / 2.0 {
            tot_locations += 1;
            let mini_location = Point { x, y };
            let mut num_visible_receivers = 0;
            // println!("mini_location: {:?}", mini_location);
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
            if guessed_location.distance(&mini_location) < GRID_SIZE {
                num_correct += 1;
            } else {
                // dbg!(mini_location, guessed_location);
            }

            let actual_error = guessed_location.distance(&mini_location);
            if actual_error > max_actual_error {
                max_actual_error = actual_error;
            }

            if error > max_error {
                max_error = error;
            }
            y += GRID_SIZE;
        }

        x += GRID_SIZE;
    }

    println!("total: {tot_locations}, correct: {num_correct}, max calculated error: {max_error}, max actual error: {max_actual_error}");
}

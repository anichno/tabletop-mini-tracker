use mini_tracker::{self, Point, Receiver, Table};

mod vis_bounding_box;
mod vis_receivers;

// 1 px == 1 mm
const PX_PER_MM: f32 = 3.0;
// 25mm == ~1 in
const MM_PER_INCH: f32 = 25.4;

// grid is
// |
// |
// |
// .--------

const TABLE_WIDTH: f32 = 930.0;
const TABLE_HEIGHT: f32 = 523.0;

const RECEIVER_SIZE: f32 = 2.5 * PX_PER_MM;

fn convert_y(y: f32) -> f32 {
    (TABLE_HEIGHT - y) * PX_PER_MM
}

fn convert_x(x: f32) -> f32 {
    x * PX_PER_MM
}

fn place_receivers() -> Vec<Receiver> {
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

        x += MM_PER_INCH / 1.0;
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

        y += MM_PER_INCH / 1.0;
    }

    dbg!(receivers.len());

    receivers
}

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
    let receivers = place_receivers();

    let table = Table::new(TABLE_WIDTH, TABLE_HEIGHT, receivers);

    let mini_location = Point { x: 37.0, y: 137.0 };

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
        visible_receivers.push((*receiver, can_see));
    }

    // for normal library based calculation
    let receiver_copies = table.receivers.clone();
    let mut copied_visible_receivers = Vec::new();
    for receiver in receiver_copies.iter() {
        let mut can_see = false;
        for point in &mini_edge_points {
            if receiver.can_see(point) {
                can_see = true;
                break;
            }
        }
        copied_visible_receivers.push((receiver, can_see));
    }

    let (estimated_location, location_error) = table.get_location(&copied_visible_receivers);
    let actual_error = mini_location.distance(&estimated_location);
    println!("Actual location: {:?}", mini_location);
    println!(
        "Estimated location: {:?} with estimated error {}, actual error {}",
        estimated_location, location_error, actual_error
    );

    // vis_receivers::run(table.clone(), mini_location, visible_receivers.clone());
    vis_bounding_box::run(table, mini_location, visible_receivers.clone());
}

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Debug, Clone, Copy)]
struct LogLine {
    timestamp: f32,
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
    gyro_x: f32,
    gyro_y: f32,
    gyro_z: f32,
}

fn parse_line(line: &str) -> LogLine {
    let mut parts = line.split(',');
    LogLine {
        timestamp: parts.next().unwrap().parse().unwrap(),
        accel_x: parts.next().unwrap().parse().unwrap(),
        accel_y: parts.next().unwrap().parse().unwrap(),
        accel_z: parts.next().unwrap().parse().unwrap(),
        gyro_x: parts.next().unwrap().parse().unwrap(),
        gyro_y: parts.next().unwrap().parse().unwrap(),
        gyro_z: parts.next().unwrap().parse().unwrap(),
    }
}

fn main() {
    env_logger::init();

    const SAMPLE_RATE_HZ: u32 = 833;

    // let no_move_file = File::open("test_data/no_movement.log").unwrap();
    let no_move_file = File::open("test_data/small_movement.log").unwrap();

    let no_move_file = BufReader::new(no_move_file);

    // Calibration data
    // let gyr_misalignment = imu_fusion::FusionMatrix::new(
    //     1.0f32, 0.0f32, 0.0f32, 0.0f32, 1.0f32, 0.0f32, 0.0f32, 0.0f32, 1.0f32,
    // );
    // let gyr_sensitivity = imu_fusion::FusionVector::new(1.0f32, 1.0f32, 1.0f32);
    let gyr_offset = imu_fusion::FusionVector::new(0.3737612, -0.0149396, -0.19158441);
    // let gyr_offset = imu_fusion::FusionVector::new(0.0, 0.0, 0.0);
    // let acc_misalignment = imu_fusion::FusionMatrix::new(
    //     1.0f32, 0.0f32, 0.0f32, 0.0f32, 1.0f32, 0.0f32, 0.0f32, 0.0f32, 1.0f32,
    // );
    // let acc_sensitivity = imu_fusion::FusionVector::new(1.0f32, 1.0f32, 1.0f32);
    // let acc_offset =
    //     imu_fusion::FusionVector::new(-1.1776061e-6f32, -0.0001076332f32, -0.030718777f32);
    let acc_offset = imu_fusion::FusionVector::new(-0.010749076, -0.0051848157, 0.0190009);

    let mut ahrs_settings = imu_fusion::FusionAhrsSettings::new();
    ahrs_settings.convention = imu_fusion::FusionConvention::NWU;
    ahrs_settings.gain = 0.5f32;
    ahrs_settings.gyr_range = 500.0f32; // replace this with actual gyroscope range in degrees/s
    ahrs_settings.acc_rejection = 10.0f32;
    ahrs_settings.recovery_trigger_period = 5 * SAMPLE_RATE_HZ as i32;

    let mut fusion = imu_fusion::Fusion::new(SAMPLE_RATE_HZ, ahrs_settings);
    fusion.acc_offset = acc_offset;
    fusion.gyr_offset = gyr_offset;
    // let mut offset = imu_fusion::FusionGyrOffset::new(SAMPLE_RATE_HZ);

    let mut prev_time = 0.0;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    for line in no_move_file.lines().map(|l| l.unwrap()) {
        let line = parse_line(&line);

        let gyr = imu_fusion::FusionVector::new(line.gyro_x, line.gyro_y, line.gyro_z);
        // let gyr = imu_fusion::FusionVector::new(0.0, 0.0, 0.0);
        let acc = imu_fusion::FusionVector::new(line.accel_x, line.accel_y, line.accel_z);

        // // Apply calibration
        // let mut gyr =
        //     fusion.inertial_calibration(gyr, gyr_misalignment, gyr_sensitivity, gyr_offset);
        // let acc = fusion.inertial_calibration(acc, acc_misalignment, acc_sensitivity, acc_offset);

        // // Update gyroscope offset correction algorithm
        // gyr = offset.update(gyr);

        fusion.update_no_mag(gyr, acc, line.timestamp);

        let euler = fusion.euler();
        let earth_acc = fusion.earth_acc();
        let time_diff = line.timestamp - prev_time;
        x += (earth_acc.x * 9.81) * time_diff;
        y += (earth_acc.y * 9.81) * time_diff;
        z += (earth_acc.z * 9.81) * time_diff;

        println!(
            "Raw: Accel(x: {}, y: {}, z: {})",
            line.accel_x, line.accel_y, line.accel_z
        );
        println!(
            "Roll {}, Pitch {}, Yaw {}\t\tx: {}, y: {}, z: {}\t({}, {}, {})",
            euler.angle.roll,
            euler.angle.pitch,
            euler.angle.yaw,
            earth_acc.x,
            earth_acc.y,
            earth_acc.z,
            x,
            y,
            z
        );

        prev_time = line.timestamp;
    }
}

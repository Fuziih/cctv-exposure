use std::f64::consts::PI;
use std::io::BufReader;
use std::fs::File;
use std::collections::HashMap;
use std::error::Error;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use hhmmss::Hhmmss;
use std::collections::hash_map::Entry::Vacant;
use gpx::read;
use gpx::{Gpx, TrackSegment};

const R: f64 = 6371.0;
const RESOLUTION: f64 = 0.5;
const ACCEPTRANGE: f64 = 1.0;

#[derive(Debug)]
struct SegmentResult {
    file: String,
    track: i32,
    segment: i32,
    total_distance: f64,
    total_time: String,
    average_speed: f64,
    number_of_unique_cams: i32,
    exposure_distance: f64,
    dist_percentage: f64,
    exposure_time: f64,
    time_percentage: f64,
    camera_dist_average: f64,
    camera_dist_median: f64,
    cameras: HashMap<usize, Camera>,
}

#[derive(Debug, Deserialize, Clone)]
struct Camera {
    latitude: f64,
    longitude: f64,
    camera_type: String,
    radius: f64,
    angle_of_view: i64,
    camera_model: String,
    url: String,
    camera_in_streetview: String,
    #[serde(default = "default_set")]
    points: HashSet<usize>,
    #[serde(default = "default_float")]
    dist: f64,
    #[serde(default = "default_float")]
    time: f64,
}

fn default_set() -> HashSet<usize>{
    HashSet::new()
}
fn default_float() -> f64{
    0.0
}


fn haversine_distance(lat0: f64, lon0: f64, lat1: f64, lon1: f64) -> f64 {

    let lat0 = lat0.to_radians();
    let lat1 = lat1.to_radians();

    let delta_latitude = lat0 - lat1;
    let delta_longitude = (lon0 - lon1).to_radians();

    let central_angle_inner = (delta_latitude / 2.0).sin().powi(2)
        + lat0.cos() * lat1.cos() * (delta_longitude / 2.0).sin().powi(2);
    let central_angle = 2.0 * central_angle_inner.sqrt().asin();

    // return distance in meters
    (R * central_angle * 1000.0).abs()
   
}

fn get_coordinates(lat: f64, lon: f64, bearing: f64, distance: f64) -> (f64, f64) {
    let brng = bearing.to_radians();
    let d = distance / 1000.0;
    let lat = lat.to_radians();
    let lon = lon.to_radians();
    let lat2 = (lat.sin() * (d/R).cos() + lat.cos() * (d/R).sin() * brng.cos()).asin();

    // return new coordinates (tuple)
    (lat2.to_degrees(),
     (lon + (brng.sin() * (d/R).sin() * lat.cos()).atan2((d/R).cos() - lat.sin() * lat2.sin())).to_degrees())
}

fn quick_distance(lat0: f64, lon0: f64, lat1: f64, lon1: f64) -> f64 {
    let x = lat1 - lat0;
    let y = (lon1 - lon0) * ((lat1 + lat0) * 0.00872664626).cos();

    // return distance in meters
    (111319.0 * (x * x + y * y).sqrt()).abs()
}

fn get_bearing(lat0: f64, lon0: f64, lat1: f64, lon1: f64) -> f64 {

    let mut dlon = lon1.to_radians() - lon0.to_radians();
    let dphi = ((lat1.to_radians() / 2.0 + PI / 4.0).tan() / (lat0.to_radians() / 2.0 + PI / 4.0).tan()).ln();

    if dlon.abs() > PI {
        if dlon > 0.0 {
            dlon = -(2.0 * PI - dlon);
        } else {
            dlon += 2.0 * PI;
        }
    };

    // return bearing in degrees
    (dlon.atan2(dphi).to_degrees() + 360.0) % 360.0
}

fn get_total_distance(segment: &TrackSegment) -> f64 {
    let mut distance: f64 = 0.0;
    for (i, point) in segment.points.iter().enumerate().skip(1) {
        let (lon0, lat0) = point.point().x_y();
        let (lon1, lat1) = segment.points[i -1].point().x_y();
        distance += quick_distance(lat0, lon0, lat1, lon1);
    }
     
    // return distance in meters
    distance
}

fn avg_speed_per_point(dist: f64, time1: i64, time2: i64) -> f64 {
    dist / (time1 - time2).abs() as f64
}

fn load_cameras(path: &str) -> Result<Vec<Camera>, Box<dyn Error>> {
    let mut cams: Vec<Camera> = Vec::new();
    let mut reader = csv::Reader::from_path(path)?;

    for result in reader.deserialize::<Camera>() {
        match result {
            Ok(c) => cams.push(c),
            Err(e) => eprintln!("Error with cameradata: {}", e),
        };
    }
    Ok(cams)
}

fn dist_per_camera_attributes(dist: f64, cam: &Camera, lat: f64, lon: f64, addon: f64) -> bool {
    if dist <= cam.radius + addon {
        if cam.camera_type == "round" {
            return true
        } else if cam.camera_type == "directed" {
            // angle to be changed when there is direction data
            let angle = 180.0;  // cam.angle

            let half_fov = (cam.angle_of_view / 2) as f64;
            let fov_range = (angle - half_fov, angle + half_fov);

            let bearing = get_bearing(cam.latitude,cam.longitude, lat, lon);
            return fov_range.0 <= bearing && bearing <= fov_range.1
        } else { return true }
    }
    return false
}

fn track_route(segment: &TrackSegment, cams: &[Camera]) -> (HashMap<usize, Vec<usize>>, HashSet<usize>, Vec<f64>) {
    let mut cameras_per_point: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut u_cams: Vec<usize> = Vec::new();
    let mut all_distances: Vec<f64> = Vec::new();

    for (i, point) in segment.points.iter().enumerate() {
        // points_in_camera_fov
        let mut point_cams: Vec<usize> = Vec::new();
        let (lon, lat) = point.point().x_y();
        for (index, cam) in cams.iter().enumerate() {
            let distance = quick_distance(lat, lon, cam.latitude, cam.longitude);
            all_distances.push(distance);

            // check distance against camera attributes
            if dist_per_camera_attributes(distance, cam, lat, lon, 0.0) {
                point_cams.push(index);
                u_cams.push(index);
            }
        }
        if !point_cams.is_empty() { cameras_per_point.insert(i, point_cams); }
        
    }
    u_cams.sort_unstable();
    let unique_cams: HashSet<_> = u_cams.drain(..).collect(); // dedup
    (cameras_per_point, unique_cams, all_distances)
}

fn calculate_median(distances: &[f64]) -> f64 {   
    if (distances.len() % 2)==0 {
        let ind_left = distances.len()/2-1; 
        let ind_right = distances.len()/2 ;
        (distances[ind_left]+distances[ind_right]) as f64 / 2.0

    } else {
        distances[(distances.len()/2)] as f64
    }
    
}

fn calculate_mean(distances: &[f64]) -> f64 {   
        let sum: f64 = distances.iter().sum();
        sum as f64 / distances.len() as f64
}

fn test_points(mut lat: f64, mut lon: f64, cam: &Camera, course: &f64, points: &i32) -> i32 {
    let mut result: i32 = 0;
    for _ in 1..=*points {
        let (new_lat, new_lon) = get_coordinates(lat, lon, *course, RESOLUTION);
        let cam_distance = quick_distance(new_lat, new_lon, cam.latitude, cam.longitude);
        if dist_per_camera_attributes(cam_distance, cam, new_lat, new_lon, ACCEPTRANGE) {
            result += 1;
        } else { break; }
        lat = new_lat;
        lon = new_lon;
    }
    result
}

fn calculate_direction(cam_expo: &mut HashMap<usize, Camera>, cams_per_point: &HashMap<usize, Vec<usize>>, cams: &mut Vec<Camera>, backward: bool, segment: &TrackSegment) -> (f64, f64) {
    let mut total_time: f64 = 0.0; let mut total_dist: f64 = 0.0; let mut lon1: f64 = 0.0; let mut lat1: f64 = 0.0; let mut time1: i64 = 0;
    for (key, value) in cams_per_point.iter() {
        if backward && *key != 0 {
            (lon1, lat1) = segment.points[*key-1].point().x_y();
            time1 = segment.points[*key-1].time.unwrap().timestamp();
        }
        else if !backward && *key != segment.points.len() - 1 {
            (lon1, lat1) = segment.points[*key+1].point().x_y();
            time1 = segment.points[*key+1].time.unwrap().timestamp();
        } else {
            continue;
        }
        let (lon0, lat0) = segment.points[*key].point().x_y();
        let time0 = segment.points[*key].time.unwrap().timestamp();
        let mut highest_time = 0.0; let mut highest_dist = 0.0;
        let course = get_bearing(lat0, lon0, lat1, lon1);
        let distance = quick_distance(lat0, lon0, lat1, lon1);
        let points: i32 = if distance > RESOLUTION  { (distance / RESOLUTION).round() as i32 }  else { 1 };

        for cam in value {
            if !backward && cam_expo[&(*cam)].points.contains(&(&*key + 1)) { continue; }
            else {
                let pseudo_points: i32 = if backward && cams_per_point.contains_key(&(&*key - 1)) && cams_per_point[&(*key - 1)].contains(&*cam) {
                    points
                } else {
                    test_points(lat0, lon0, &cams[*cam], &course, &points)
                };

                let avg = avg_speed_per_point(distance, time0, time1);
                let cam_time: f64 = if avg != 0.0 {
                    pseudo_points as f64 / (1.0 / RESOLUTION) / avg
                } else {
                    (time1 - time0).abs() as f64
                };

                let cam_dist = pseudo_points as f64 / (1.0 / RESOLUTION);

                if cam_time > highest_time { highest_time = cam_time; }
                if cam_dist > highest_dist { highest_dist = cam_dist; }

                if let Vacant(e) = cam_expo.entry(*cam) {
                    let mut cam_entry = &mut cams[*cam];
                    cam_entry.points.insert(*key); cam_entry.dist += cam_dist; cam_entry.time += cam_time;
                    e.insert(cam_entry.to_owned());
                } else {
                    cam_expo.entry(*cam).and_modify(|cam_entry| {
                    cam_entry.points.insert(*key); cam_entry.dist += cam_dist; cam_entry.time += cam_time;
                   });
                }
            }
        }

        total_dist += highest_dist;
        total_time += highest_time;
    }
    (total_dist, total_time)
}

fn main() {

    let path = env::args().nth(1).expect("No .gpx file path."); 
    let cam_path = env::args().nth(2).expect("No camerafile path."); 
    let mut cams = match load_cameras(&cam_path) {
        Ok(cam) => cam,
        Err(error) => {
            panic!("Error with cameradata: {:?}", error)
        },
    };

    // Open .gxp
    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    // read takes any io::Read and gives a Result<Gpx, Error>.
    let gpx: Gpx = match read(reader) {
        Ok(res) => res,
        Err(error) => {
            panic!("Error with gpx file: {:?}", error)
        },
    };


    // iterate over track and segments
    for (t, track) in gpx.tracks.iter().enumerate() {
        for (s, segment) in track.segments.iter().enumerate() {
            let (cameras_per_point, unique_cams, mut distances) = track_route(segment, &cams);
            // sort distances for median calculation
            distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let mut cam_expo: HashMap<usize, Camera> = HashMap::new();
            let (b_dist, b_time) = calculate_direction(&mut cam_expo, &cameras_per_point, &mut cams, true, segment);
            let (f_dist, f_time) = calculate_direction(&mut cam_expo, &cameras_per_point, &mut cams, false, segment);

            let total_distance = get_total_distance(segment);
            let total_time = segment.points[segment.points.len() - 1].time.unwrap() - segment.points[0].time.unwrap();
            let dist = b_dist + f_dist; let time = b_time + f_time;
            let dist_percentage = dist/total_distance * 100.0;
            let time_percentage = time / total_time.num_seconds() as f64 * 100.0;
            // let dist_neat = format!("{}% {}/{}", dist/total_distance * 100.0, dist, total_distance);
            // let time_neat = format!("{}% {}/{:?}", time / total_time_secs as f64 * 100.0, time, total_time);

            let (_, name) = &path.rsplit_once('/').unwrap();
            let result = SegmentResult {
                file: name.to_owned().to_string(), track: t as i32, segment: s as i32,
                total_distance, total_time: total_time.hhmmss(),
                average_speed: avg_speed_per_point(total_distance, total_time.num_seconds(), 0) * 3.6,
                number_of_unique_cams: unique_cams.len() as i32,
                exposure_distance: dist, dist_percentage,
                exposure_time: time, time_percentage,
                camera_dist_average: calculate_mean(&distances), camera_dist_median: calculate_median(&distances),
                cameras: cam_expo
            };

            println!("{:#?}", result)
        }
    }
}

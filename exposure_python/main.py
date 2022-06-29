import json
import os
import csv
import logging
import argparse
import gpxpy
import gpxpy.gpx
import numpy as np
import random

LOGGER = logging.getLogger()
R = 6371
resolution = 0.5    # pseudo point distance in meters


def load_gpx(file):
    """ Load gpx-files"""
    with open(file, 'r') as gpxfile:
        gpx = gpxpy.parse(gpxfile)
    return gpx


def haversine_distance(lat0, lon0, lat1, lon1):
    lat_diff_rad = np.deg2rad(lat1 - lat0)
    lon_diff_rad = np.deg2rad(lon1 - lon0)
    radlat = np.deg2rad(lat0)
    radlat1 = np.deg2rad(lat1)
    a = np.power(np.sin(lat_diff_rad / 2.), 2.) + np.power(np.sin(lon_diff_rad / 2.), 2.) * np.cos(radlat) * np.cos(
        radlat1)
    c = 2 * np.arctan2(np.sqrt(a), np.sqrt(1 - a))
    return R * c * 1000


def get_coordinates(lat, lng, bearing, distance):
    """ Return new coordinates based on old coordinates, bearing, and distance """
    brng = np.deg2rad(bearing)
    d = distance / 1000  # Distance m converted to km
    lat1 = np.deg2rad(lat)  # Current dd lat point converted to radians
    lon1 = np.deg2rad(lng)  # Current dd long point converted to radians
    lat2 = np.arcsin(np.sin(lat1) * np.cos(d / R) + np.cos(lat1) * np.sin(d / R) * np.cos(brng))
    lon2 = lon1 + np.arctan2(np.sin(brng) * np.sin(d / R) * np.cos(lat1), np.cos(d / R) - np.sin(lat1) * np.sin(lat2))
    return [np.degrees(lat2), np.degrees(lon2)]


def quick_distance(lat0, lon0, lat1, lon1):
    """ Calculate distance between coordinates using Euclidean distance (quick and accurate enough for really short
    distances """
    x = lat1 - lat0
    y = (lon1 - lon0) * np.cos((lat1 + lat0) * 0.00872664626)
    """
    111.319 - is the distance that corresponds to 1degree at Equator,
    you could replace it with your median value like here https://www.cartographyunchained.com/cgsta1/ or
    replace it with a simple lookup table.
    """
    return 111319 * np.sqrt(x * x + y * y)


def get_bearing(lat0, lon0, lat1, lon1):
    """ Get bearing between coordinates """
    dlon = np.deg2rad(lon1) - np.deg2rad(lon0)
    dphi = np.log(np.tan(np.deg2rad(lat1) / 2.0 + np.pi / 4.0) / np.tan(np.deg2rad(lat0) / 2.0 + np.pi / 4.0))
    if abs(dlon) > np.pi:
        if dlon > 0.0:
            dlon = -(2.0 * np.pi - dlon)
        else:
            dlon = (2.0 * np.pi + dlon)
    return (np.degrees(np.arctan2(dlon, dphi)) + 360.0) % 360.0


class CCTVExposure:
    """ CCTV Exposure main class """

    def __init__(self, args, gpx):
        self.camfile = args.camfile
        self.radius = args.radius
        self.accept_range = 1.0  # how many meters above radius meters, that we accept the "second in fov"
        self.cams = self.load_cameras()
        self.time_enabled = True
        self.points = gpx.points
        self.distances = []
        self.cameras_per_point = dict()
        self.unique_cameras = set()
        self.cam_amount = 0
        self.cam_expo = {}
        self.speed_enabled = False
        self.track_route()
        self.total_distance = self.get_total_distance()
        self.total_time = self.points[-1].time - self.points[0].time if self.time_enabled else 0

    def get_total_distance(self):
        """ Calculate total distance """
        distance = 0
        number_of_points = len(self.points)
        for i in range(number_of_points - 1):
            distance += quick_distance(self.points[-1 - i].latitude, self.points[-1 - i].longitude,
                                       self.points[-2 - i].latitude, self.points[-2 - i].longitude)
        return distance

    def track_route(self):
        """ Go through route (single segment gpx) """
        for index, point in enumerate(self.points):
            if not point.time:
                self.time_enabled = False
            predictions, distances = self.point_in_camera_fov(point.latitude, point.longitude)
            if point.speed:
                self.speed_enabled = True

            self.distances.extend(distances)
            if predictions:
                self.cameras_per_point.update({index: predictions})
                self.unique_cameras.update(predictions)

        self.cam_amount = len(self.unique_cameras)

    def point_in_camera_fov(self, lat, lon):
        """ Check if a point is in camera area"""
        results = dict()
        distances = []
        for cam in self.cams.items():
            distance = quick_distance(lat, lon, float(cam[1].get('latitude')), float(cam[1].get('longitude')))
            distances.append(distance)
            if self.check_dist_to_cam(distance, cam[1], lat, lon):
                results.update({cam[0]: cam[1]})
        return results, distances

    def check_dist_to_cam(self, d, cam, lat, lon, addon=0.0):
        """ Check distance against camera FoV """
        fov = self.radius if self.radius else float(cam.get('radius', 10))
        if d <= fov + addon:
            camtype = cam.get('camera type', 'round')
            if camtype == 'round':
                return True
            elif camtype == 'directed':
                angle = int(cam.get('angle of view', '360'))
                direction = cam.get('direction', None)
                if angle < 360:
                    fov_range = ((direction - angle / 2 + 360.0) % 360.0, (direction + angle / 2 + 360.0) % 360.0)
                    bearing = get_bearing(float(cam.get('latitude')), float(cam.get('longitude')), lat, lon)
                    if fov_range[0] <= bearing <= fov_range[1]:
                        return True
                else:  # 360-degree directed
                    return True
        return False

    def avg_speed_per_point(self, dist, point1, point2):
        """ Calculate average speed per point """
        return dist / np.absolute((self.points[point2].time - self.points[point1].time).total_seconds())

    def test_points(self, ind, points, cam, course):
        """ Test time in camera fov per second """
        result = 0
        lat = self.points[ind].latitude
        lon = self.points[ind].longitude
        for point in range(points):
            new_lat, new_lon = get_coordinates(lat, lon, course, resolution)
            cam_distance = quick_distance(new_lat, new_lon, float(self.cams[cam].get('latitude')),
                                          float(self.cams[cam].get('longitude')))

            if self.check_dist_to_cam(cam_distance, self.cams[cam], new_lat, new_lon, addon=self.accept_range):
                result += 1
            else:
                break
            lat = new_lat
            lon = new_lon
        return result

    def calculate_direction(self, backward):
        """ Calculate time and distance for one direction """
        total_seconds, total_meters = 0.0, 0.0
        for point_ind, cameras in self.cameras_per_point.items():
            if backward:
                if point_ind != 0:
                    other_point = point_ind - 1
                else:
                    continue
            else:
                if point_ind != len(self.points) -1:
                    other_point = point_ind + 1
                else:
                    continue
            highest_dist = 0.0
            highest_time = 0.0
            course = get_bearing(self.points[point_ind].latitude, self.points[point_ind].longitude,
                                 self.points[other_point].latitude, self.points[other_point].longitude)
            distance = quick_distance(self.points[point_ind].latitude, self.points[point_ind].longitude,
                                      self.points[other_point].latitude,
                                      self.points[other_point].longitude)
            points = int(np.rint(distance / resolution)) if distance > resolution else 1

            for cam in cameras.keys():
                if cam not in self.cam_expo.keys():
                    self.cam_expo.update({cam: {'points': [], 'dist': 0.0, 'time': 0.0}})
                if not backward and point_ind + 1 in self.cam_expo[cam]['points']:
                    continue
                else:
                    if point_ind not in self.cam_expo[cam]['points']:
                        self.cam_expo[cam]['points'].append(point_ind)
                    if backward and self.cameras_per_point.get(point_ind - 1) and \
                            cam in self.cameras_per_point[point_ind - 1]:
                        pseudo_points = points
                    else:
                        pseudo_points = self.test_points(point_ind, points, cam, course)

                    speed = 0
                    if self.speed_enabled:
                        speed = self.points[point_ind].speed
                    elif self.time_enabled:
                        speed = self.avg_speed_per_point(distance, point_ind, other_point)
                    if speed != 0:
                        cam_time = (pseudo_points / (1 / resolution)) / speed
                    else:
                        cam_time = np.abs(self.points[other_point].time.timestamp() - self.points[point_ind].time.timestamp())
                    if cam_time > highest_time:
                        highest_time = cam_time
                    self.cam_expo[cam]['time'] += round(cam_time, 2)

                    cam_dist = float(pseudo_points / (1 / resolution))
                    if cam_dist > highest_dist:
                        highest_dist = cam_dist
                    self.cam_expo[cam]['dist'] += cam_dist

            total_seconds += highest_time
            total_meters += highest_dist

        return total_meters, total_seconds

    def time_and_distance_in_camera_fov(self):
        """ Calculate time and distance in camera fov for each point """
        back_d, back_t = self.calculate_direction(backward=True)
        front_d, front_t = self.calculate_direction(backward=False)
        return back_d + front_d, back_t + front_t

    def calc_distance_stats(self):
        """ Calculate statistics on waypoint distances on cameras"""
        avg = np.average(self.distances) if self.distances else 0
        median = np.median(self.distances) if self.distances else 0
        return avg, median

    def load_cameras(self):
        """ Load camerafile """
        try:
            with open(self.camfile, 'r') as camfile:
                reader = csv.reader(camfile)
                columns = next(reader)
                cams = dict()
                for index, row in enumerate(reader):
                    cams[index] = dict()
                    for i, column in enumerate(row):
                        if column:
                            cams.get(index, {}).update({columns[i]: column})
                return cams
        except (PermissionError, FileNotFoundError) as exc:
            LOGGER.error(str(exc))
            raise


def main(args):
    """ CCTV Exposure main function """

    gpx = load_gpx(args.gpxfile)

    # Iterate over tracks and segments
    for t_ind, track in enumerate(gpx.tracks):
        for s_ind, segment in enumerate(track.segments):

            exposure = CCTVExposure(args, segment)

            dist, time = exposure.time_and_distance_in_camera_fov()
            avg, median = exposure.calc_distance_stats()
            dist_percentage = dist / exposure.total_distance * 100

            result = {
                'file': f"{os.path.basename(args.gpxfile)}",
                'track': t_ind,
                'segment': s_ind,
                'total_distance': exposure.total_distance,
                'number_of_unique_cams': exposure.cam_amount,
                'exposure_distance': dist,
                'dist_percentage': round(dist_percentage, 2),
                #'dist_neat': f"{round(dist_percentage, 2)}% "
                #             f"({round(dist, 2)}/{round(exposure.total_distance, 2)})",
                'camera_distance_avg': avg,
                'camera_distance_median': median,
                'cameras': {}
            }
            if exposure.time_enabled:
                time_percentage = time / exposure.total_time.total_seconds() * 100
                result.update({'avg_speed': round(exposure.total_distance / exposure.total_time.total_seconds() * 3.6, 2),
                               #'time_neat': f"{round(time_percentage, 2)}% "
                               #f" {str(timedelta(seconds=time))}/{exposure.total_time}",
                               'time_percentage': round(time_percentage, 2),
                               'exposure_time': time})
            if exposure.radius:
                result.update({'fov_radius': exposure.radius})
            for cam in exposure.unique_cameras:
                exposure.cams[cam].update({'time_in_camera_fov': exposure.cam_expo[cam].get('time'),
                                           'distance_in_camera_fov': exposure.cam_expo[cam].get('dist')})
                result['cameras'].update({cam: exposure.cams[cam]})

            print(json.dumps(result, indent=4))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='CCTV Exposure')
    parser.add_argument('-c', '--camfile', type=str, help='Path to the camera file.',
                        default='/home/fusier/gitlab/cctv-aware/code/cctv-exposure/exposure/src/all_cameras_standart10.csv')
    parser.add_argument('-g', '--gpxfile', type=str, help='Path to the gpx file.',
                        default='/home/fusier/Documents/cctv/test/real1.gpx')
    parser.add_argument('-r', '--radius', required=False, type=int,
                        help='Field-of-view for cameras, overrides individual camera settings')
    main(parser.parse_args())

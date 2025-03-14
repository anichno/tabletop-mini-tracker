#!/usr/bin/env python3

import base64
import json
import math
from dataclasses import dataclass
from io import BytesIO
from typing import List, Tuple

import microdots as mdots
import numpy as np
from microdots.mini_sequences import A1, A2, MNS
from PIL import Image, ImageDraw

GRID_SPACING = 7.666666666666667
DOT_CENTER_TO_GRID = GRID_SPACING / 3
IMAGE_SIZE = 36


@dataclass
class Point:
    x: float
    y: float

    def __init__(self, x, y):
        self.x = x
        self.y = y

    def distance(self, other):
        return math.sqrt((self.x - other.x) ** 2 + (self.y - other.y) ** 2)

    def angle(self, other):
        dx = self.x - other.x
        dy = self.y - other.y

        radians = math.atan2(dy, dx)
        degrees = math.degrees(radians)
        if degrees < 0:
            degrees += 180

        return degrees

    def angle_360(self, other):
        dx = other.x - self.x
        dy = other.y - self.y

        radians = math.atan2(dy, dx)
        degrees = math.degrees(radians)

        return degrees


class Blob:
    points: List[Tuple[int, int]]

    def __init__(self):
        self.points = list()

    def area(self):
        return len(self.points)

    def max_length(self):
        max_len = 0
        for i in range(len(self.points)):
            point_1 = self.points[i]
            for j in range(i + 1, len(self.points)):
                point_2 = self.points[j]
                distance = math.sqrt(
                    (point_1[0] - point_2[0]) ** 2 + (point_1[1] - point_2[1]) ** 2
                )
                if distance > max_len:
                    max_len = distance

        return max_len

    def split(self):
        max_len = 0
        max_point_1 = None
        max_point_2 = None
        for i in range(len(self.points)):
            point_1 = self.points[i]
            for j in range(i + 1, len(self.points)):
                point_2 = self.points[j]
                distance = math.sqrt(
                    (point_1[0] - point_2[0]) ** 2 + (point_1[1] - point_2[1]) ** 2
                )
                if distance > max_len:
                    max_point_1 = point_1
                    max_point_2 = point_2
                    max_len = distance

        max_len = 0
        max_point_3 = None
        max_point_4 = None
        for i in range(len(self.points)):
            point_1 = self.points[i]
            if point_1 == max_point_1 or point_1 == max_point_2:
                continue

            for j in range(i + 1, len(self.points)):
                point_2 = self.points[j]
                if point_2 == max_point_1 or point_2 == max_point_2:
                    continue
                distance = math.sqrt(
                    (point_1[0] - point_2[0]) ** 2 + (point_1[1] - point_2[1]) ** 2
                )
                if distance > max_len:
                    max_point_3 = point_1
                    max_point_4 = point_2
                    max_len = distance

        distance1 = math.sqrt(
            (max_point_1[0] - max_point_3[0]) ** 2
            + (max_point_1[1] - max_point_3[1]) ** 2
        )
        distance2 = math.sqrt(
            (max_point_1[0] - max_point_4[0]) ** 2
            + (max_point_1[1] - max_point_4[1]) ** 2
        )
        if distance1 < distance2:
            midpoint1_x = (max_point_1[0] + max_point_3[0]) / 2
            midpoint1_y = (max_point_1[1] + max_point_3[1]) / 2
            midpoint2_x = (max_point_2[0] + max_point_4[0]) / 2
            midpoint2_y = (max_point_2[1] + max_point_4[1]) / 2
            midpoint1 = (midpoint1_x, midpoint1_y)
            midpoint2 = (midpoint2_x, midpoint2_y)
        else:
            midpoint1_x = (max_point_1[0] + max_point_4[0]) / 2
            midpoint1_y = (max_point_1[1] + max_point_4[1]) / 2
            midpoint2_x = (max_point_2[0] + max_point_3[0]) / 2
            midpoint2_y = (max_point_2[1] + max_point_3[1]) / 2
            midpoint1 = (midpoint1_x, midpoint1_y)
            midpoint2 = (midpoint2_x, midpoint2_y)

        # build 2 clusters around max_point_1/2
        blob_1 = Blob()
        blob_2 = Blob()

        for point in self.points:
            # distance1 = math.sqrt((point[0] - max_point_1[0])**2 + (point[1] - max_point_1[1])**2)
            # distance2 = math.sqrt((point[0] - max_point_2[0])**2 + (point[1] - max_point_2[1])**2)
            distance1 = math.sqrt(
                (point[0] - midpoint1[0]) ** 2 + (point[1] - midpoint1[1]) ** 2
            )
            distance2 = math.sqrt(
                (point[0] - midpoint2[0]) ** 2 + (point[1] - midpoint2[1]) ** 2
            )

            if distance1 < distance2:
                blob_1.points.append(point)
            else:
                blob_2.points.append(point)

        return (blob_1, blob_2)

    def center(self):
        avg_x = 0
        avg_y = 0
        for point in self.points:
            avg_x += point[0]
            avg_y += point[1]

        avg_x /= len(self.points)
        avg_y /= len(self.points)

        return (avg_x, avg_y)


def label_blobs(img_array):
    blobs = dict()
    cur_blob_id = 2
    for y, row in enumerate(img_array):
        for x, col in enumerate(row):
            if col == 1:
                # scan surrounding pixels for group id
                found_id = 0
                for ydiff in (-1, 0, 1):
                    if y + ydiff >= len(img_array) or y + ydiff < 0:
                        continue
                    if found_id > 0:
                        break
                    for xdiff in (-1, 0, 1):
                        if x + xdiff >= len(img_array) or x + xdiff < 0:
                            continue
                        if xdiff == 0 and ydiff == 0:
                            continue
                        if img_array[y + ydiff][x + xdiff] > 1:
                            found_id = img_array[y + ydiff][x + xdiff]
                            break

                if found_id == 0:
                    found_id = cur_blob_id
                    cur_blob_id += 1

                img_array[y][x] = found_id
                if found_id not in blobs:
                    blobs[found_id] = Blob()
                blobs[found_id].points.append((x, y))

                # for every pixel around this one, assign it to the same cur_blob_id
                for ydiff in (-1, 0, 1):
                    if y + ydiff >= len(img_array) or y + ydiff < 0:
                        continue
                    for xdiff in (-1, 0, 1):
                        if x + xdiff >= len(img_array) or x + xdiff < 0:
                            continue
                        if xdiff == 0 and ydiff == 0:
                            continue
                        if img_array[y + ydiff][x + xdiff] == 1:
                            img_array[y + ydiff][x + xdiff] = found_id
                            blobs[found_id].points.append((x + xdiff, y + ydiff))

    return blobs


# def build_refere


# def complete_grid(
#     point_pairs: List[Tuple[Tuple[float, float], Tuple[float, float]]],
# ) -> List[Tuple[float, float]]:
#     """
#     Fill out a grid based on pairs of adjacent points and an axis-aligned bounding box.

#     Args:
#         point_pairs: List of point pairs, where each pair contains adjacent grid points
#                      that are one unit of spacing apart.
#                      Format: [((x1, y1), (x2, y2)), ...]
#         bounding_box: Axis-aligned bounding box [min_x, min_y, max_x, max_y]

#     Returns:
#         List of all grid points within the bounding box.
#     """
#     # Step 1: Extract individual points from pairs
#     all_points = []
#     for pair in point_pairs:
#         all_points.extend(pair)
#     # Remove duplicates
#     all_points = list({point for point in all_points})

#     # Step 2: Determine grid vectors
#     vectors = []
#     for p1, p2 in point_pairs:
#         vector = (p2[0] - p1[0], p2[1] - p1[1])
#         vectors.append(vector)

#     # Find two linearly independent vectors
#     selected_vectors = []
#     for v in vectors:
#         # Skip zero vectors
#         if v[0] == 0 and v[1] == 0:
#             continue

#         # Normalize vector
#         length = math.sqrt(v[0] ** 2 + v[1] ** 2)
#         normalized_v = (v[0] / length, v[1] / length)

#         # Check if vector is linearly independent from already selected ones
#         is_independent = True
#         for selected_v in selected_vectors:
#             # If vectors are parallel (dot product ≈ ±1)
#             dot_product = abs(
#                 normalized_v[0] * selected_v[0] + normalized_v[1] * selected_v[1]
#             )
#             if abs(dot_product - 1.0) < 1e-10:
#                 is_independent = False
#                 break

#         if is_independent:
#             selected_vectors.append(normalized_v)
#             if len(selected_vectors) == 2:
#                 break

#     if len(selected_vectors) < 2:
#         # If we don't have 2 independent vectors, try to infer the second one
#         if len(selected_vectors) == 1:
#             # Create a perpendicular vector to the one we have
#             v = selected_vectors[0]
#             perpendicular_v = (-v[1], v[0])
#             selected_vectors.append(perpendicular_v)
#         else:
#             raise ValueError(
#                 "Could not determine grid orientation from provided points"
#             )

#     # Get unit vectors with correct magnitudes
#     unit_vectors = []
#     for v_direction in selected_vectors:
#         # Find the best matching actual vector with this direction
#         best_match = None
#         min_angle_diff = float("inf")

#         for v in vectors:
#             if v[0] == 0 and v[1] == 0:
#                 continue

#             v_length = math.sqrt(v[0] ** 2 + v[1] ** 2)
#             v_norm = (v[0] / v_length, v[1] / v_length)

#             # Compute angle difference
#             dot_product = v_norm[0] * v_direction[0] + v_norm[1] * v_direction[1]
#             angle_diff = abs(1.0 - abs(dot_product))

#             if angle_diff < min_angle_diff:
#                 min_angle_diff = angle_diff
#                 best_match = v

#         if best_match is not None:
#             unit_vectors.append(best_match)
#         else:
#             # If no match found, use the direction vector with unit length
#             unit_vectors.append(v_direction)

#     # Step 3: Determine grid origin (can be any of the input points)
#     origin = all_points[0]

#     # Step 4: Find min/max grid indices needed to cover the bounding box
#     min_x, min_y, max_x, max_y = (0, 0, 35, 35)

#     # Create vectors as tuples
#     v1 = unit_vectors[0]
#     v2 = unit_vectors[1]

#     # Estimate the range of grid indices needed
#     # This is a conservative estimate to ensure we include all points within the box
#     max_distance = max(
#         math.sqrt((max_x - min_x) ** 2 + (max_y - min_y) ** 2),
#         math.sqrt((max_x - origin[0]) ** 2 + (max_y - origin[1]) ** 2),
#         math.sqrt((min_x - origin[0]) ** 2 + (min_y - origin[1]) ** 2),
#     )

#     # Ensure we cover enough space
#     index_range = int(max_distance * 2) + 5  # Add some buffer

#     # Step 5: Generate all potential grid points
#     grid_points = []
#     for i in range(-index_range, index_range + 1):
#         for j in range(-index_range, index_range + 1):
#             x = origin[0] + i * v1[0] + j * v2[0]
#             y = origin[1] + i * v1[1] + j * v2[1]
#             grid_points.append((x, y))

#     # Step 6: Filter points to only those inside the axis-aligned bounding box
#     filtered_points = [
#         p for p in grid_points if min_x <= p[0] <= max_x and min_y <= p[1] <= max_y
#     ]

#     return filtered_points


# def get_grid_from_adjacent(adj_blobs):


def points_at_distance(p1, p2, distance):
    """
    Calculate two new points at a specified distance from the original points,
    extending the line in both directions.

    Args:
        p1: Tuple (x, y) of the first point
        p2: Tuple (x, y) of the second point
        distance: The distance to place the new points from the original points

    Returns:
        Tuple (p0, p3): Two new points - p0 before p1 and p3 after p2
    """
    # Calculate direction vector
    dx = p2[0] - p1[0]
    dy = p2[1] - p1[1]

    # Calculate length of the direction vector
    length = math.sqrt(dx**2 + dy**2)

    # Normalize to get unit vector
    if length > 0:
        unit_x = dx / length
        unit_y = dy / length
    else:
        raise ValueError("Points must be different to define a line")

    # Calculate new points
    p0 = (p1[0] - distance * unit_x, p1[1] - distance * unit_y)
    p3 = (p2[0] + distance * unit_x, p2[1] + distance * unit_y)

    return p0, p3


def cluster_pairs(point_pairs: List[Tuple[Point, Point, float]], tolerance):
    point_pairs.sort(key=lambda x: x[2])
    clusters = list()
    cur_cluster = [point_pairs[0]]
    cur_angle = point_pairs[0][2]

    for pair in point_pairs[1:]:
        angle = pair[2]
        if abs(angle - cur_angle) <= tolerance:
            cur_cluster.append(pair)
        else:
            clusters.append(cur_cluster)
            cur_cluster = [pair]
            cur_angle = angle

    clusters.append(cur_cluster)
    clusters.sort(key=lambda x: len(x))
    clusters.reverse()

    return clusters


def grid_to_blob_direction(g: Point, b: Point, tolerance: float):
    angle = g.angle_360(b)

    if -tolerance <= angle <= tolerance:
        return "R"
    elif 90 - tolerance <= angle <= 90 + tolerance:
        return "D"
    elif -90 - tolerance <= angle <= -90 + tolerance:
        return "U"
    elif (180 - tolerance <= angle <= 180) or (-180 <= angle <= -180 + tolerance):
        return "L"
    else:
        return "!"
    
def extract_4x4(solve_array, start_x, start_y):
    north = np.array([0, 0])
    south = np.array([1, 1])
    west = np.array([1, 0])
    east = np.array([0, 1])

    extracted = list()
    for y in range(start_y, len(solve_array)):
        row = list()
        for x in range(start_x, len(solve_array[y])):
            val = solve_array[y][x]
            if val != "*" and val != "!":
                if val == "U":
                    val = north
                elif val == "D":
                    val = south
                elif val == "L":
                    val = west
                elif val == "R":
                    val = east
                else:
                    raise Exception(f"Invalid val: {val}")
                row.append(val)
                if len(row) == 4:
                    break
            else:
                return None
        if len(row) == 4:
            extracted.append(row)
            if len(extracted) == 4:
                break
        else:
            return None

    if len(extracted) == 4:
        return np.array(extracted)
    else:
        return None

def find_4x4(solve_array):
    for y, row in enumerate(solve_array):
        for x, col in enumerate(row):
            if col != "*" and col != "!":
                extracted = extract_4x4(solve_array, x, y)
                if extracted is not None:
                    return extracted
    
# def encode_solution(solve_array):
#     north = np.array([0, 0])
#     south = np.array([1, 1])
#     west = np.array([1, 0])
#     east = np.array([0, 1])

#     for row in solve_array:
#         for col in row:
#             if col == "U":
#                 col = north
#             elif col == "D":
#                 col = south
#             elif col == "L":
#                 col = west
#             elif col == "R":
#                 col = east
#             else:
#                 raise Exception("Invalid direction")
    
#     return np.array(solve_array)


def solve(image: Image):
    draw = ImageDraw.Draw(image)
    # first get image into 2d array, since that is what we will actually process
    img_array = np.array(image.convert("L"))
    # print(img_array.shape)
    np.set_printoptions(linewidth=np.inf, threshold=np.inf)
    # print(img_array)

    thresholded = list()
    for row in img_array:
        new_row = list()
        for col in row:
            if col < 200:
                new_row.append(1)
            else:
                new_row.append(0)

        thresholded.append(new_row)

    # for row in thresholded:
    #     print(row)

    # print()
    blobs = label_blobs(thresholded)
    # for row in thresholded:
    #     for col in row:
    #         if col == 0:
    #             print("    ", end="")
    #         else:
    #             print(f"{col:02}  ", end="")
    #     print()
    # print()

    # for blob in blobs:
    #     print(blob, blobs[blob].area(), blobs[blob].max_length())
    # print(blobs)

    new_blobs = list()
    adjacent_blobs = list()
    for blob in blobs:
        if blobs[blob].max_length() > DOT_CENTER_TO_GRID * 2:
            blob_1, blob_2 = blobs[blob].split()
            new_blobs.append(blob_1)
            new_blobs.append(blob_2)
            adjacent_blobs.append((blob_1, blob_2))
        else:
            new_blobs.append(blobs[blob])

    # print()
    blob_centers = list()
    for blob in new_blobs:
        x, y = blob.center()
        draw.point((round(x), round(y)), fill="blue")
        # print(x, y)
        blob_centers.append(Point(x, y))

    grid_pairs = list()
    for i, p1 in enumerate(blob_centers):
        for p2 in blob_centers[i + 1 :]:
            dist = p1.distance(p2)
            for loc in range(1, 3):
                if abs(dist - loc * GRID_SPACING) < 0.05 * GRID_SPACING:
                    grid_pairs.append((p1, p2, p1.angle(p2)))
                    break

    # print("Pairs")
    # for pair in grid_pairs:
    #     print(pair)
    #     p1 = pair[0]
    #     x1,y1 = p1.x,p1.y
    #     p2 = pair[1]
    #     x2,y2 = p2.x,p2.y
    #     draw.line(((x1,y1), (x2,y2)), fill="red", width=1)

    # cluster based on angle, there should be two clusters roughly 90 deg apart
    clusters = cluster_pairs(grid_pairs, 5.0)

    # print("Clusters")
    # for cluster in clusters:
    #     print(len(cluster), cluster)

    # if more than 1 cluster, avg the cluster angles and make sure they are ~90 deg apart. if not, only use the first cluster and calc 90 deg off of it
    angle1 = 0.0
    for pair in clusters[0]:
        angle1 += pair[2]
    angle1 /= len(clusters[0])

    angle2 = None
    if len(clusters) > 1:
        angle2 = 0.0
        for pair in clusters[1]:
            angle2 += pair[2]
        angle2 /= len(clusters[1])

        diff = angle1 - angle2
        if diff < 0:
            diff += 180

        if abs(90 - diff) > 5:
            angle2 = None

    if angle2 is None:
        angle2 = angle1 - 90
        if angle2 < 0:
            angle2 += 180

    print("Angles:", angle1, angle2)

    # TODO: rotate all points so that grid is horizontal and vertical

    # starting with an arbitrary first point, create the horizontal and vertical grid points, going 1 line past the bounds of the view window
    cur_point = blob_centers[0]
    cur_x = cur_point.x
    cur_y = cur_point.y
    while cur_y > -GRID_SPACING:
        cur_y -= GRID_SPACING

    while cur_x > -GRID_SPACING:
        cur_x -= GRID_SPACING

    grid_points = list()
    while cur_y < 36 + GRID_SPACING:
        x = cur_x
        row = list()
        while x < 36 + GRID_SPACING:
            row.append(Point(x, cur_y))
            x += GRID_SPACING
        cur_y += GRID_SPACING
        grid_points.append(row)

    # TODO: Determine how to shift grid points. Shift should only be by DOT_CENTER_TO_GRID
    for row in grid_points:
        for col in row:
            col.y += DOT_CENTER_TO_GRID

    code_points = list()
    for row in grid_points:
        crow = list()
        for col in row:
            draw.point((round(col.x), round(col.y)), fill="green")
            # find blob center that is within DOT_CENTER_TO_GRID (+margin), then derive that grid points direction
            found = False
            for b in blob_centers:
                if col.distance(b) < DOT_CENTER_TO_GRID * 1.25:
                    crow.append(grid_to_blob_direction(col, b, 20.0))
                    found = True
                    break

            if not found:
                crow.append("*")

        code_points.append(crow)

    # TODO: if any side has some known, where the known ones are in line with the side, any unknowns can be inferred to be just outside the view window

    for row in code_points:
        for col in row:
            print(col, end="")
        print()

    # check for a contiguous 4x4 region in code_points, then convert that to the format expected by py-microdots
    extracted = find_4x4(code_points)

    # pass matrix to py-microdots and return the coords
    codec4x4 = mdots.AnotoCodec(
        mns=MNS,
        mns_order=4,
        sns=[A1, A2],
        pfactors=(3, 5),
        delta_range=(1, 15),
    )
    pos = codec4x4.decode_position(extracted)
    print(f"Pos: {pos}")
    sec = codec4x4.decode_section(extracted, pos=pos)
    print(f"Sec: {sec}")


    image.save("test.png")
    # show_image(image)


def show_image(image):
    display = image.resize((400, 400), Image.LANCZOS)
    display.show()


if __name__ == "__main__":
    with open("testcases.json", "r") as infi:
        test_cases = json.load(infi)

    test_case = test_cases[98]
    for key,val in test_case.items():
        if key != "image":
            print(key, ":", val)
    # test_case = test_cases[0]
    img_bytes = base64.b64decode(test_case["image"])
    img = Image.open(BytesIO(img_bytes))

    # show_image(img)

    solve(img)

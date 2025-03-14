#!/usr/bin/env python3

import base64
import json
from io import BytesIO

import microdots as mdots
import numpy
from microdots.mini_sequences import A1, A2, MNS
from PIL import Image, ImageDraw

WIDTH = 3295
HEIGHT = 2539
MARGIN = 120
SPACING = 3
DPI = 300

CAMERA_VIEW_PIXELS = 14
CAMERA_RESOLUTION = 36


class TestCase:
    image: str
    x: int
    y: int

    def __init__(self, img, x, y):
        buffered = BytesIO()
        img.save(buffered, format="PNG")
        img_bytes = buffered.getvalue()
        self.image = base64.b64encode(img_bytes).decode("utf-8")
        self.x = x
        self.y = y


def codec():
    codec4x4 = mdots.AnotoCodec(
        mns=MNS,
        mns_order=4,
        sns=[A1, A2],
        pfactors=(3, 5),
        delta_range=(1, 15),
    )

    return codec4x4


def process_selection(img: Image, x, y):
    # Extract the subset
    subset = img.crop((x, y, x + CAMERA_VIEW_PIXELS, y + CAMERA_VIEW_PIXELS))

    # Resize to 36x36
    resized = subset.resize((CAMERA_RESOLUTION, CAMERA_RESOLUTION), Image.LANCZOS)

    # # Rotate
    # angle = self.rotation_angle.get()
    # rotated = resized.rotate(angle, expand=True)

    # # Save the result
    # save_path = filedialog.asksaveasfilename(
    #     defaultextension=".jpg",
    #     filetypes=[("JPEG files", "*.jpg"), ("PNG files", "*.png"), ("All files", "*.*")]
    # )

    # if save_path:
    #     rotated.save(save_path)
    #     self.status.set(f"Processed image saved to {save_path}")

    #     # Show the result
    # self.show_result(rotated)

    # display = rotated.resize((400,400), Image.LANCZOS)
    # self.show_result(display)
    # solve(rotated)
    return resized


im = Image.new("RGB", (WIDTH, HEIGHT), color="white")
draw = ImageDraw.Draw(im)

# Instantiate the codec
north = numpy.array([0, 0])
south = numpy.array([1, 1])
west = numpy.array([1, 0])
east = numpy.array([0, 1])

g = codec().encode_bitmatrix(shape=(3375, 3375), section=(0, 0))
draw_y = MARGIN + 1
for y in range(3375):
    draw_x = MARGIN + 1
    for x in range(3375):
        val = g[y, x]
        if numpy.array_equal(north, val):
            draw.point((draw_x, draw_y - 1), fill="black")
        elif numpy.array_equal(south, val):
            draw.point((draw_x, draw_y + 1), fill="black")
        elif numpy.array_equal(east, val):
            draw.point((draw_x + 1, draw_y), fill="black")
        elif numpy.array_equal(west, val):
            draw.point((draw_x - 1, draw_y), fill="black")

        # draw
        # draw.point((draw_x, draw_y), fill="red")
        draw_x += SPACING

        if draw_x > WIDTH - MARGIN:
            break

    draw_y += SPACING

    if draw_y > HEIGHT - MARGIN:
        break

# im.save(f"anoto_4x4_{SPACING}spacing_{DPI}dpi.png")
# im.show()

test_cases = list()
for x in range(10):
    pix_x = MARGIN + x * SPACING
    for y in range(10):
        pix_y = MARGIN + y * SPACING
        img = process_selection(im, pix_x, pix_y)
        # test_cases.append(TestCase(img, x, y))
        buffered = BytesIO()
        img.save(buffered, format="PNG")
        img_bytes = buffered.getvalue()

        test_case = dict()
        test_case["image"] = base64.b64encode(img_bytes).decode("utf-8")
        test_case["x"] = x
        test_case["y"] = y
        test_cases.append(test_case)

with open("testcases.json", "w") as outfi:
    json.dump(test_cases, outfi)

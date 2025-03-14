#!/usr/bin/env python3

import microdots as mdots
import numpy
from microdots.mini_sequences import A1, A2, MNS
from PIL import Image, ImageDraw, ImageFont

# WIDTH = 3295
# HEIGHT = 2539
WIDTH = 7000
HEIGHT = 4960
# MARGIN = 120
MARGIN = 240
# BOX_LEN = 355
BOX_LEN = 355 * 2
X_BOXES = 8
Y_BOXES = 6
SPACING = 6
DPI = 600


def codec():
    codec4x4 = mdots.AnotoCodec(
        mns=MNS,
        mns_order=4,
        sns=[A1, A2],
        pfactors=(3, 5),
        delta_range=(1, 15),
    )

    # g = codec4x4.encode_bitmatrix(shape=(4375, 3375), section=(0, 0))
    # # print(g)

    # # Render dots
    # fig, ax = plt.subplots()
    # mdots.draw_dots(g, grid_size=1.0, show_grid=True, ax=ax)
    # fig.savefig("dots.pdf")
    # plt.close(fig)

    return codec4x4


im = Image.new("RGB", (WIDTH, HEIGHT), color="white")
draw = ImageDraw.Draw(im)

font_path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
font = ImageFont.truetype(font_path, 40)

draw.text((MARGIN + 10, MARGIN + 10), f"Spacing: {SPACING}", fill="black", font=font)

draw.text((MARGIN + 10, MARGIN + 60), f"DPI: {DPI}", fill="black", font=font)

# draw grid

# draw horizontal lines
cur_y = MARGIN
for y in range(Y_BOXES + 1):
    draw.line(((MARGIN, cur_y), (WIDTH - MARGIN, cur_y)), width=3, fill="black")
    cur_y += BOX_LEN

# draw vertical lines
cur_x = MARGIN
for x in range(X_BOXES + 1):
    draw.line(((cur_x, MARGIN), (cur_x, HEIGHT - MARGIN)), width=3, fill="black")
    cur_x += BOX_LEN

# label box with coords
cur_y = MARGIN
for y in range(Y_BOXES):
    cur_x = MARGIN
    for x in range(X_BOXES):
        if not (x == 0 and y == 0):
            draw.text((cur_x + 10, cur_y + 10), f"{x},{y}", fill="black", font=font)
        cur_x += BOX_LEN

    cur_y += BOX_LEN


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
im.save(f"anoto_4x4_{SPACING}spacing_{DPI}dpi.pdf", "PDF", resolution=DPI)

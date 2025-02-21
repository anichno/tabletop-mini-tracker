#!/usr/bin/env python3

from PIL import Image, ImageDraw, ImageFont

WIDTH = 2539
HEIGHT = 3295
MARGIN = 120
BOX_LEN = 355
X_BOXES = 6
Y_BOXES = 8


def fill_box(x, y, spacing):
    draw.text((x+10, y+10), str(spacing), fill="black", font=font)

    cur_y = y+20
    max_y = y+BOX_LEN-20
    while cur_y <= max_y:
        cur_x = x+20
        max_x = x+BOX_LEN-20

        while cur_x <= max_x:
            draw.point((cur_x, cur_y), fill="black")
            cur_x += spacing

        cur_y += spacing


im = Image.new('RGB', (WIDTH, HEIGHT), color="white")
# pixels = im.load()
# pixels[512,512] = (0,0,0)

draw = ImageDraw.Draw(im)

font_path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
font = ImageFont.truetype(font_path, 40)

# draw grid

# draw horizontal lines
cur_y = MARGIN
for y in range(Y_BOXES+1):
    draw.line(((MARGIN, cur_y), (WIDTH-MARGIN, cur_y)), width=3, fill="black")
    cur_y += BOX_LEN

# draw vertical lines
cur_x = MARGIN
for x in range(X_BOXES+1):
    draw.line(((cur_x, MARGIN), (cur_x, HEIGHT-MARGIN)), width=3, fill="black")
    cur_x += BOX_LEN

# fill each box
box_num = 2
cur_y = MARGIN
for y in range(Y_BOXES):
    cur_x = MARGIN
    for x in range(X_BOXES):
        fill_box(cur_x, cur_y, box_num)
        box_num += 1
        cur_x += BOX_LEN

    cur_y += BOX_LEN

# im.show()
im.save("test.png")

# import sys
# sys.path.insert(0, "./py-microdots")
# import microdots as mdots
# from microdots.mini_sequences import MNS, A1, A2
# # Instantiate the codec
# codec4x4 = mdots.AnotoCodec(
# mns=MNS,
# mns_order=4,
# sns=[A1, A2],
# pfactors=(3, 5),
# delta_range=(1, 15),
# )

# g = codec4x4.encode_bitmatrix(shape=(9,16), section=(10,2))

# import matplotlib.pyplot as plt
# # Render dots
# fig, ax = plt.subplots()
# mdots.draw_dots(g, grid_size=1.0, show_grid=True, ax=ax)
# fig.savefig("dots.pdf")
# plt.close(fig)

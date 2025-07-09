#!/usr/bin/env python3

import tkinter as tk
from tkinter import filedialog
from PIL import Image, ImageTk
import numpy as np
from dataclasses import dataclass
import math

CAMERA_VIEW_PIXELS = 14
CAMERA_RESOLUTION = 36

@dataclass
class Blob:
    min_x: int
    max_x: int
    min_y: int
    max_y: int

    def center(self):
        cx = (self.min_x + self.max_x) / 2
        cy = (self.min_y + self.max_y) / 2

        return (cx, cy)


def solve(image):
    # first get image into 2d array, since that is what we will actually process
    img_array = np.array(image.convert('L'))
    print(img_array.shape)
    np.set_printoptions(linewidth=np.inf, threshold=np.inf)
    print(img_array)

    thresholded = list()
    for row in img_array:
        new_row = list()
        for col in row:
            if col < 235:
                new_row.append(1)
            else:
                new_row.append(0)

        thresholded.append(new_row)

    for row in thresholded:
        print(row)

    # find blobs. Scan through rows looking for val set to 1, then find all connected 1 vals, recording them and setting them to 0. For each group record center and radius
    blob_scores = list()#[[0 for x in range(len(thresholded))] for y in range(len(thresholded))]
    # groups = dict()
    # for y,row in enumerate(thresholded):
    #     for x,col in enumerate(row):
    #         if col == 1:
    #             # look at all adjacent pixels, for known group id, if all 0 or 1, create new group and make pixel that id. If adjacent is in a group, add it to that group and set it to that id
    #             for ydiff in (-1,0,1):
    #                 for xdiff in (-1,0,1):
    #                     if xdiff == 0 and ydiff == 0:
    #                         continue
    #                     pix = thresholded[y+ydiff][x+xdiff]
    #                     if pix > 1:
    #                         thresholded[y][x] = pix
    #                         groups[pix].min_x = min(groups[pix].min_x, x)
    #                         groups[pix].max_x = 

    # scan 4x4 boxes, adding up all "1"s found and using that as the blob score. If score less than 3x3 throw away blob candidate
    for y in range(len(thresholded)):
        for x in range(len(thresholded)):
            score = 0
            for yscan in range(4):
                if y+yscan >= len(thresholded):
                    continue
                for xscan in range(4):
                    if x+xscan >= len(thresholded):
                        continue
                    score += thresholded[y+yscan][x+xscan]
            if score >= 3*3:
                blob_scores.append((x, y, score))

    # for each blob candidate, pick the best of the 2 within distance 3 of one another
    for i in range(len(blob_scores)):
        blob_1 = blob_scores[i]
        if blob_1[2] == 0:
            continue
        for j in range(i+1, len(blob_scores)):
            blob_2 = blob_scores[j]
            if blob_2[2] == 0:
                continue
            distance = math.sqrt((blob_1[0] - blob_2[0])**2 + (blob_1[1] - blob_2[1])**2)
            if distance < 2:
                if blob_1[2] >= blob_2[2]:
                    blob_scores[j] = (0,0,0)
                else:
                    blob_scores[i] = (0,0,0)

    # to simplify, make list of blobs we kept
    blobs = list()
    for blob in blob_scores:
        if blob[2] > 0:
            blobs.append(blob)

    print()
    for y in range(len(thresholded)):
        for x in range(len(thresholded)):
            known_blob = False
            for blob in blobs:
                # if blob[2] == 0:
                #     continue
                if blob[0] == x and blob[1] == y:
                    print(f"{blob[2]:02}", end="")
                    known_blob = True
            
            if not known_blob:
                # print("0"+str(thresholded[y][x]), end="")
                print("  ", end="")

            print(", ", end="")
        print()
                        



# def find_blobs(img):


class ImageCropperApp:
    def __init__(self, root):
        self.root = root
        self.root.title("Image Cropper and Rotator")
        
        # Variables
        self.image_path = None
        self.original_image = None
        self.display_image = None
        self.photo_image = None
        self.start_x = None
        self.start_y = None
        self.crop_box = None
        self.rect_id = None
        self.rotation_angle = tk.IntVar(value=0)
        
        # Main frame
        main_frame = tk.Frame(root)
        main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        # Controls frame
        controls_frame = tk.Frame(main_frame)
        controls_frame.pack(fill=tk.X, side=tk.TOP, pady=5)
        
        # Load image button
        load_btn = tk.Button(controls_frame, text="Load Image", command=self.load_image)
        load_btn.pack(side=tk.LEFT, padx=5)
        
        # Rotation control
        tk.Label(controls_frame, text="Rotation angle:").pack(side=tk.LEFT, padx=5)
        rotation_scale = tk.Scale(controls_frame, variable=self.rotation_angle, from_=0, to=360, 
                                 orient=tk.HORIZONTAL, length=200)
        rotation_scale.pack(side=tk.LEFT, padx=5)
        
        # # Process button
        # process_btn = tk.Button(controls_frame, text="Process Selected Area", command=self.process_selection)
        # process_btn.pack(side=tk.LEFT, padx=5)
        
        # Canvas for the image
        self.canvas_frame = tk.Frame(main_frame)
        self.canvas_frame.pack(fill=tk.BOTH, expand=True, pady=5)
        
        self.canvas = tk.Canvas(self.canvas_frame, bg="gray")
        self.canvas.pack(fill=tk.BOTH, expand=True)
        
        # Status bar
        self.status = tk.StringVar()
        self.status.set("Load an image to begin")
        status_bar = tk.Label(main_frame, textvariable=self.status, bd=1, relief=tk.SUNKEN, anchor=tk.W)
        status_bar.pack(side=tk.BOTTOM, fill=tk.X)
        
        # Bind mouse events
        # self.canvas.bind("<ButtonPress-1>", self.on_mouse_down)
        # self.canvas.bind("<B1-Motion>", self.on_mouse_drag)
        self.canvas.bind("<ButtonRelease-1>", self.on_mouse_up)
        
        # Result window
        self.result_window = None
        self.result_canvas = None

    def load_image(self):
        # Open file dialog
        file_path = filedialog.askopenfilename(
            filetypes=[("Image files", "*.jpg *.jpeg *.png *.bmp *.gif")])
        
        if not file_path:
            return
        
        self.image_path = file_path
        self.original_image = Image.open(file_path)
        
        # Adjust canvas size
        self.canvas.config(width=self.original_image.width, height=self.original_image.height)
        
        # Display the image
        self.display_image = self.original_image.copy()
        self.photo_image = ImageTk.PhotoImage(self.display_image)
        self.canvas.create_image(0, 0, anchor=tk.NW, image=self.photo_image)
        
        # Reset selection
        self.crop_box = None
        if self.rect_id:
            self.canvas.delete(self.rect_id)
            self.rect_id = None
        
        self.status.set(f"Loaded image: {file_path} - Click and drag to select area")

    # def on_mouse_down(self, event):
    #     if self.original_image is None:
    #         return
        
    #     self.start_x = event.x
    #     self.start_y = event.y
        
    #     if self.rect_id:
    #         self.canvas.delete(self.rect_id)
        
    #     self.rect_id = self.canvas.create_rectangle(
    #         self.start_x, self.start_y, self.start_x, self.start_y, 
    #         outline="red", width=2
    #     )

    # def on_mouse_drag(self, event):
    #     if self.rect_id is None:
    #         return
        
    #     self.canvas.coords(self.rect_id, self.start_x, self.start_y, event.x, event.y)
    #     self.status.set(f"Selection: ({self.start_x}, {self.start_y}) to ({event.x}, {event.y})")

    def on_mouse_up(self, event):
        # if self.rect_id is None:
        #     return
        
        # x1, y1 = self.start_x, self.start_y
        # x2, y2 = event.x, event.y
        x1 = event.x - CAMERA_VIEW_PIXELS/2
        y1 = event.y - CAMERA_VIEW_PIXELS/2
        x2 = event.x + CAMERA_VIEW_PIXELS/2
        y2 = event.y + CAMERA_VIEW_PIXELS/2
        
        # # Ensure x1,y1 is the top left and x2,y2 is the bottom right
        # if x1 > x2:
        #     x1, x2 = x2, x1
        # if y1 > y2:
        #     y1, y2 = y2, y1
        
        self.crop_box = (x1, y1, x2, y2)

        if self.rect_id:
            self.canvas.delete(self.rect_id)

        self.rect_id = self.canvas.create_rectangle(
            x1, y1, x2, y2, 
            outline="red", width=2
        )

        # self.status.set(f"Selected area: {self.crop_box}")
        self.process_selection()

    def process_selection(self):
        if self.original_image is None or self.crop_box is None:
            self.status.set("Please load an image and select an area first")
            return
        
        # Extract the subset
        subset = self.original_image.crop(self.crop_box)
        
        # Resize to 36x36
        resized = subset.resize((CAMERA_RESOLUTION, CAMERA_RESOLUTION), Image.LANCZOS)
        
        # Rotate
        angle = self.rotation_angle.get()
        rotated = resized.rotate(angle, expand=True)
        
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

        display = rotated.resize((400,400), Image.LANCZOS)
        self.show_result(display)
        solve(rotated)
        
    def show_result(self, image):
        # Create a new window for the result if it doesn't exist
        if not self.result_window or not tk.Toplevel.winfo_exists(self.result_window):
            self.result_window = tk.Toplevel(self.root)
            self.result_window.title("Processed Result")
            
            self.result_canvas = tk.Canvas(
                self.result_window, 
                width=max(image.width, 200), 
                height=max(image.height, 200),
                bg="gray"
            )
            self.result_canvas.pack(padx=10, pady=10)
        
        # Display the processed image
        self.result_photo = ImageTk.PhotoImage(image)
        # self.result_canvas.config(width=max(image.width, 200), height=max(image.height, 200))
        self.result_canvas.config(width=image.width, height=image.height)
        self.result_canvas.create_image(
            image.width//2, image.height//2, 
            image=self.result_photo
        )
        
        # # Add info label
        # info_text = f"Size: {image.width}x{image.height}, Rotation: {self.rotation_angle.get()}°"
        # tk.Label(self.result_window, text=info_text).pack(pady=5)

if __name__ == "__main__":
    root = tk.Tk()
    app = ImageCropperApp(root)
    root.mainloop()
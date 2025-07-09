#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "matplotlib",
#     "numpy",
#     "pyserial",
# ]
# ///

import json
import time
import tkinter as tk
from tkinter import simpledialog

import matplotlib.pyplot as plt
import numpy as np
import serial
from PIL import Image
from serial import Serial

SERIAL_PORT = "/dev/ttyACM0"


def display_image_stream():
    """
    Continuously read and display 36x36 images from serial device /dev/ttyACM0
    Each image is 1296 bytes followed by a newline
    """
    try:
        # Set up serial connection
        ser = Serial(SERIAL_PORT, 115200, timeout=1)
        print(f"Connected to {SERIAL_PORT}")

        # Set up the plot
        plt.ion()  # Enable interactive mode
        fig, ax = plt.subplots(figsize=(8, 8))
        img_display = ax.imshow(np.zeros((36, 36)), cmap="gray")
        ax.axis("off")
        plt.title("36x36 Image Visualization (Live)")

        root = tk.Tk()
        root.withdraw()  # Hide the main window

        buffer = bytearray()
        FRAME_MARKER = b"FRAME"
        CAPTURE_MARKER = b"CAPTURE"
        # SOLVE_MARKER = b'SOLVE'
        EXPECTED_SIZE = 1296 + 8 * 8

        frame_data = None

        found_frame_start = False

        while True:
            try:
                # Read data byte by byte
                if ser.in_waiting:
                    buffer.extend(ser.read(ser.in_waiting))

                    if found_frame_start:
                        if len(buffer) >= EXPECTED_SIZE:
                            frame_buffer = buffer[:EXPECTED_SIZE]
                            buffer = buffer[len(frame_buffer) :]
                            found_frame_start = False
                            frame_data = frame_buffer[:1296]
                            solve_data = frame_buffer[1296 : 1296 + 8 * 8]

                            pixels = np.frombuffer(frame_data, dtype=np.uint8)

                            # Reshape into 36x36 grid
                            image = pixels.reshape(36, 36)

                            np.set_printoptions(linewidth=np.inf, threshold=np.inf)
                            print()
                            print(image)
                            print()

                            # Update the display
                            img_display.set_data(image)
                            img_display.set_clim(vmin=np.min(image), vmax=np.max(image))

                            # Redraw the figure
                            fig.canvas.draw_idle()
                            fig.canvas.flush_events()

                    else:
                        # Look for FRAME marker
                        marker_pos = buffer.find(FRAME_MARKER)

                        if marker_pos >= 0:
                            # Clear processed data and marker from buffer
                            buffer = buffer[marker_pos + len(FRAME_MARKER) :]
                            found_frame_start = True
                        elif buffer.find(CAPTURE_MARKER) >= 0:
                            buffer = buffer[marker_pos + len(FRAME_MARKER) :]
                            user_input = simpledialog.askstring(
                                "Input", "Enter some text:"
                            )
                            print(user_input)
                            if user_input is not None and pixels is not None:
                                timestamp = int(time.time())
                                # samples.append((user_input, list(frame_data)))
                                with open(
                                    f"samples/{user_input}_{timestamp}.json", "w"
                                ) as outfi:
                                    json.dump(list(frame_data), outfi)

                                pixels = np.frombuffer(frame_data, dtype=np.uint8)

                                # Reshape into 36x36 grid
                                image = pixels.reshape(36, 36)
                                img = Image.fromarray(
                                    image, mode="L"
                                )  # 'L' mode is for greyscale
                                img.save(f"samples/{user_input}_{timestamp}.png")

                    # Prevent buffer from growing too large
                    if len(buffer) > EXPECTED_SIZE * 4:
                        print("Buffer overflow, clearing...")
                        buffer.clear()

                time.sleep(0.01)  # Small delay to prevent CPU overload

            except KeyboardInterrupt:
                print("\nStopping image capture...")
                break
            except Exception as e:
                print(f"Error processing frame: {e}")
                buffer.clear()
                continue

    except serial.SerialException as e:
        print(f"Error opening serial port: {e}")
    finally:
        # Clean up
        if "ser" in locals():
            ser.close()
        plt.ioff()
        plt.close("all")


if __name__ == "__main__":
    display_image_stream()

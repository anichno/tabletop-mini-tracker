#!/usr/bin/env python3

import numpy as np
import matplotlib.pyplot as plt
import serial
from serial import Serial
import time

def display_image_stream():
    """
    Continuously read and display 36x36 images from serial device /dev/ttyACM0
    Each image is 1296 bytes followed by a newline
    """
    try:
        # Set up serial connection
        ser = Serial('/dev/ttyACM0', 115200, timeout=1)
        print("Connected to /dev/ttyACM0")
        
        # Set up the plot
        plt.ion()  # Enable interactive mode
        fig, ax = plt.subplots(figsize=(8, 8))
        img_display = ax.imshow(np.zeros((36, 36)), cmap='gray')
        ax.axis('off')
        plt.title('36x36 Image Visualization (Live)')
        
        buffer = bytearray()
        FRAME_MARKER = b'FRAME'
        EXPECTED_SIZE = 1296

        while True:
            try:
                # Read data byte by byte
                if ser.in_waiting:
                    buffer.extend(ser.read(ser.in_waiting))
                    
                    # Look for FRAME marker
                    marker_pos = buffer.find(FRAME_MARKER)
                    
                    if marker_pos >= 0:
                        # Extract the frame data before the marker
                        frame_data = buffer[:marker_pos]
                        
                        # Clear processed data and marker from buffer
                        buffer = buffer[marker_pos + len(FRAME_MARKER):]
                        
                        # Check if we have the correct number of bytes
                        if len(frame_data) == EXPECTED_SIZE:
                            # Convert bytes to numpy array
                            pixels = np.frombuffer(frame_data, dtype=np.uint8)
                            
                            # Reshape into 36x36 grid
                            image = pixels.reshape(36, 36)
                            
                            # Update the display
                            img_display.set_data(image)
                            img_display.set_clim(vmin=np.min(image), vmax=np.max(image))
                            
                            # Redraw the figure
                            fig.canvas.draw_idle()
                            fig.canvas.flush_events()
                        else:
                            print(f"Invalid frame size: {len(frame_data)} bytes (expected {EXPECTED_SIZE})")
                            
                    # Prevent buffer from growing too large
                    if len(buffer) > EXPECTED_SIZE * 2:
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
        if 'ser' in locals():
            ser.close()
        plt.ioff()
        plt.close('all')

if __name__ == "__main__":
    display_image_stream()
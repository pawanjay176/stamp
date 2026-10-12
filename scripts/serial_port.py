"""Resolve a serial port path; never open, reset, or write to a device."""

import glob
import os
import sys


def main():
    argument = sys.argv[1] if len(sys.argv) > 1 else ""
    ports = sorted(
        {path for pattern in ("/dev/cu.usbmodem*", "/dev/ttyACM*", "/dev/ttyUSB*")
         for path in glob.glob(pattern)}
    )
    if argument == "--list":
        print("\n".join(ports) if ports else "No candidate USB serial ports found.")
        return
    if argument:
        if not os.path.exists(argument):
            sys.exit(f"Serial port does not exist: {argument}")
        print(argument)
        return
    if len(ports) != 1:
        detail = ", ".join(ports) if ports else "none"
        sys.exit(
            f"Expected exactly one USB serial port; found {detail}. "
            "Connect the X4 or pass PORT=/dev/... explicitly."
        )
    print(ports[0])


if __name__ == "__main__":
    main()
